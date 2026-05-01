//! Spatial outliner canvas pour Forgexalith — Phase 22 / 22b / 22c.
//!
//! **Phase 22** : canvas DOM-CSS infini — cartes notes, pan/zoom/drag.
//! **Phase 22b** : couche de dessin minimaliste — Pen, Line, Rect, Circle,
//!   palette de couleurs, undo, clear — rendue via overlay SVG co-transformé.
//! **Phase 22c** : Arrow tool, palette custom (color picker), persistance des
//!   dessins (save/load JSON via IPC), filtre de notes par tags.
//!
//! # Modes
//! - **Navigation** (défaut) : pan fond, drag cartes, clic → ouvre note.
//! - **Dessin** : clic-glisser crée des éléments graphiques SVG. Les cartes
//!   restent visibles mais leur drag est désactivé.
//!
//! # Architecture
//! Tous les éléments de dessin sont en coordonnées *canvas* (espace logique).
//! L'overlay `<svg>` est placé à l'intérieur du viewport-div qui reçoit le
//! `transform: translate/scale`, donc pan et zoom s'appliquent naturellement.

use crate::app::AppState;
use crate::tab_manager::TabManager;
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

// ── Types locaux ──────────────────────────────────────────────────────────────

/// Carte de note positionnée sur le canvas (session uniquement, non persistée).
#[derive(Clone, Debug)]
struct LocalCanvasItem {
    /// Chemin relatif dans le vault — sert d'identifiant unique.
    id: String,
    /// Position X en pixels canvas (espace logique).
    x: f64,
    /// Position Y en pixels canvas (espace logique).
    y: f64,
    /// Titre affiché (basename sans `.md`).
    title: String,
}

/// Outil de dessin actif.
#[derive(Clone, Debug, PartialEq)]
enum DrawTool {
    Hand,
    Pen,
    Eraser,
    Fill,
    Line,
    Arrow,
    Rect,
    Circle,
}

/// Item currently selected or hovered by the Hand tool.
#[derive(Clone, Debug, PartialEq)]
enum SelectedItem {
    /// Index into `draw_elements`.
    Drawing(usize),
    /// Card id (path).
    Card(String),
}

/// Undoable action recorded in the undo stack.
#[derive(Clone, Debug)]
enum UndoAction {
    /// A drawing element was added at the given index.
    AddDrawing(usize),
    /// One or more items were moved by (dx, dy) in canvas coords.
    MoveItems {
        /// Each entry: selected item + cumulative delta applied.
        items: Vec<(SelectedItem, f64, f64)>,
    },
}

/// Plage min/max pour le slider de taille Pen/Eraser (en pixels canvas).
const BRUSH_MIN: f64 = 0.5;
const BRUSH_MAX: f64 = 500.0;

/// Default stroke width.
fn default_stroke_width() -> f64 { 2.5 }

/// Default opacity for backward-compatible deserialization.
fn default_opacity() -> f64 { 1.0 }

/// Élément de dessin finalisé, stocké en coordonnées canvas.
///
/// Serde-enabled for persistence: canvas drawings are saved as JSON
/// alongside the vault in `.forgexalith/canvas-drawings.json`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum DrawEl {
    Path {
        pts: Vec<(f64, f64)>,
        color: String,
        #[serde(default = "default_stroke_width")]
        width: f64,
        #[serde(default = "default_opacity")]
        opacity: f64,
    },
    Line {
        x1: f64, y1: f64, x2: f64, y2: f64, color: String,
        #[serde(default = "default_opacity")]
        opacity: f64,
    },
    Arrow {
        x1: f64, y1: f64, x2: f64, y2: f64, color: String,
        #[serde(default = "default_opacity")]
        opacity: f64,
    },
    Rect {
        x: f64, y: f64, w: f64, h: f64, color: String,
        #[serde(default = "default_opacity")]
        opacity: f64,
    },
    Circle {
        cx: f64, cy: f64, r: f64, color: String,
        #[serde(default = "default_opacity")]
        opacity: f64,
    },
    /// Filled rectangle placed by the Fill tool (zone coloring).
    /// Rendered BEFORE strokes (z-order: behind everything else).
    FillRect {
        x: f64, y: f64, w: f64, h: f64, color: String,
        #[serde(default = "default_opacity")]
        opacity: f64,
    },
}

/// Convertit un [`DrawEl`] en vue SVG (`AnyView`).
///
/// # Attributs SVG — attention au `view!` de Leptos 0.7
///
/// Les attributs géométriques (`points`, `x1/y1/x2/y2`, `x/y/width/height`,
/// `cx/cy/r`) doivent être écrits en **attributs nus**, SANS le préfixe
/// `attr:`. En effet, le compilateur du `view!` de Leptos 0.7 ne strippe PAS
/// le préfixe `attr:` dans ce contexte (fonction libre retournant `AnyView`) :
/// le DOM se retrouve alors avec des attributs littéraux `attr:points="..."`,
/// `attr:x1="..."`, etc. — noms inconnus du namespace SVG, donc ignorés par
/// le moteur de rendu. Résultat : styles corrects mais géométrie invisible.
///
/// Pattern confirmé fonctionnel dans `graph_view_svg_legacy.rs` : attributs
/// nus sur `<line>` et `<circle>`. Règle à appliquer ici.
///
/// Toutes les propriétés de peinture (`fill`, `stroke`, `stroke-width`,
/// `stroke-linecap`, `stroke-linejoin`) passent via `style` pour éviter les
/// problèmes de noms d'attributs SVG avec tirets (non acceptés tels quels
/// par le parser Rust du `view!`).
///
/// # Dépendance au viewport parent
///
/// Cette fonction retourne juste `<polyline>` / `<line>` / `<rect>` /
/// `<circle>` nus. Le viewport SVG (taille, `viewBox`, `overflow="visible"`)
/// doit être défini par le parent `<svg>` appelant — voir la déclaration de
/// l'overlay SVG plus bas dans ce fichier. Si le viewport est trop petit
/// (ex: `width:1px;height:1px`), les éléments seront clippés avant peinture
/// malgré un DOM correct.
fn draw_el_view(el: &DrawEl) -> AnyView {
    const SW: &str = "stroke-linecap:round;stroke-linejoin:round;";

    match el {
        DrawEl::Path { pts, color, width, opacity } => {
            let pts_str = pts
                .iter()
                .map(|(x, y)| format!("{x:.1},{y:.1}"))
                .collect::<Vec<_>>()
                .join(" ");
            let w = if *width > 0.0 { *width } else { 2.5 };
            let style = format!("fill:none;stroke:{color};stroke-width:{w};opacity:{opacity};{SW}");
            view! { <polyline points={pts_str} style={style} /> }.into_any()
        }
        DrawEl::Line { x1, y1, x2, y2, color, opacity } => {
            let style = format!("stroke:{color};opacity:{opacity};{SW}");
            view! {
                <line
                    x1={x1.to_string()} y1={y1.to_string()}
                    x2={x2.to_string()} y2={y2.to_string()}
                    style={style}
                />
            }
            .into_any()
        }
        DrawEl::Arrow { x1, y1, x2, y2, color, opacity } => {
            // Shaft + arrowhead as a single <path>.
            // Arrowhead: two wings at +/-30 deg from tip, 15 canvas-px long.
            let angle = (y2 - y1).atan2(x2 - x1);
            let head_len = 15.0_f64;
            let ha = 0.5_f64; // ~28.6 deg half-angle
            let hx1 = x2 - head_len * (angle - ha).cos();
            let hy1 = y2 - head_len * (angle - ha).sin();
            let hx2 = x2 - head_len * (angle + ha).cos();
            let hy2 = y2 - head_len * (angle + ha).sin();
            let d = format!(
                "M {:.1},{:.1} L {:.1},{:.1} M {:.1},{:.1} L {:.1},{:.1} L {:.1},{:.1}",
                x1, y1, x2, y2, hx1, hy1, x2, y2, hx2, hy2
            );
            let style = format!("fill:none;stroke:{color};opacity:{opacity};{SW}");
            view! { <path d={d} style={style} /> }.into_any()
        }
        DrawEl::Rect { x, y, w, h, color, opacity } => {
            let style = format!("fill:none;stroke:{color};stroke-width:2.5;opacity:{opacity};");
            view! {
                <rect
                    x={x.to_string()} y={y.to_string()}
                    width={w.to_string()} height={h.to_string()}
                    style={style}
                />
            }
            .into_any()
        }
        DrawEl::Circle { cx, cy, r, color, opacity } => {
            let style = format!("fill:none;stroke:{color};stroke-width:2.5;opacity:{opacity};");
            view! {
                <circle
                    cx={cx.to_string()} cy={cy.to_string()}
                    r={r.to_string()}
                    style={style}
                />
            }
            .into_any()
        }
        DrawEl::FillRect { x, y, w, h, color, opacity } => {
            let style = format!("fill:{color};stroke:none;opacity:{opacity};");
            view! {
                <rect
                    x={x.to_string()} y={y.to_string()}
                    width={w.to_string()} height={h.to_string()}
                    style={style}
                />
            }
            .into_any()
        }
    }
}

/// Palette de couleurs prédéfinies (label, hex).
static PALETTE: &[(&str, &str)] = &[
    ("Rose",    "#e11d48"),
    ("Sky",     "#0ea5e9"),
    ("Emerald", "#10b981"),
    ("Amber",   "#f59e0b"),
    ("Violet",  "#8b5cf6"),
    ("Blanc",   "#f1f5f9"),
];

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Convertit les coordonnées écran en coordonnées canvas.
#[inline]
fn to_canvas(mx: f64, my: f64, pan_x: f64, pan_y: f64, zoom: f64) -> (f64, f64) {
    ((mx - pan_x) / zoom, (my - pan_y) / zoom)
}

/// Construit le `DrawEl` preview/finalisé pour Line, Arrow, Rect, Circle, Fill.
fn make_shape(tool: &DrawTool, sx: f64, sy: f64, ex: f64, ey: f64, color: String, opacity: f64) -> DrawEl {
    match tool {
        DrawTool::Line => DrawEl::Line { x1: sx, y1: sy, x2: ex, y2: ey, color, opacity },
        DrawTool::Arrow => DrawEl::Arrow { x1: sx, y1: sy, x2: ex, y2: ey, color, opacity },
        DrawTool::Rect => DrawEl::Rect {
            x: sx.min(ex),
            y: sy.min(ey),
            w: (ex - sx).abs(),
            h: (ey - sy).abs(),
            color, opacity,
        },
        DrawTool::Circle => {
            let r = ((ex - sx).powi(2) + (ey - sy).powi(2)).sqrt();
            DrawEl::Circle { cx: sx, cy: sy, r, color, opacity }
        }
        DrawTool::Fill => {
            DrawEl::FillRect {
                x: sx.min(ex),
                y: sy.min(ey),
                w: (ex - sx).abs(),
                h: (ey - sy).abs(),
                color, opacity,
            }
        }
        DrawTool::Pen | DrawTool::Eraser | DrawTool::Hand => unreachable!("make_shape ne gère pas Pen/Eraser/Hand"),
    }
}

/// Renvoie vrai si l'élément a une dimension non-nulle (évite les points isolés).
fn has_extent(el: &DrawEl) -> bool {
    match el {
        DrawEl::Path { pts, .. }                 => pts.len() > 2,
        DrawEl::Line { x1, y1, x2, y2, .. }     => (x2 - x1).hypot(y2 - y1) > 2.0,
        DrawEl::Arrow { x1, y1, x2, y2, .. }    => (x2 - x1).hypot(y2 - y1) > 2.0,
        DrawEl::Rect { w, h, .. }                => *w > 2.0 && *h > 2.0,
        DrawEl::Circle { r, .. }                 => *r > 2.0,
        DrawEl::FillRect { w, h, .. }             => *w > 2.0 && *h > 2.0,
    }
}

/// Compute the axis-aligned bounding box of a DrawEl as (x, y, w, h).
fn bounding_box(el: &DrawEl) -> (f64, f64, f64, f64) {
    match el {
        DrawEl::Path { pts, width, .. } => {
            if pts.is_empty() {
                return (0.0, 0.0, 0.0, 0.0);
            }
            let pad = width / 2.0;
            let min_x = pts.iter().map(|(x, _)| *x).fold(f64::INFINITY, f64::min) - pad;
            let min_y = pts.iter().map(|(_, y)| *y).fold(f64::INFINITY, f64::min) - pad;
            let max_x = pts.iter().map(|(x, _)| *x).fold(f64::NEG_INFINITY, f64::max) + pad;
            let max_y = pts.iter().map(|(_, y)| *y).fold(f64::NEG_INFINITY, f64::max) + pad;
            (min_x, min_y, max_x - min_x, max_y - min_y)
        }
        DrawEl::Line { x1, y1, x2, y2, .. } | DrawEl::Arrow { x1, y1, x2, y2, .. } => {
            let min_x = x1.min(*x2);
            let min_y = y1.min(*y2);
            let max_x = x1.max(*x2);
            let max_y = y1.max(*y2);
            (min_x, min_y, max_x - min_x, max_y - min_y)
        }
        DrawEl::Rect { x, y, w, h, .. } | DrawEl::FillRect { x, y, w, h, .. } => {
            (*x, *y, *w, *h)
        }
        DrawEl::Circle { cx, cy, r, .. } => {
            (cx - r, cy - r, r * 2.0, r * 2.0)
        }
    }
}

/// Translate all coordinates of a DrawEl by (dx, dy).
fn translate_draw_el(el: &mut DrawEl, dx: f64, dy: f64) {
    match el {
        DrawEl::Path { pts, .. } => {
            for p in pts.iter_mut() {
                p.0 += dx;
                p.1 += dy;
            }
        }
        DrawEl::Line { x1, y1, x2, y2, .. } | DrawEl::Arrow { x1, y1, x2, y2, .. } => {
            *x1 += dx; *y1 += dy;
            *x2 += dx; *y2 += dy;
        }
        DrawEl::Rect { x, y, .. } | DrawEl::FillRect { x, y, .. } => {
            *x += dx; *y += dy;
        }
        DrawEl::Circle { cx, cy, .. } => {
            *cx += dx; *cy += dy;
        }
    }
}

/// Hit-test: check if point (px, py) is within the bounding box of el, with tolerance.
fn point_in_bbox(px: f64, py: f64, el: &DrawEl, tol: f64) -> bool {
    let (bx, by, bw, bh) = bounding_box(el);
    px >= bx - tol && px <= bx + bw + tol && py >= by - tol && py <= by + bh + tol
}

/// Hit-test a point against a card (given its position and standard size 180x~50).
fn point_in_card(px: f64, py: f64, card_x: f64, card_y: f64, tol: f64) -> bool {
    // Cards are 180px wide, ~50px tall (approximate).
    let cw = 180.0;
    let ch = 50.0;
    px >= card_x - tol && px <= card_x + cw + tol && py >= card_y - tol && py <= card_y + ch + tol
}

/// Background color for the eraser tool.
const CANVAS_BG: &str = "#0f172a";

/// Generate a standalone SVG string from the drawing elements (for export).
fn build_export_svg(els: &[DrawEl]) -> String {
    const SW: &str = "stroke-linecap:round;stroke-linejoin:round;";
    let mut body = String::new();
    for el in els {
        match el {
            DrawEl::Path { pts, color, width, opacity } => {
                let pts_str: String = pts.iter()
                    .map(|(x, y)| format!("{x:.1},{y:.1}"))
                    .collect::<Vec<_>>()
                    .join(" ");
                let w = if *width > 0.0 { *width } else { 2.5 };
                body.push_str(&format!(
                    r#"<polyline points="{pts_str}" style="fill:none;stroke:{color};stroke-width:{w};opacity:{opacity};{SW}"/>"#
                ));
                body.push('\n');
            }
            DrawEl::Line { x1, y1, x2, y2, color, opacity } => {
                body.push_str(&format!(
                    r#"<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" style="stroke:{color};opacity:{opacity};{SW}"/>"#
                ));
                body.push('\n');
            }
            DrawEl::Arrow { x1, y1, x2, y2, color, opacity } => {
                let angle = (y2 - y1).atan2(x2 - x1);
                let hl = 15.0_f64;
                let ha = 0.5_f64;
                let hx1 = x2 - hl * (angle - ha).cos();
                let hy1 = y2 - hl * (angle - ha).sin();
                let hx2 = x2 - hl * (angle + ha).cos();
                let hy2 = y2 - hl * (angle + ha).sin();
                let d = format!(
                    "M {x1:.1},{y1:.1} L {x2:.1},{y2:.1} M {hx1:.1},{hy1:.1} L {x2:.1},{y2:.1} L {hx2:.1},{hy2:.1}"
                );
                body.push_str(&format!(
                    r#"<path d="{d}" style="fill:none;stroke:{color};opacity:{opacity};{SW}"/>"#
                ));
                body.push('\n');
            }
            DrawEl::Rect { x, y, w, h, color, opacity } => {
                body.push_str(&format!(
                    r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" style="fill:none;stroke:{color};stroke-width:2.5;opacity:{opacity};"/>"#
                ));
                body.push('\n');
            }
            DrawEl::Circle { cx, cy, r, color, opacity } => {
                body.push_str(&format!(
                    r#"<circle cx="{cx}" cy="{cy}" r="{r}" style="fill:none;stroke:{color};stroke-width:2.5;opacity:{opacity};"/>"#
                ));
                body.push('\n');
            }
            DrawEl::FillRect { x, y, w, h, color, opacity } => {
                body.push_str(&format!(
                    r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" style="fill:{color};stroke:none;opacity:{opacity};"/>"#
                ));
                body.push('\n');
            }
        }
    }
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="20000" height="20000" viewBox="0 0 20000 20000" style="background:{CANVAS_BG}">
{body}</svg>
"#
    )
}

// ── Composant principal ───────────────────────────────────────────────────────

/// Canvas spatial avec couche de dessin minimaliste.
///
/// Requiert [`AppState`] et [`TabManager`] fournis comme contextes Leptos par `App`.
#[component]
pub fn CanvasView() -> impl IntoView {
    // ── Contextes ─────────────────────────────────────────────────────────────

    let state   = use_context::<AppState>().expect("AppState context manquant");
    let tab_mgr = use_context::<TabManager>().expect("TabManager context manquant");

    // ── Signaux navigation ────────────────────────────────────────────────────

    let items:          RwSignal<Vec<LocalCanvasItem>>     = RwSignal::new(vec![]);
    let pan_x:          RwSignal<f64>                      = RwSignal::new(0.0);
    let pan_y:          RwSignal<f64>                      = RwSignal::new(0.0);
    let zoom:           RwSignal<f64>                      = RwSignal::new(1.0);
    let dragging_card:  RwSignal<Option<String>>           = RwSignal::new(None);
    let drag_start:     RwSignal<(f64, f64)>               = RwSignal::new((0.0, 0.0));
    let drag_offset:    RwSignal<(f64, f64)>               = RwSignal::new((0.0, 0.0));
    let pan_dragging:   RwSignal<bool>                     = RwSignal::new(false);
    let pan_start:      RwSignal<(f64, f64, f64, f64)>     = RwSignal::new((0.0, 0.0, 0.0, 0.0));

    // Offset du conteneur canvas par rapport au viewport navigateur (client coords).
    // Mis a jour a chaque mousedown sur le conteneur principal.
    // Sert a corriger client_x/client_y en coordonnees locales au canvas
    // (compensation sidebar, toolbar, scrollbar, etc.).
    let container_off:  RwSignal<(f64, f64)>               = RwSignal::new((0.0, 0.0));

    // ── Signaux dessin ────────────────────────────────────────────────────────
    // Doc en commentaires `//` (et non `///`) car rustdoc ne genere pas de doc
    // pour les statements `let` ; cf. warning `unused_doc_comments`.

    // `true` = mode Dessin ; `false` = mode Navigation.
    let draw_mode:      RwSignal<bool>                     = RwSignal::new(false);
    let active_tool:    RwSignal<DrawTool>                 = RwSignal::new(DrawTool::Pen);
    let active_color:   RwSignal<String>                   = RwSignal::new("#e11d48".to_string());
    let active_opacity: RwSignal<f64>                      = RwSignal::new(1.0);
    // Elements de dessin finalises.
    let draw_elements:  RwSignal<Vec<DrawEl>>              = RwSignal::new(vec![]);
    // Vrai pendant le clic-glisser en mode dessin.
    let is_drawing:     RwSignal<bool>                     = RwSignal::new(false);
    // Point de depart du trait/forme courant (coords canvas).
    let drawing_from:   RwSignal<Option<(f64, f64)>>       = RwSignal::new(None);
    // Points accumules pour l'outil Pen.
    let current_path:   RwSignal<Vec<(f64, f64)>>          = RwSignal::new(vec![]);
    // Apercu live de l'element en cours (mis a jour a chaque mousemove).
    let preview_el:     RwSignal<Option<DrawEl>>           = RwSignal::new(None);
    // `true` = afficher les cartes notes sur le canvas ; `false` = tableau blanc pur.
    let show_notes:     RwSignal<bool>                     = RwSignal::new(true);
    // Taille du trait pour Pen et Eraser (indépendante des autres outils).
    let pen_size:       RwSignal<f64>                      = RwSignal::new(2.5);
    let eraser_size:    RwSignal<f64>                      = RwSignal::new(10.0);
    // Popup de sélection de taille (affiché au clic droit sur Pen/Eraser).
    // Valeur = quel outil a déclenché le popup, None = fermé.
    let size_popup:     RwSignal<Option<DrawTool>>         = RwSignal::new(None);
    // Undo stack: records actions for Ctrl+Z / Shift+Z.
    let undo_stack:     RwSignal<Vec<UndoAction>>          = RwSignal::new(vec![]);

    // ── Signaux Hand tool (selection / drag) ─────────────────────────────
    // Currently selected items (drawing elements and/or cards).
    let selected_elements: RwSignal<Vec<SelectedItem>>     = RwSignal::new(vec![]);
    // Element under the cursor (for hover highlight).
    let hovered_element:   RwSignal<Option<SelectedItem>>  = RwSignal::new(None);
    // True while dragging selected items with the Hand tool.
    let hand_dragging:     RwSignal<bool>                  = RwSignal::new(false);
    // Canvas position where the Hand drag started.
    let hand_drag_start:   RwSignal<(f64, f64)>            = RwSignal::new((0.0, 0.0));
    // Last known canvas position during drag (for incremental delta).
    let hand_drag_last:    RwSignal<(f64, f64)>            = RwSignal::new((0.0, 0.0));
    // Accumulated delta during a Hand drag (for undo).
    let hand_drag_accum:   RwSignal<(f64, f64)>            = RwSignal::new((0.0, 0.0));

    // ── Signaux filtre par tags ──────────────────────────────────────────
    // Liste des tags disponibles dans le vault (chargee au mount).
    let available_tags: RwSignal<Vec<String>>               = RwSignal::new(vec![]);
    // Tag actif pour le filtre (None = afficher toutes les notes).
    let filter_tag:     RwSignal<Option<String>>            = RwSignal::new(None);
    // Notes correspondant au tag filtre (chemins relatifs).
    let filtered_notes: RwSignal<Option<Vec<String>>>       = RwSignal::new(None);

    // ── Effect: chargement des dessins persistés ──────────────────────────
    //
    // Au montage du composant (premier tick), charge les dessins depuis
    // `.forgexalith/canvas-drawings.json` via IPC. Si le fichier n'existe
    // pas (null), draw_elements reste vide.
    {
        let load_once = RwSignal::new(false);
        Effect::new(move |_| {
            if load_once.get_untracked() { return; }
            load_once.set(true);
            spawn_local(async move {
                match crate::ipc::load_canvas_drawings().await {
                    Ok(js_val) => {
                        if !js_val.is_null() && !js_val.is_undefined() {
                            if let Ok(els) = serde_wasm_bindgen::from_value::<Vec<DrawEl>>(js_val) {
                                draw_elements.set(els);
                            }
                        }
                    }
                    Err(e) => {
                        leptos::logging::warn!("canvas: load_canvas_drawings failed: {}", e);
                    }
                }
            });
        });
    }

    // ── Effect: auto-save dessins à chaque modification ──────────────────
    //
    // Sérialise draw_elements en JsValue et envoie au backend via IPC.
    // Réactif : se déclenche chaque fois que draw_elements change.
    Effect::new(move |_| {
        let els = draw_elements.get();
        spawn_local(async move {
            if let Ok(js_val) = serde_wasm_bindgen::to_value(&els) {
                if let Err(e) = crate::ipc::save_canvas_drawings(js_val).await {
                    leptos::logging::warn!("canvas: save_canvas_drawings failed: {}", e);
                }
            }
        });
    });

    // ── Effect: chargement des tags disponibles ────────────────────────────
    {
        let tags_loaded = RwSignal::new(false);
        Effect::new(move |_| {
            if tags_loaded.get_untracked() { return; }
            tags_loaded.set(true);
            spawn_local(async move {
                let tags = crate::ipc::list_vault_tags().await;
                if !tags.is_empty() {
                    available_tags.set(tags);
                }
            });
        });
    }

    // ── Effect: filtre notes par tag ─────────────────────────────────────
    //
    // Quand filter_tag change, charge la liste de notes matchant ce tag.
    // Si filter_tag est None, reset filtered_notes a None (= toutes).
    Effect::new(move |_| {
        let tag = filter_tag.get();
        match tag {
            None => { filtered_notes.set(None); }
            Some(t) => {
                let tag_clone = t.clone();
                spawn_local(async move {
                    let notes = crate::ipc::notes_by_tag(&tag_clone).await;
                    filtered_notes.set(Some(notes));
                });
            }
        }
    });

    // ── Effect: population des cartes ─────────────────────────────────────────

    Effect::new(move |_| {
        let notes = state.note_list.get();
        let new_items: Vec<LocalCanvasItem> = notes
            .iter()
            .enumerate()
            .map(|(i, path)| {
                let title = path
                    .rsplit(|c: char| c == '/' || c == '\\')
                    .next()
                    .unwrap_or(path.as_str())
                    .trim_end_matches(".md")
                    .to_string();
                let col = (i % 5) as f64;
                let row = (i / 5) as f64;
                LocalCanvasItem { id: path.clone(), x: 40.0 + col * 220.0, y: 40.0 + row * 160.0, title }
            })
            .collect();
        items.set(new_items);
    });

    // ── Closures globales mousemove / mouseup ─────────────────────────────────
    //
    // Installées une fois via `window.add_event_listener_with_callback` et
    // maintenues en vie par `.forget()`. Pattern identique à app.rs (resize sidebar).

    {
        let on_move =
            Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |ev: web_sys::MouseEvent| {
                // Guard: if the component was disposed (user left canvas view),
                // all RwSignals are dead. Early-return to avoid WASM panic.
                let Some((off_x, off_y)) = container_off.try_get_untracked() else { return; };
                let mx = ev.client_x() as f64;
                let my = ev.client_y() as f64;

                // ── Mode Dessin ───────────────────────────────────────────────
                if draw_mode.get_untracked() {
                    // -- Hand tool: drag or hover --------------------------
                    if active_tool.get_untracked() == DrawTool::Hand {
                        let z = zoom.get_untracked().max(0.001);
                        let (cx, cy) = to_canvas(mx - off_x, my - off_y, pan_x.get_untracked(), pan_y.get_untracked(), z);

                        if hand_dragging.get_untracked() {
                            // Compute incremental delta from last position.
                            let (lx, ly) = hand_drag_last.get_untracked();
                            let ddx = cx - lx;
                            let ddy = cy - ly;
                            hand_drag_last.set((cx, cy));
                            // Accumulate for undo.
                            let (ax, ay) = hand_drag_accum.get_untracked();
                            hand_drag_accum.set((ax + ddx, ay + ddy));

                            let sel = selected_elements.get_untracked();
                            // Move selected drawing elements.
                            draw_elements.update(|els| {
                                for item in &sel {
                                    if let SelectedItem::Drawing(idx) = item {
                                        if let Some(el) = els.get_mut(*idx) {
                                            translate_draw_el(el, ddx, ddy);
                                        }
                                    }
                                }
                            });
                            // Move selected cards.
                            items.update(|list| {
                                for item in &sel {
                                    if let SelectedItem::Card(ref card_id) = item {
                                        if let Some(c) = list.iter_mut().find(|c| &c.id == card_id) {
                                            c.x += ddx;
                                            c.y += ddy;
                                        }
                                    }
                                }
                            });
                        } else {
                            // Hover: hit-test against all draw elements (reverse for z-order).
                            let els = draw_elements.get_untracked();
                            let mut found: Option<SelectedItem> = None;
                            for (i, el) in els.iter().enumerate().rev() {
                                if point_in_bbox(cx, cy, el, 5.0) {
                                    found = Some(SelectedItem::Drawing(i));
                                    break;
                                }
                            }
                            // Also hit-test cards if notes are visible.
                            if found.is_none() && show_notes.get_untracked() {
                                let card_list = items.get_untracked();
                                for card in card_list.iter().rev() {
                                    if point_in_card(cx, cy, card.x, card.y, 5.0) {
                                        found = Some(SelectedItem::Card(card.id.clone()));
                                        break;
                                    }
                                }
                            }
                            hovered_element.set(found);
                        }
                        return;
                    }

                    if is_drawing.get_untracked() {
                        let z = zoom.get_untracked().max(0.001);
                        let (cx, cy) = to_canvas(mx - off_x, my - off_y, pan_x.get_untracked(), pan_y.get_untracked(), z);
                        let color = active_color.get_untracked();
                        let opa = active_opacity.get_untracked();
                        match active_tool.get_untracked() {
                            DrawTool::Pen => {
                                current_path.update(|p| p.push((cx, cy)));
                                let pts = current_path.get_untracked();
                                let w = pen_size.get_untracked();
                                preview_el.set(Some(DrawEl::Path { pts, color, width: w, opacity: opa }));
                            }
                            DrawTool::Eraser => {
                                current_path.update(|p| p.push((cx, cy)));
                                let pts = current_path.get_untracked();
                                let w = eraser_size.get_untracked();
                                preview_el.set(Some(DrawEl::Path { pts, color: CANVAS_BG.to_string(), width: w, opacity: 1.0 }));
                            }
                            ref tool => {
                                let (sx, sy) = drawing_from.get_untracked().unwrap_or((cx, cy));
                                preview_el.set(Some(make_shape(tool, sx, sy, cx, cy, color, opa)));
                            }
                        }
                    }
                    return; // never pan/drag-card in draw mode
                }

                // ── Mode Navigation : drag carte ──────────────────────────────
                if let Some(card_id) = dragging_card.get_untracked() {
                    let z = zoom.get_untracked().max(0.001);
                    let (ox, oy) = drag_offset.get_untracked();
                    let (cx, cy) = to_canvas(mx, my, pan_x.get_untracked(), pan_y.get_untracked(), z);
                    items.update(|list| {
                        if let Some(item) = list.iter_mut().find(|c| c.id == card_id) {
                            item.x = cx - ox;
                            item.y = cy - oy;
                        }
                    });
                    return;
                }

                // ── Mode Navigation : pan fond ────────────────────────────────
                if pan_dragging.get_untracked() {
                    let (sx, sy, px0, py0) = pan_start.get_untracked();
                    pan_x.set(px0 + (mx - sx));
                    pan_y.set(py0 + (my - sy));
                }
            });

        let on_up =
            Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |ev: web_sys::MouseEvent| {
                // Guard: component may be disposed — skip if signals are dead.
                let Some((off_x, off_y)) = container_off.try_get_untracked() else { return; };
                let mx = ev.client_x() as f64;
                let my = ev.client_y() as f64;

                // ── Mode Dessin : Hand tool mouseup ──────────────────────────
                if draw_mode.get_untracked() && active_tool.get_untracked() == DrawTool::Hand {
                    if hand_dragging.get_untracked() {
                        let z = zoom.get_untracked().max(0.001);
                        let (cx, cy) = to_canvas(mx - off_x, my - off_y, pan_x.get_untracked(), pan_y.get_untracked(), z);
                        let (sx, sy) = hand_drag_start.get_untracked();
                        let dist = ((cx - sx).powi(2) + (cy - sy).powi(2)).sqrt();
                        hand_dragging.set(false);

                        // If distance < 5 it was a click, not a drag: selection
                        // was already handled in mousedown, nothing else needed.
                        if dist >= 5.0 {
                            // Record move in undo stack.
                            let (adx, ady) = hand_drag_accum.get_untracked();
                            if adx.abs() > 0.1 || ady.abs() > 0.1 {
                                let sel = selected_elements.get_untracked();
                                let entries: Vec<(SelectedItem, f64, f64)> =
                                    sel.into_iter().map(|s| (s, adx, ady)).collect();
                                undo_stack.update(|stk| stk.push(UndoAction::MoveItems { items: entries }));
                            }
                        }
                    }
                    return;
                }

                // ── Mode Dessin : finalisation du trait ───────────────────────
                if draw_mode.get_untracked() && is_drawing.get_untracked() {
                    let z = zoom.get_untracked().max(0.001);
                    let (cx, cy) = to_canvas(mx - off_x, my - off_y, pan_x.get_untracked(), pan_y.get_untracked(), z);
                    let color   = active_color.get_untracked();
                    let opa     = active_opacity.get_untracked();
                    let el = match active_tool.get_untracked() {
                        DrawTool::Pen => {
                            let mut pts = current_path.get_untracked();
                            pts.push((cx, cy));
                            let w = pen_size.get_untracked();
                            DrawEl::Path { pts, color, width: w, opacity: opa }
                        }
                        DrawTool::Eraser => {
                            let mut pts = current_path.get_untracked();
                            pts.push((cx, cy));
                            let w = eraser_size.get_untracked();
                            DrawEl::Path { pts, color: CANVAS_BG.to_string(), width: w, opacity: 1.0 }
                        }
                        ref tool => {
                            let (sx, sy) = drawing_from.get_untracked().unwrap_or((cx, cy));
                            make_shape(tool, sx, sy, cx, cy, color, opa)
                        }
                    };
                    if has_extent(&el) {
                        let is_fill = matches!(el, DrawEl::FillRect { .. });
                        // FillRect goes to front of vec (rendered first = behind strokes).
                        // All other elements go to end (rendered last = on top).
                        draw_elements.update(|v| {
                            if is_fill {
                                v.insert(0, el);
                            } else {
                                v.push(el);
                            }
                        });
                        // Record in undo stack.
                        let idx = if is_fill { 0 } else { draw_elements.get_untracked().len() - 1 };
                        undo_stack.update(|stk| stk.push(UndoAction::AddDrawing(idx)));
                    }
                    is_drawing.set(false);
                    drawing_from.set(None);
                    current_path.set(vec![]);
                    preview_el.set(None);
                    return;
                }

                // ── Mode Navigation : détection clic carte → ouvrir note ──────
                if let Some(card_id) = dragging_card.get_untracked() {
                    let (sx, sy) = drag_start.get_untracked();
                    let dx = mx - sx;
                    let dy = my - sy;
                    if (dx * dx + dy * dy).sqrt() < 5.0 {
                        let path  = card_id.clone();
                        let vault = state.vault_path.get_untracked();
                        spawn_local(async move {
                            let abs_path = if path.starts_with('/') || path.contains(':') {
                                path.clone()
                            } else {
                                format!("{}/{}", vault.trim_end_matches('/'), path)
                            };
                            match crate::ipc::get_note(&abs_path).await {
                                Ok(content) => {
                                    tab_mgr.open(&abs_path, &path, &content);
                                    state.active_view.set(crate::app::ActiveView::Editor);
                                }
                                Err(e) => {
                                    leptos::logging::warn!(
                                        "canvas_view: get_note failed for {}: {}",
                                        path, e
                                    );
                                }
                            }
                        });
                    }
                    dragging_card.set(None);
                }
                pan_dragging.set(false);
            });

        // ── Raccourci clavier Ctrl+Z / Shift+Z : undo ────────────────────
        let on_keydown =
            Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |ev: web_sys::KeyboardEvent| {
                // Guard: component may be disposed — skip if signals are dead.
                let Some(dm) = draw_mode.try_get_untracked() else { return; };
                if !dm { return; }
                let is_undo = (ev.key() == "z" || ev.key() == "Z")
                    && ((ev.ctrl_key() || ev.meta_key()) || ev.shift_key());
                if !is_undo { return; }
                ev.prevent_default();

                // Pop the last action from the undo stack.
                let action = undo_stack.try_update(|stk| stk.pop()).flatten();
                match action {
                    Some(UndoAction::AddDrawing(idx)) => {
                        draw_elements.update(|v| {
                            if idx < v.len() { v.remove(idx); }
                        });
                    }
                    Some(UndoAction::MoveItems { items: moved }) => {
                        // Reverse the move: apply -dx, -dy.
                        for (sel_item, dx, dy) in &moved {
                            match sel_item {
                                SelectedItem::Drawing(idx) => {
                                    draw_elements.update(|els| {
                                        if let Some(el) = els.get_mut(*idx) {
                                            translate_draw_el(el, -dx, -dy);
                                        }
                                    });
                                }
                                SelectedItem::Card(ref card_id) => {
                                    items.update(|list: &mut Vec<LocalCanvasItem>| {
                                        if let Some(c) = list.iter_mut().find(|c| &c.id == card_id) {
                                            c.x -= dx;
                                            c.y -= dy;
                                        }
                                    });
                                }
                            }
                        }
                    }
                    None => {
                        // Fallback: if undo stack empty, just pop last drawing.
                        draw_elements.update(|v| { v.pop(); });
                    }
                }
            });

        if let Some(window) = web_sys::window() {
            let _ = window.add_event_listener_with_callback("mousemove", on_move.as_ref().unchecked_ref());
            let _ = window.add_event_listener_with_callback("mouseup",   on_up.as_ref().unchecked_ref());
            let _ = window.add_event_listener_with_callback("keydown",   on_keydown.as_ref().unchecked_ref());
        }
        on_move.forget();
        on_up.forget();
        on_keydown.forget();
    }

    // ── Vue ───────────────────────────────────────────────────────────────────

    view! {
        // Conteneur principal — clip + contexte d'empilement.
        <div
            style=move || {
                let cursor = if draw_mode.get() && active_tool.get() == DrawTool::Hand {
                    if hand_dragging.get() { "grabbing" } else { "pointer" }
                } else {
                    "default"
                };
                format!("position:relative;width:100%;height:100%;overflow:hidden;background:var(--trl-abyss-deep);cursor:{cursor};")
            }
            on:mousedown=move |ev: web_sys::MouseEvent| {
                if ev.button() != 0 { return; }

                // Calibrer l'offset conteneur a chaque mousedown pour que
                // les coordonnees canvas soient correctes malgre la sidebar,
                // la toolbar, et tout scroll/resize intermediaire.
                if let Some(target) = ev.current_target() {
                    if let Some(el) = target.dyn_ref::<web_sys::Element>() {
                        let rect = el.get_bounding_client_rect();
                        container_off.set((rect.left(), rect.top()));
                    }
                }
                let (off_x, off_y) = container_off.get_untracked();

                if draw_mode.get_untracked() {
                    // Fermer le popup taille si on clique sur le canvas.
                    size_popup.set(None);
                    let mx = ev.client_x() as f64 - off_x;
                    let my = ev.client_y() as f64 - off_y;
                    let z  = zoom.get_untracked().max(0.001);
                    let (cx, cy) = to_canvas(mx, my, pan_x.get_untracked(), pan_y.get_untracked(), z);

                    let tool = active_tool.get_untracked();

                    // ── Hand tool : selection / drag start ────────────────────
                    if tool == DrawTool::Hand {
                        let ctrl = ev.ctrl_key() || ev.meta_key();

                        // Hit-test drawing elements (reverse for z-order).
                        let els = draw_elements.get_untracked();
                        let mut hit: Option<SelectedItem> = None;
                        for (i, el) in els.iter().enumerate().rev() {
                            if point_in_bbox(cx, cy, el, 5.0) {
                                hit = Some(SelectedItem::Drawing(i));
                                break;
                            }
                        }
                        // Hit-test cards if no drawing element was hit.
                        if hit.is_none() && show_notes.get_untracked() {
                            let card_list = items.get_untracked();
                            for card in card_list.iter().rev() {
                                if point_in_card(cx, cy, card.x, card.y, 5.0) {
                                    hit = Some(SelectedItem::Card(card.id.clone()));
                                    break;
                                }
                            }
                        }

                        match hit {
                            Some(item) => {
                                if ctrl {
                                    // Toggle item in selection.
                                    selected_elements.update(|sel| {
                                        if let Some(pos) = sel.iter().position(|s| s == &item) {
                                            sel.remove(pos);
                                        } else {
                                            sel.push(item.clone());
                                        }
                                    });
                                } else {
                                    let already = selected_elements.get_untracked().contains(&item);
                                    if !already {
                                        selected_elements.set(vec![item.clone()]);
                                    }
                                }
                                // Start drag.
                                hand_dragging.set(true);
                                hand_drag_start.set((cx, cy));
                                hand_drag_last.set((cx, cy));
                                hand_drag_accum.set((0.0, 0.0));
                            }
                            None => {
                                // Click on empty space: deselect all.
                                if !ctrl {
                                    selected_elements.set(vec![]);
                                }
                            }
                        }
                        return;
                    }

                    // ── Dessin : initialisation du trait ──────────────────────
                    is_drawing.set(true);
                    drawing_from.set(Some((cx, cy)));
                    if tool == DrawTool::Pen || tool == DrawTool::Eraser {
                        current_path.set(vec![(cx, cy)]);
                    }
                    return;
                }

                // ── Navigation : démarrage du pan ─────────────────────────────
                if dragging_card.get_untracked().is_none() {
                    pan_dragging.set(true);
                    pan_start.set((
                        ev.client_x() as f64,
                        ev.client_y() as f64,
                        pan_x.get_untracked(),
                        pan_y.get_untracked(),
                    ));
                }
            }
            on:wheel=move |ev: web_sys::WheelEvent| {
                ev.prevent_default();
                let delta = ev.delta_y();
                zoom.update(|z| { *z = (*z * (1.0 - delta * 0.001)).clamp(0.2, 5.0); });
            }
        >
            // ── Viewport : transform CSS pan + zoom ───────────────────────────
            <div
                style=move || format!(
                    "position:absolute;width:0;height:0;\
                     transform:translate({}px,{}px) scale({:.3});\
                     transform-origin:0 0;",
                    pan_x.get(), pan_y.get(), zoom.get()
                )
            >
                // ── Cartes notes ──────────────────────────────────────────────
                //
                // collect_view() (pas <For>) : Leptos <For> réutilise les noeuds DOM
                // pour les clés identiques — les positions x/y ne se mettraient pas
                // à jour pendant le drag.
                //
                // En mode whiteboard (`show_notes == false`), les cartes sont masquees
                // (vec vide → collect_view vide).
                {move || {
                    let list = if show_notes.get() {
                        let all = items.get();
                        // Apply tag filter if active.
                        match filtered_notes.get() {
                            Some(allowed) => all.into_iter()
                                .filter(|item| allowed.contains(&item.id))
                                .collect::<Vec<_>>(),
                            None => all,
                        }
                    } else { vec![] };
                    list.into_iter().map(|item| {
                    let id       = item.id.clone();
                    let id_down  = id.clone();
                    let x        = item.x;
                    let y        = item.y;
                    let title    = item.title.clone();
                    view! {
                        <div
                            style=format!(
                                "position:absolute;left:{x}px;top:{y}px;z-index:2;\
                                 background:var(--trl-abyss);border:1px solid var(--trl-abyss-light);\
                                 border-radius:6px;padding:10px 14px;\
                                 min-width:180px;max-width:180px;\
                                 cursor:grab;user-select:none;\
                                 box-shadow:0 2px 8px rgba(0,0,0,0.4);\
                                 color:var(--trl-text);font-size:13px;line-height:1.4;"
                            )
                            on:mousedown=move |ev: web_sys::MouseEvent| {
                                // En mode dessin : laisser l'événement remonter
                                // jusqu'au conteneur principal pour démarrer un trait.
                                if draw_mode.get_untracked() { return; }
                                ev.stop_propagation();
                                if ev.button() != 0 { return; }
                                drag_start.set((ev.client_x() as f64, ev.client_y() as f64));
                                let z  = zoom.get_untracked().max(0.001);
                                let mx = ev.client_x() as f64;
                                let my = ev.client_y() as f64;
                                let ox = (mx - pan_x.get_untracked()) / z - x;
                                let oy = (my - pan_y.get_untracked()) / z - y;
                                drag_offset.set((ox, oy));
                                dragging_card.set(Some(id_down.clone()));
                            }
                        >
                            <span style="display:block;overflow:hidden;text-overflow:ellipsis;\
                                         white-space:nowrap;color:var(--trl-cyan);font-weight:600;\
                                         font-size:12px;margin-bottom:4px;">
                                { title }
                            </span>
                            <span style="display:block;overflow:hidden;text-overflow:ellipsis;\
                                         white-space:nowrap;color:var(--trl-text-secondary);font-size:11px;">
                                { id.clone() }
                            </span>
                        </div>
                    }
                }).collect_view()
                }}

                // ── Overlay SVG dessin (coordonnées canvas, co-transformé) ────
                //
                // # Viewport SVG
                //
                // Dimensions EXPLICITES via attributs SVG `width`/`height` +
                // `viewBox` identique : user units mappent 1:1 aux CSS pixels.
                // `overflow="visible"` en attribut SVG (pas CSS) pour garantir
                // que les éléments au-delà du viewport (20000×20000) soient
                // quand même peints — défense en profondeur contre les UA
                // stylesheets Chromium/WebView2 qui forcent `overflow:hidden`
                // sur l'outer `<svg>` malgré la CSS `overflow:visible`.
                //
                // ⚠ NB architectural : un précédent design avait `width:1px;
                // height:1px;overflow:visible` (CSS uniquement). Ça semblait
                // marcher en théorie mais WebView2 appliquait un clip à 1×1 px
                // malgré `overflow:visible`. Les éléments étaient créés dans
                // le DOM mais JAMAIS peints. Le fix impose des dimensions
                // réelles pour le viewport SVG.
                //
                // # Positionnement
                //
                // `position:absolute;top:0;left:0` dans le viewport-div qui
                // porte le `transform:translate/scale` → pan & zoom héritent.
                // `pointer-events:none` → transparent aux events (les clics
                // passent au conteneur parent qui gère pan/drag/dessin).
                //
                // # Taille du viewport (20000 px)
                //
                // Valeur choisie pour couvrir la majorité des usages canvas
                // (pan large, infinite-canvas feel) sans exploser la mémoire
                // (SVG reste vectoriel — seules les zones visibles sont
                // rastérisées par le compositing). Augmenter si un utilisateur
                // dépasse en pratique.
                <svg
                    width="20000"
                    height="20000"
                    viewBox="0 0 20000 20000"
                    overflow="visible"
                    preserveAspectRatio="none"
                    style="position:absolute;top:0;left:0;pointer-events:none;z-index:1;"
                >
                    // Éléments finalisés
                    {move || draw_elements.get().iter().map(draw_el_view).collect_view()}
                    // Aperçu live
                    {move || preview_el.get().as_ref().map(draw_el_view)}
                    // Selection overlay: bounding boxes for hovered and selected items
                    {move || {
                        let els = draw_elements.get();
                        let card_list = items.get();
                        let sel = selected_elements.get();
                        let hov = hovered_element.get();
                        let mut rects: Vec<AnyView> = Vec::new();

                        // Helper: get bbox for a SelectedItem.
                        let get_bbox = |item: &SelectedItem| -> Option<(f64, f64, f64, f64)> {
                            match item {
                                SelectedItem::Drawing(idx) => {
                                    els.get(*idx).map(bounding_box)
                                }
                                SelectedItem::Card(ref id) => {
                                    card_list.iter()
                                        .find(|c| &c.id == id)
                                        .map(|c| (c.x, c.y, 180.0, 50.0))
                                }
                            }
                        };

                        // Hover highlight (dashed, 1px).
                        if let Some(ref h) = hov {
                            // Only show hover if not already selected.
                            if !sel.contains(h) {
                                if let Some((bx, by, bw, bh)) = get_bbox(h) {
                                    let pad = 4.0;
                                    rects.push(view! {
                                        <rect
                                            x={(bx - pad).to_string()}
                                            y={(by - pad).to_string()}
                                            width={(bw + pad * 2.0).to_string()}
                                            height={(bh + pad * 2.0).to_string()}
                                            style="fill:none;stroke:#3b82f6;stroke-width:1;stroke-dasharray:5,3;"
                                        />
                                    }.into_any());
                                }
                            }
                        }

                        // Selected items (solid, 2px).
                        for item in &sel {
                            if let Some((bx, by, bw, bh)) = get_bbox(item) {
                                let pad = 4.0;
                                rects.push(view! {
                                    <rect
                                        x={(bx - pad).to_string()}
                                        y={(by - pad).to_string()}
                                        width={(bw + pad * 2.0).to_string()}
                                        height={(bh + pad * 2.0).to_string()}
                                        style="fill:none;stroke:#3b82f6;stroke-width:2;"
                                    />
                                }.into_any());
                            }
                        }

                        rects.collect_view()
                    }}
                </svg>
            </div>

            // ── Toolbar ────────────────────────────────────────────────────────
            <div style="position:absolute;top:8px;left:8px;z-index:10;\
                        display:flex;gap:4px;align-items:center;\
                        background:rgba(15,23,42,0.92);border:1px solid var(--trl-abyss-light);\
                        border-radius:6px;padding:4px 8px;pointer-events:all;">

                // Bouton mode toggle
                <button
                    style=move || format!(
                        "padding:3px 10px;border-radius:4px;font-size:12px;\
                         cursor:pointer;border:none;transition:background 0.15s;{}",
                        if draw_mode.get() {
                            "background:var(--trl-cyan);color:var(--trl-text);"
                        } else {
                            "background:var(--trl-abyss);color:var(--trl-text-secondary);"
                        }
                    )
                    on:click=move |_| {
                        draw_mode.update(|v| *v = !*v);
                        // Cleanup drawing state on mode switch.
                        is_drawing.set(false);
                        preview_el.set(None);
                        current_path.set(vec![]);
                        // Cleanup Hand tool state.
                        selected_elements.set(vec![]);
                        hovered_element.set(None);
                        hand_dragging.set(false);
                    }
                >
                    { move || if draw_mode.get() { "Dessin" } else { "Nav" } }
                </button>

                // Toggle whiteboard (masquer/afficher les cartes notes)
                <button
                    title="Tableau blanc / Notes"
                    style=move || format!(
                        "padding:4px 10px;border-radius:6px;font-size:12px;\
                         font-weight:600;cursor:pointer;border:1px solid var(--trl-abyss-light);{}",
                        if show_notes.get() {
                            "background:transparent;color:var(--trl-text-secondary);"
                        } else {
                            "background:var(--trl-cyan);color:var(--trl-abyss-deep);"
                        }
                    )
                    on:click=move |_| show_notes.update(|v| *v = !*v)
                >
                    { move || if show_notes.get() { "Notes" } else { "Whiteboard" } }
                </button>

                // ── Filtre par tag (select dropdown) ─────────────────────
                //
                // Visible uniquement quand les notes sont affichees.
                // "Tous" = aucun filtre ; chaque tag filtre les cartes.
                { move || {
                    if !show_notes.get() {
                        return view! { <span/> }.into_any();
                    }
                    let tags = available_tags.get();
                    if tags.is_empty() {
                        return view! { <span/> }.into_any();
                    }
                    view! {
                        <select
                            title="Filtrer par tag"
                            style="padding:2px 6px;border-radius:4px;font-size:11px;\
                                   background:var(--trl-abyss);color:var(--trl-text-secondary);border:1px solid var(--trl-abyss-light);\
                                   cursor:pointer;max-width:130px;"
                            on:change=move |ev: web_sys::Event| {
                                if let Some(target) = ev.target() {
                                    if let Ok(select) = target.dyn_into::<web_sys::HtmlSelectElement>() {
                                        let val = select.value();
                                        if val.is_empty() || val == "__all__" {
                                            filter_tag.set(None);
                                        } else {
                                            filter_tag.set(Some(val));
                                        }
                                    }
                                }
                            }
                        >
                            <option value="__all__">"Tous"</option>
                            {tags.into_iter().map(|t| {
                                let t2 = t.clone();
                                view! { <option value={t}>{t2}</option> }
                            }).collect_view()}
                        </select>
                    }.into_any()
                }}

                // Outils + palette (visibles uniquement en mode dessin)
                { move || {
                    if !draw_mode.get() {
                        return view! { <span/> }.into_any();
                    }

                    // ── Séparateur ────────────────────────────────────────────
                    let sep = || view! {
                        <span style="display:inline-block;width:1px;height:18px;\
                                     background:var(--trl-abyss-light);margin:0 3px;"/>
                    };

                    // ── Boutons outils ────────────────────────────────────────
                    let tool_list: &[(&str, DrawTool)] = &[
                        ("✋",  DrawTool::Hand),
                        ("✏",  DrawTool::Pen),
                        ("⌫",  DrawTool::Eraser),
                        ("◉",  DrawTool::Fill),
                        ("╱",  DrawTool::Line),
                        ("→",  DrawTool::Arrow),
                        ("▭",  DrawTool::Rect),
                        ("◯",  DrawTool::Circle),
                    ];
                    let tool_btns = tool_list.iter().map(|(label, tool)| {
                        let tool_cmp  = tool.clone();
                        let tool_set  = tool.clone();
                        let label_s   = *label;
                        // Right-click opens size popup only for Pen and Eraser
                        let has_size = matches!(tool, DrawTool::Pen | DrawTool::Eraser);
                        let tool_ctx  = tool.clone();
                        view! {
                            <button
                                style=move || format!(
                                    "padding:2px 7px;border-radius:4px;font-size:13px;\
                                     cursor:pointer;border:none;{}",
                                    if active_tool.get() == tool_cmp {
                                        "background:var(--trl-abyss-light);color:var(--trl-text);"
                                    } else {
                                        "background:transparent;color:var(--trl-text-tertiary);"
                                    }
                                )
                                on:click=move |_| {
                                    size_popup.set(None);
                                    active_tool.set(tool_set.clone());
                                }
                                on:contextmenu={
                                    let ctx_tool = tool_ctx.clone();
                                    move |ev: web_sys::MouseEvent| {
                                        ev.prevent_default();
                                        if has_size {
                                            // Toggle popup for this tool
                                            let current = size_popup.get_untracked();
                                            if current.as_ref() == Some(&ctx_tool) {
                                                size_popup.set(None);
                                            } else {
                                                active_tool.set(ctx_tool.clone());
                                                size_popup.set(Some(ctx_tool.clone()));
                                            }
                                        }
                                    }
                                }
                            >{ label_s }</button>
                        }
                    }).collect_view();

                    // ── Pastilles couleur ─────────────────────────────────────
                    // Quatre alias du meme hex car chaque attribut Leptos peut capturer
                    // la valeur par move independamment (title / style / on:click /
                    // comparaison dans la closure style). Eviter le clone dans les
                    // closures en pre-clonant une fois par usage.
                    let color_btns = PALETTE.iter().map(|(_label, hex)| {
                        let hex_s     = hex.to_string();
                        let hex_title = hex_s.clone();
                        let hex_bg    = hex_s.clone();
                        let hex_cmp   = hex_s.clone();
                        let hex_set   = hex_s;
                        view! {
                            <button
                                title={hex_title}
                                style=move || format!(
                                    "width:16px;height:16px;border-radius:50%;\
                                     cursor:pointer;background:{hex_bg};\
                                     border:2px solid {};flex-shrink:0;",
                                    if active_color.get() == hex_cmp { "var(--trl-gray-light)" } else { "transparent" }
                                )
                                on:click={
                                    let c = hex_set.clone();
                                    move |_| active_color.set(c.clone())
                                }
                            />
                        }
                    }).collect_view();

                    // ── Color picker natif ────────────────────────────
                    // Un <input type="color"> HTML5 permet de choisir une
                    // couleur libre au-dela des 6 pastilles predefines.
                    let picker_hex = active_color.get_untracked();
                    let color_picker = view! {
                        <input
                            type="color"
                            title="Couleur libre"
                            value={picker_hex}
                            style="width:22px;height:22px;padding:0;border:none;\
                                   border-radius:4px;cursor:pointer;background:transparent;\
                                   flex-shrink:0;"
                            on:input=move |ev: web_sys::Event| {
                                if let Some(target) = ev.target() {
                                    if let Ok(input) = target.dyn_into::<web_sys::HtmlInputElement>() {
                                        active_color.set(input.value());
                                    }
                                }
                            }
                        />
                    };

                    // ── Opacity slider (inline, next to color picker) ────────
                    let opacity_slider = {
                        let cur_opa = active_opacity.get_untracked();
                        let pct_val = (cur_opa * 100.0) as i32;
                        view! {
                            <div
                                title="Opacité"
                                style="display:flex;align-items:center;gap:3px;\
                                       margin-left:2px;"
                                on:mousedown=move |ev: web_sys::MouseEvent| { ev.stop_propagation(); }
                            >
                                // Opacity icon (droplet-like)
                                <span style="font-size:12px;color:var(--trl-text-secondary);cursor:default;user-select:none;"
                                >{ "💧" }</span>
                                // Slider 1–100
                                <input
                                    type="range"
                                    min="1"
                                    max="100"
                                    value={pct_val.to_string()}
                                    style="width:56px;height:3px;accent-color:var(--trl-cyan);cursor:pointer;"
                                    on:input=move |ev: web_sys::Event| {
                                        if let Some(target) = ev.target() {
                                            if let Ok(input) = target.dyn_into::<web_sys::HtmlInputElement>() {
                                                if let Ok(v) = input.value().parse::<f64>() {
                                                    active_opacity.set(v / 100.0);
                                                }
                                            }
                                        }
                                    }
                                />
                                // Label %
                                <span style="color:var(--trl-text-secondary);font-size:10px;white-space:nowrap;\
                                             min-width:26px;text-align:right;">
                                    {move || format!("{}%", (active_opacity.get() * 100.0) as i32)}
                                </span>
                            </div>
                        }
                    };

                    // ── Size popup (right-click on Pen / Eraser) ─────────────
                    // Slider continu (rond + trait) avec prévisualisation du cercle.
                    let size_popup_view = move || {
                        let popup_tool = match size_popup.get() {
                            Some(t) => t,
                            None => return view! { <span/> }.into_any(),
                        };
                        let is_pen = popup_tool == DrawTool::Pen;
                        let sig = if is_pen { pen_size } else { eraser_size };
                        let cur = sig.get();
                        // Map [BRUSH_MIN..BRUSH_MAX] to [0..1000] for the slider.
                        let pct = ((cur - BRUSH_MIN) / (BRUSH_MAX - BRUSH_MIN) * 1000.0) as i32;
                        let pct = pct.clamp(0, 1000);
                        // Preview circle diameter (clamped for display).
                        let preview_d = cur.clamp(2.0, 40.0);
                        view! {
                            <div
                                style="position:absolute;top:100%;left:50%;transform:translateX(-50%);\
                                       margin-top:6px;background:var(--trl-abyss);border:1px solid var(--trl-abyss-light);\
                                       border-radius:8px;padding:8px 12px;z-index:100;\
                                       box-shadow:0 4px 12px rgba(0,0,0,0.5);\
                                       display:flex;align-items:center;gap:8px;min-width:200px;"
                                // Prevent mousedown from propagating to the canvas
                                // (which would start drawing).
                                on:mousedown=move |ev: web_sys::MouseEvent| { ev.stop_propagation(); }
                            >
                                // Preview circle
                                <div style=format!(
                                    "width:{preview_d:.0}px;height:{preview_d:.0}px;\
                                     border-radius:50%;flex-shrink:0;\
                                     background:{};opacity:0.85;",
                                    if is_pen { active_color.get_untracked() } else { "var(--trl-text-secondary)".to_string() }
                                )/>
                                // Slider
                                <input
                                    type="range"
                                    min="0"
                                    max="1000"
                                    value={pct.to_string()}
                                    style="flex:1;height:4px;accent-color:var(--trl-cyan);cursor:pointer;"
                                    on:input=move |ev: web_sys::Event| {
                                        if let Some(target) = ev.target() {
                                            if let Ok(input) = target.dyn_into::<web_sys::HtmlInputElement>() {
                                                if let Ok(v) = input.value().parse::<f64>() {
                                                    let px = BRUSH_MIN + (v / 1000.0) * (BRUSH_MAX - BRUSH_MIN);
                                                    sig.set(px);
                                                }
                                            }
                                        }
                                    }
                                />
                                // Label px
                                <span style="color:var(--trl-text-secondary);font-size:10px;white-space:nowrap;min-width:32px;\
                                             text-align:right;">
                                    {format!("{cur:.1}")}
                                </span>
                            </div>
                        }.into_any()
                    };

                    view! {
                        <div style="display:contents;">
                            {sep()}
                            {tool_btns}
                            {size_popup_view}
                            {sep()}
                            {color_btns}
                            {color_picker}
                            {opacity_slider}
                            {sep()}
                            // Undo (dernier action — dessin ou déplacement)
                            <button
                                title="Annuler (Ctrl+Z)"
                                style="padding:2px 7px;border-radius:4px;font-size:13px;\
                                       cursor:pointer;border:none;\
                                       background:transparent;color:var(--trl-text-tertiary);"
                                on:click=move |_| {
                                    let action = undo_stack.try_update(|stk| stk.pop()).flatten();
                                    match action {
                                        Some(UndoAction::AddDrawing(idx)) => {
                                            draw_elements.update(|v| {
                                                if idx < v.len() { v.remove(idx); }
                                            });
                                        }
                                        Some(UndoAction::MoveItems { items: entries }) => {
                                            for (sel_item, dx, dy) in &entries {
                                                match sel_item {
                                                    SelectedItem::Drawing(idx) => {
                                                        draw_elements.update(|els| {
                                                            if let Some(el) = els.get_mut(*idx) {
                                                                translate_draw_el(el, -dx, -dy);
                                                            }
                                                        });
                                                    }
                                                    SelectedItem::Card(ref card_id) => {
                                                        items.update(|list: &mut Vec<LocalCanvasItem>| {
                                                            if let Some(c) = list.iter_mut().find(|c| &c.id == card_id) {
                                                                c.x -= dx;
                                                                c.y -= dy;
                                                            }
                                                        });
                                                    }
                                                }
                                            }
                                        }
                                        None => {
                                            draw_elements.update(|v| { v.pop(); });
                                        }
                                    }
                                }
                            >{ "↩" }</button>
                            // Clear all drawings
                            <button
                                title="Effacer tous les dessins"
                                style="padding:2px 7px;border-radius:4px;font-size:13px;\
                                       cursor:pointer;border:none;\
                                       background:transparent;color:var(--trl-text-tertiary);"
                                on:click=move |_| {
                                    draw_elements.set(vec![]);
                                    undo_stack.set(vec![]);
                                }
                            >{ "✕" }</button>
                            // Export SVG (save dialog)
                            <button
                                title="Exporter en SVG"
                                style="padding:2px 7px;border-radius:4px;font-size:13px;\
                                       cursor:pointer;border:none;\
                                       background:transparent;color:var(--trl-text-tertiary);"
                                on:click=move |_| {
                                    let els = draw_elements.get_untracked();
                                    spawn_local(async move {
                                        let svg = build_export_svg(&els);
                                        if let Err(e) = crate::ipc::export_canvas_svg(svg).await {
                                            leptos::logging::warn!("export_canvas_svg failed: {e}");
                                        }
                                    });
                                }
                            >{ "SVG" }</button>
                        </div>
                    }
                    .into_any()
                }}
            </div>

            // ── HUD bas-droite ────────────────────────────────────────────────
            <div style="position:absolute;bottom:12px;right:12px;\
                        display:flex;flex-direction:column;align-items:flex-end;\
                        gap:4px;pointer-events:none;">
                <span style="color:var(--trl-text-secondary);font-size:11px;\
                              background:rgba(15,23,42,0.8);padding:2px 6px;border-radius:4px;">
                    { move || {
                        let total = items.get().len();
                        let shown = match filtered_notes.get() {
                            Some(ref f) => f.len(),
                            None => total,
                        };
                        if shown == total {
                            format!("{}% — {} notes", (zoom.get() * 100.0) as i32, total)
                        } else {
                            format!("{}% — {}/{} notes", (zoom.get() * 100.0) as i32, shown, total)
                        }
                    }}
                </span>
                <button
                    style="pointer-events:all;background:var(--trl-abyss);border:1px solid var(--trl-abyss-light);\
                           border-radius:4px;color:var(--trl-text-secondary);font-size:11px;\
                           padding:3px 8px;cursor:pointer;"
                    on:click=move |_| { pan_x.set(0.0); pan_y.set(0.0); zoom.set(1.0); }
                >
                    "Reset view"
                </button>
            </div>
        </div>
    }
}

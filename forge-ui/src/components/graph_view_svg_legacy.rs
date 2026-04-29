//! Force-directed graph visualization with click-to-open nodes.
//!
//! Organic layout inspired by Obsidian's native graph: nodes find natural
//! positions through repulsion/attraction forces, and the camera auto-fits
//! the bounding box during simulation.  Supports mouse, trackpad, and touch.

use leptos::prelude::*;
use leptos::task::spawn_local;
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use crate::app::AppState;
use crate::ipc;
use crate::tab_manager::TabManager;

// ───────────────────────── Performance helpers ─────────────────────────

/// High-resolution timestamp in milliseconds via `performance.now()`.
fn perf_now() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or(0.0)
}

// ───────────────────────── Data ─────────────────────────

#[derive(Clone, Debug)]
struct NodePos {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    degree: usize,
}

const MAX_SIM_TICKS: u32 = 250;
/// Reference viewBox dimensions (logical coordinate space).
const VB_W: f64 = 1000.0;
const VB_H: f64 = 750.0;

// ───────────────────────── Physics ─────────────────────────

struct SimParams {
    k: f64,
    damping: f64,
    initial_temp: f64,
    repulsion: f64,
    attraction: f64,
    radius: f64,
}

impl SimParams {
    /// Obsidian-like parameters: strong repulsion pushes nodes apart,
    /// moderate edge attraction keeps clusters together, weak gravity
    /// prevents infinite drift.
    fn for_count(n: usize) -> Self {
        if n <= 50 {
            Self { k: 90.0, damping: 0.90, initial_temp: 1.0, repulsion: 1.2, attraction: 0.06, radius: 180.0 }
        } else if n <= 200 {
            Self { k: 110.0, damping: 0.87, initial_temp: 0.9, repulsion: 0.7, attraction: 0.04, radius: 350.0 }
        } else {
            let s = (n as f64).sqrt();
            Self { k: 130.0, damping: 0.82, initial_temp: 0.6, repulsion: 0.35, attraction: 0.025, radius: s * 22.0 }
        }
    }
}

fn force_tick(
    pos: &mut HashMap<String, NodePos>,
    edge_list: &[(String, String)],
    params: &SimParams,
    tick: u32,
    pinned_node: Option<&str>,
) -> bool {
    if pos.is_empty() { return false; }

    let n = pos.len();
    let k = params.k;
    let progress = tick as f64 / MAX_SIM_TICKS as f64;
    // When a node is pinned (interactive drag), use a constant moderate temperature
    // so connected nodes keep following smoothly instead of freezing.
    let temp = if pinned_node.is_some() {
        0.3
    } else {
        params.initial_temp * (1.0 - progress).max(0.01)
    };

    let ids: Vec<_> = pos.keys().cloned().collect();
    let cutoff_sq = if n > 300 { (3.5 * k).powi(2) } else { f64::MAX };

    // Repulsion (Coulomb-like: all pairs).
    // Forces are computed for ALL nodes including pinned — the pinned node
    // pushes/pulls others normally, it just won't move itself.
    for i in 0..ids.len() {
        for j in (i + 1)..ids.len() {
            if let (Some(n1), Some(n2)) = (pos.get(&ids[i]), pos.get(&ids[j])) {
                let dx = n2.x - n1.x;
                let dy = n2.y - n1.y;
                let dist_sq = dx * dx + dy * dy;
                if dist_sq > cutoff_sq { continue; }
                let dist = dist_sq.max(1.0).sqrt();
                let force = (k * k / dist) * params.repulsion * temp;
                let fx = force * dx / dist;
                let fy = force * dy / dist;
                if let Some(m) = pos.get_mut(&ids[i]) { m.vx -= fx; m.vy -= fy; }
                if let Some(m) = pos.get_mut(&ids[j]) { m.vx += fx; m.vy += fy; }
            }
        }
    }

    // Attraction along edges (spring-like).
    for (src, tgt) in edge_list {
        if let (Some(sp), Some(tp)) = (pos.get(src), pos.get(tgt)) {
            let dx = tp.x - sp.x;
            let dy = tp.y - sp.y;
            let dist = ((dx * dx + dy * dy).sqrt()).max(0.1);
            let force = (dist * dist / k) * params.attraction * temp;
            let fx = force * dx / dist;
            let fy = force * dy / dist;
            if let Some(m) = pos.get_mut(src) { m.vx += fx; m.vy += fy; }
            if let Some(m) = pos.get_mut(tgt) { m.vx -= fx; m.vy -= fy; }
        }
    }

    // Weak gravity toward origin (prevents infinite drift).
    let gravity = 0.01 * temp;
    for p in pos.values_mut() {
        p.vx -= p.x * gravity;
        p.vy -= p.y * gravity;
    }

    // Integrate — free space, no clamping.
    // Skip integration for the pinned node: its position is controlled
    // by the mouse, but its forces still affect others.
    let max_disp = k * temp;
    let mut max_v: f64 = 0.0;
    for (id, p) in pos.iter_mut() {
        if pinned_node == Some(id.as_str()) {
            // Pinned node: zero velocity, do not move.
            p.vx = 0.0;
            p.vy = 0.0;
            continue;
        }
        p.vx *= params.damping;
        p.vy *= params.damping;
        let speed = (p.vx * p.vx + p.vy * p.vy).sqrt();
        if speed > max_disp {
            let s = max_disp / speed;
            p.vx *= s;
            p.vy *= s;
        }
        p.x += p.vx;
        p.y += p.vy;
        max_v = max_v.max(p.vx.abs()).max(p.vy.abs());
    }

    max_v > 0.02
}

/// Compute the bounding box of all positions.
fn bounding_box(pos: &HashMap<String, NodePos>) -> (f64, f64, f64, f64) {
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    for p in pos.values() {
        if p.x < min_x { min_x = p.x; }
        if p.y < min_y { min_y = p.y; }
        if p.x > max_x { max_x = p.x; }
        if p.y > max_y { max_y = p.y; }
    }
    (min_x, min_y, max_x, max_y)
}

/// Compute zoom and pan to fit all nodes in the viewBox with padding.
fn fit_view(pos: &HashMap<String, NodePos>) -> (f64, f64, f64) {
    if pos.len() < 2 { return (1.0, 0.0, 0.0); }
    let (min_x, min_y, max_x, max_y) = bounding_box(pos);
    let range_x = (max_x - min_x).max(1.0);
    let range_y = (max_y - min_y).max(1.0);
    let margin = 1.2; // 20% padding
    let z_x = VB_W / (range_x * margin);
    let z_y = VB_H / (range_y * margin);
    let z = z_x.min(z_y).clamp(0.05, 8.0);
    let cx = (min_x + max_x) / 2.0;
    let cy = (min_y + max_y) / 2.0;
    // Pan offsets: viewBox center = VB_W/2 + pan_x
    let px = cx - VB_W / 2.0;
    let py = cy - VB_H / 2.0;
    (z, px, py)
}

/// Convert screen pixel coordinates to graph-space coordinates.
///
/// `screen_x`, `screen_y`: mouse position on screen.
/// `svg_rect`: bounding rect of the SVG element.
/// `z`, `px`, `py`: current zoom and pan values.
fn screen_to_graph(
    screen_x: f64,
    screen_y: f64,
    svg_left: f64,
    svg_top: f64,
    svg_width: f64,
    svg_height: f64,
    z: f64,
    px: f64,
    py: f64,
) -> (f64, f64) {
    // The viewBox is: (cx - w/2, cy - h/2, w, h)
    // where w = VB_W/z, h = VB_H/z, cx = VB_W/2 + px, cy = VB_H/2 + py.
    let vb_w = VB_W / z;
    let vb_h = VB_H / z;
    let vb_x = (VB_W / 2.0 + px) - vb_w / 2.0;
    let vb_y = (VB_H / 2.0 + py) - vb_h / 2.0;

    let frac_x = (screen_x - svg_left) / svg_width;
    let frac_y = (screen_y - svg_top) / svg_height;
    (vb_x + frac_x * vb_w, vb_y + frac_y * vb_h)
}

// ───────────────────────── Component ─────────────────────────

#[component]
pub fn GraphView() -> impl IntoView {
    let state = use_context::<AppState>().expect("AppState not found");
    let tab_mgr = use_context::<TabManager>().expect("TabManager not found");

    let edges: RwSignal<Vec<(String, String)>> = RwSignal::new(Vec::new());
    let positions: RwSignal<HashMap<String, NodePos>> = RwSignal::new(HashMap::new());
    let is_loading = RwSignal::new(true);
    let error_msg = RwSignal::new(String::new());
    let graph_ready = RwSignal::new(false);
    let zoom = RwSignal::new(1.0_f64);
    let pan_x = RwSignal::new(0.0_f64);
    let pan_y = RwSignal::new(0.0_f64);
    let node_count_info = RwSignal::new(String::new());
    let id_to_path: RwSignal<HashMap<String, String>> = RwSignal::new(HashMap::new());

    // When true, the camera auto-fits to the bounding box during simulation.
    // Disabled when the user manually pans/zooms.
    let auto_fit = RwSignal::new(true);

    // Drag state (mouse + touch).
    let is_dragging = RwSignal::new(false);
    let drag_moved = RwSignal::new(false);
    let drag_start_x = RwSignal::new(0.0_f64);
    let drag_start_y = RwSignal::new(0.0_f64);
    let drag_last_x = RwSignal::new(0.0_f64);
    let drag_last_y = RwSignal::new(0.0_f64);
    const DRAG_THRESHOLD: f64 = 3.0;

    // Pinch-to-zoom state (touch).
    let pinch_dist = RwSignal::new(0.0_f64);
    let is_pinching = RwSignal::new(false);

    // Node drag state — when set, the user is dragging a specific node.
    // The node follows the cursor while the simulation keeps running
    // (connected nodes follow gently, rest of graph adapts).
    let dragged_node_id: RwSignal<Option<String>> = RwSignal::new(None);

    // ViewBox via DOM — Leptos lowercases dynamic SVG attrs, so we use
    // document.getElementById + set_attribute to ensure correct camelCase.
    Effect::new(move |_| {
        let z = zoom.get();
        let px = pan_x.get();
        let py = pan_y.get();
        let w = VB_W / z;
        let h = VB_H / z;
        let cx = VB_W / 2.0 + px;
        let cy = VB_H / 2.0 + py;
        let vb = format!("{} {} {} {}", cx - w / 2.0, cy - h / 2.0, w, h);
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            if let Some(el) = doc.get_element_by_id("forge-graph-svg") {
                let _ = el.set_attribute("viewBox", &vb);
            }
        }
    });

    // Simulation bookkeeping (non-reactive for tick/params, signal for handle
    // so node circles can check if the sim is running).
    let sim_tick: Rc<Cell<u32>> = Rc::new(Cell::new(0));
    let sim_handle: Rc<Cell<Option<i32>>> = Rc::new(Cell::new(None));
    let sim_handle_for_nodes: RwSignal<Option<i32>> = RwSignal::new(None);
    let sim_params: Rc<Cell<Option<SimParams>>> = Rc::new(Cell::new(None));

    // ---- start_simulation ----
    let start_simulation = {
        let tick = sim_tick.clone();
        let handle = sim_handle.clone();
        let params_rc = sim_params.clone();
        Rc::new(move || {
            if let Some(h) = handle.get() {
                let _ = web_sys::window().map(|w| w.clear_interval_with_handle(h));
            }
            tick.set(0);
            auto_fit.set(true);

            let tick_c = tick.clone();
            let handle_c = handle.clone();
            let params_c = params_rc.clone();

            let sim_start = Rc::new(Cell::new(perf_now()));
            leptos::logging::log!("[perf] simulation started ({} nodes)", positions.get_untracked().len());
            let cb = Closure::<dyn FnMut()>::new(move || {
                let t = tick_c.get();

                let stop = || {
                    if let Some(h) = handle_c.get() {
                        let _ = web_sys::window().map(|w| w.clear_interval_with_handle(h));
                        handle_c.set(None);
                        sim_handle_for_nodes.set(None);
                    }
                };

                // While a node is being dragged, keep the simulation running
                // indefinitely (reset tick counter to stay in the "warm" zone).
                let node_pinned = dragged_node_id.get_untracked().is_some();
                if node_pinned && t > 50 {
                    // Keep ticking at a stable temperature by capping the tick count.
                    tick_c.set(50);
                }

                if t >= MAX_SIM_TICKS && !node_pinned {
                    stop();
                    let sim_ms = perf_now() - sim_start.get();
                    leptos::logging::log!("[perf] simulation: {:.0}ms ({} ticks)", sim_ms, t);
                    // Final fit.
                    let pos = positions.get_untracked();
                    let (z, px, py) = fit_view(&pos);
                    zoom.set(z);
                    pan_x.set(px);
                    pan_y.set(py);
                    return;
                }

                let p = params_c.take();
                if p.is_none() { return; }
                let p = p.unwrap();

                let mut pos = positions.get_untracked();
                let el = edges.get_untracked();
                let pinned = dragged_node_id.get_untracked();
                let still = force_tick(&mut pos, &el, &p, t, pinned.as_deref());

                // Auto-fit camera every 5 ticks (smooth follow).
                if auto_fit.get_untracked() && t % 5 == 0 {
                    let (z, px, py) = fit_view(&pos);
                    zoom.set(z);
                    pan_x.set(px);
                    pan_y.set(py);
                }

                positions.set(pos);
                tick_c.set(t + 1);
                params_c.set(Some(p));

                if !still && !node_pinned {
                    stop();
                    let sim_ms = perf_now() - sim_start.get();
                    leptos::logging::log!("[perf] simulation settled: {:.0}ms ({} ticks)", sim_ms, t);
                    let pos = positions.get_untracked();
                    let (z, px, py) = fit_view(&pos);
                    zoom.set(z);
                    pan_x.set(px);
                    pan_y.set(py);
                }
            });

            if let Some(w) = web_sys::window() {
                let id = w
                    .set_interval_with_callback_and_timeout_and_arguments_0(
                        cb.as_ref().unchecked_ref(),
                        33,
                    )
                    .unwrap_or(-1);
                handle.set(Some(id));
                sim_handle_for_nodes.set(Some(id));
            }
            cb.forget();
        })
    };

    // ---- restart_sim_interactive (signal-triggered) ----
    // When restart_sim_trigger is incremented, this Effect restarts the
    // simulation in interactive mode (for node drag after initial settle).
    // We use a signal + Effect instead of Rc<closure> because Leptos
    // reactive closures inside view! require Send, and Rc is not Send.
    let restart_sim_trigger = RwSignal::new(0u32);
    {
        let tick = sim_tick.clone();
        let handle = sim_handle.clone();
        let params_rc = sim_params.clone();
        Effect::new(move |prev: Option<()>| {
            let _t = restart_sim_trigger.get(); // subscribe to trigger
            if prev.is_none() { return; } // skip initial run

            // Always (re)create sim params from current node count for interactive mode.
            let n = positions.get_untracked().len();
            params_rc.set(Some(SimParams::for_count(n)));

            if let Some(h) = handle.get() {
                let _ = web_sys::window().map(|w| w.clear_interval_with_handle(h));
            }
            tick.set(50); // Start warm (constant temperature zone for interactive)

            let tick_c = tick.clone();
            let handle_c = handle.clone();
            let params_c = params_rc.clone();

            let cb = Closure::<dyn FnMut()>::new(move || {
                let t = tick_c.get();

                let node_pinned = dragged_node_id.get_untracked().is_some();

                // Stop when no longer dragging and settled.
                if !node_pinned {
                    // Run 30 more "settle" ticks after release, then stop.
                    if t > 280 {
                        if let Some(h) = handle_c.get() {
                            let _ = web_sys::window().map(|w| w.clear_interval_with_handle(h));
                            handle_c.set(None);
                            sim_handle_for_nodes.set(None);
                        }
                        return;
                    }
                }

                // Keep tick counter capped while dragging for constant temperature.
                if node_pinned && t > 50 {
                    tick_c.set(50);
                }

                let p = params_c.take();
                if p.is_none() { return; }
                let p = p.unwrap();

                let mut pos = positions.get_untracked();
                let el = edges.get_untracked();
                let pinned = dragged_node_id.get_untracked();
                let _still = force_tick(&mut pos, &el, &p, t, pinned.as_deref());

                positions.set(pos);
                tick_c.set(t + 1);
                params_c.set(Some(p));
            });

            if let Some(w) = web_sys::window() {
                let id = w
                    .set_interval_with_callback_and_timeout_and_arguments_0(
                        cb.as_ref().unchecked_ref(),
                        33,
                    )
                    .unwrap_or(-1);
                handle.set(Some(id));
                sim_handle_for_nodes.set(Some(id));
            }
            cb.forget();
        });
    }

    // ---- Fetch & build ----
    let sp = sim_params.clone();
    let ss = start_simulation.clone();
    Effect::new(move |_| {
        let vault = state.vault_path.get();
        if vault.is_empty() { return; }

        is_loading.set(true);
        graph_ready.set(false);
        error_msg.set(String::new());

        let sp = sp.clone();
        let ss = ss.clone();
        spawn_local(async move {
            let fetch_t0 = perf_now();
            match ipc::get_graph_snapshot().await {
                Ok(snapshot) => {
                    let fetch_ms = perf_now() - fetch_t0;
                    leptos::logging::log!("[perf] graph fetch: {:.1}ms", fetch_ms);
                    let total = snapshot.nodes.len();
                    let total_edges = snapshot.edges.len();

                    id_to_path.set(snapshot.id_to_path);

                    let mut connected = HashSet::new();
                    for (src, tgt) in &snapshot.edges {
                        connected.insert(src.clone());
                        connected.insert(tgt.clone());
                    }

                    let n = connected.len();
                    if n == 0 {
                        node_count_info.set(format!("{} notes — no links", total));
                        positions.set(HashMap::new());
                        edges.set(Vec::new());
                        is_loading.set(false);
                        return;
                    }

                    node_count_info.set(format!(
                        "{} linked / {} total — {} links",
                        n, total, total_edges
                    ));

                    let params = SimParams::for_count(n);

                    let mut deg: HashMap<String, usize> = HashMap::new();
                    for (src, tgt) in &snapshot.edges {
                        *deg.entry(src.clone()).or_insert(0) += 1;
                        *deg.entry(tgt.clone()).or_insert(0) += 1;
                    }

                    // Initial placement: small random perturbation on a circle.
                    // The randomness breaks symmetry and leads to more organic layouts.
                    let nodes: Vec<_> = connected.into_iter().collect();
                    let mut pm = HashMap::new();
                    let mut seed: u64 = 42;
                    for (i, nid) in nodes.iter().enumerate() {
                        let a = i as f64 * 2.0 * std::f64::consts::PI / n as f64;
                        // Simple LCG for deterministic pseudo-random jitter.
                        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                        let jitter = ((seed >> 33) as f64 / (1u64 << 31) as f64 - 0.5) * params.radius * 0.3;
                        let r = params.radius + jitter;
                        pm.insert(nid.clone(), NodePos {
                            x: r * a.cos(),
                            y: r * a.sin(),
                            vx: 0.0,
                            vy: 0.0,
                            degree: *deg.get(nid).unwrap_or(&0),
                        });
                    }

                    let fe: Vec<_> = snapshot.edges.into_iter()
                        .filter(|(s, t)| pm.contains_key(s) && pm.contains_key(t))
                        .collect();

                    sp.set(Some(params));
                    edges.set(fe);
                    positions.set(pm);
                    is_loading.set(false);
                    graph_ready.set(true);

                    ss();
                }
                Err(e) => {
                    error_msg.set(format!("Graph error: {}", e));
                    is_loading.set(false);
                }
            }
        });
    });

    // ── Helper closures for pan logic (shared by mouse + touch) ──

    let start_pan = move |x: f64, y: f64| {
        is_dragging.set(true);
        drag_moved.set(false);
        drag_start_x.set(x);
        drag_start_y.set(y);
        drag_last_x.set(x);
        drag_last_y.set(y);
    };

    let update_pan = move |cx: f64, cy: f64, el_w: f64, el_h: f64| {
        if !is_dragging.get_untracked() { return; }
        if !drag_moved.get_untracked() {
            let dx_s = (cx - drag_start_x.get_untracked()).abs();
            let dy_s = (cy - drag_start_y.get_untracked()).abs();
            if dx_s < DRAG_THRESHOLD && dy_s < DRAG_THRESHOLD { return; }
            drag_moved.set(true);
            auto_fit.set(false); // user took manual control
        }
        let ddx = cx - drag_last_x.get_untracked();
        let ddy = cy - drag_last_y.get_untracked();
        drag_last_x.set(cx);
        drag_last_y.set(cy);
        let z = zoom.get_untracked();
        let scale_x = (VB_W / z) / el_w;
        let scale_y = (VB_H / z) / el_h;
        pan_x.set(pan_x.get_untracked() - ddx * scale_x);
        pan_y.set(pan_y.get_untracked() - ddy * scale_y);
    };

    let end_pan = move || { is_dragging.set(false); };

    view! {
        <div class="forge-graph">
            {move || {
                if is_loading.get() {
                    return view! {
                        <div class="forge-graph__placeholder">
                            <p>"Loading graph..."</p>
                        </div>
                    }.into_any();
                }
                let err = error_msg.get();
                if !err.is_empty() {
                    return view! {
                        <div class="forge-graph__placeholder">
                            <p>{err}</p>
                        </div>
                    }.into_any();
                }
                if !graph_ready.get() {
                    let info = node_count_info.get();
                    let msg = if info.is_empty() {
                        "No linked notes found.".to_string()
                    } else {
                        info
                    };
                    return view! {
                        <div class="forge-graph__placeholder">
                            <p>{msg}</p>
                        </div>
                    }.into_any();
                }

                let state_c = state.clone();
                let info = node_count_info.get();

                view! {
                    <div class="forge-graph__wrapper"
                        style=move || {
                            if dragged_node_id.get().is_some() && drag_moved.get() {
                                "cursor:grabbing;touch-action:none;"
                            } else if is_dragging.get() && drag_moved.get() {
                                "cursor:grabbing;touch-action:none;"
                            } else {
                                "cursor:grab;touch-action:none;"
                            }
                        }

                        // ── Mouse events ──
                        on:mousedown=move |ev: web_sys::MouseEvent| {
                            if ev.button() != 0 { return; }
                            // Don't start camera pan if a node drag is starting
                            // (node mousedown fires first and stops propagation).
                            if dragged_node_id.get_untracked().is_some() { return; }
                            start_pan(ev.client_x() as f64, ev.client_y() as f64);
                        }
                        on:mousemove=move |ev: web_sys::MouseEvent| {
                            let cx = ev.client_x() as f64;
                            let cy = ev.client_y() as f64;

                            // If dragging a node, move it in graph space.
                            if let Some(ref nid) = dragged_node_id.get_untracked() {
                                if !drag_moved.get_untracked() {
                                    let dx_s = (cx - drag_start_x.get_untracked()).abs();
                                    let dy_s = (cy - drag_start_y.get_untracked()).abs();
                                    if dx_s < DRAG_THRESHOLD && dy_s < DRAG_THRESHOLD { return; }
                                    drag_moved.set(true);
                                }

                                // Convert screen coords to graph coords via the SVG bounding rect.
                                if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                                    if let Some(svg_el) = doc.get_element_by_id("forge-graph-svg") {
                                        let rect = svg_el.get_bounding_client_rect();
                                        let (gx, gy) = screen_to_graph(
                                            cx, cy,
                                            rect.left(), rect.top(),
                                            rect.width(), rect.height(),
                                            zoom.get_untracked(),
                                            pan_x.get_untracked(),
                                            pan_y.get_untracked(),
                                        );
                                        // Update the node's position directly.
                                        positions.update(|pos| {
                                            if let Some(node) = pos.get_mut(nid) {
                                                node.x = gx;
                                                node.y = gy;
                                                node.vx = 0.0;
                                                node.vy = 0.0;
                                            }
                                        });
                                    }
                                }
                                return;
                            }

                            // Otherwise: pan the camera.
                            let target = ev.current_target()
                                .and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok());
                            let (ew, eh) = match &target {
                                Some(el) => (el.client_width() as f64, el.client_height() as f64),
                                None => (1280.0, 720.0),
                            };
                            update_pan(cx, cy, ew, eh);
                        }
                        on:mouseup=move |_| {
                            // Release node drag if active.
                            if dragged_node_id.get_untracked().is_some() {
                                dragged_node_id.set(None);
                            }
                            end_pan();
                        }
                        on:mouseleave=move |_| {
                            if dragged_node_id.get_untracked().is_some() {
                                dragged_node_id.set(None);
                            }
                            end_pan();
                        }

                        // ── Wheel / trackpad ──
                        on:wheel=move |ev: web_sys::WheelEvent| {
                            ev.prevent_default();
                            auto_fit.set(false);
                            let delta = ev.delta_y();
                            let factor = if delta > 0.0 { 1.0 / 1.15 } else { 1.15 };
                            let new_z = (zoom.get_untracked() * factor).clamp(0.05, 8.0);
                            zoom.set(new_z);
                        }

                        // ── Touch events (mobile / tablet) ──
                        on:touchstart=move |ev: web_sys::TouchEvent| {
                            let touches = ev.touches();
                            if touches.length() == 1 {
                                // Single finger: pan.
                                if let Some(t) = touches.get(0) {
                                    start_pan(t.client_x() as f64, t.client_y() as f64);
                                }
                            } else if touches.length() >= 2 {
                                // Two fingers: pinch-to-zoom.
                                is_pinching.set(true);
                                is_dragging.set(false);
                                if let (Some(t0), Some(t1)) = (touches.get(0), touches.get(1)) {
                                    let dx = (t1.client_x() - t0.client_x()) as f64;
                                    let dy = (t1.client_y() - t0.client_y()) as f64;
                                    pinch_dist.set((dx * dx + dy * dy).sqrt());
                                }
                            }
                        }
                        on:touchmove=move |ev: web_sys::TouchEvent| {
                            ev.prevent_default();
                            let touches = ev.touches();
                            if is_pinching.get_untracked() && touches.length() >= 2 {
                                // Pinch zoom.
                                if let (Some(t0), Some(t1)) = (touches.get(0), touches.get(1)) {
                                    let dx = (t1.client_x() - t0.client_x()) as f64;
                                    let dy = (t1.client_y() - t0.client_y()) as f64;
                                    let new_dist = (dx * dx + dy * dy).sqrt();
                                    let old_dist = pinch_dist.get_untracked();
                                    if old_dist > 1.0 {
                                        let factor = new_dist / old_dist;
                                        let new_z = (zoom.get_untracked() * factor).clamp(0.05, 8.0);
                                        zoom.set(new_z);
                                        auto_fit.set(false);
                                    }
                                    pinch_dist.set(new_dist);
                                }
                            } else if touches.length() == 1 {
                                // Single finger pan.
                                if let Some(t) = touches.get(0) {
                                    let target = ev.current_target()
                                        .and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok());
                                    let (ew, eh) = match &target {
                                        Some(el) => (el.client_width() as f64, el.client_height() as f64),
                                        None => (800.0, 600.0),
                                    };
                                    update_pan(t.client_x() as f64, t.client_y() as f64, ew, eh);
                                }
                            }
                        }
                        on:touchend=move |ev: web_sys::TouchEvent| {
                            if ev.touches().length() == 0 {
                                end_pan();
                                is_pinching.set(false);
                            } else if ev.touches().length() < 2 {
                                is_pinching.set(false);
                                // Switch back to single-finger pan.
                                if let Some(t) = ev.touches().get(0) {
                                    start_pan(t.client_x() as f64, t.client_y() as f64);
                                }
                            }
                        }
                    >
                        <svg
                            id="forge-graph-svg"
                            viewBox="0 0 1000 750"
                            class="forge-graph__svg"
                            preserveAspectRatio="xMidYMid meet"
                        >
                            <defs>
                                <radialGradient id="nodeGrad" cx="35%" cy="35%" r="65%">
                                    <stop offset="0%" stop-color="#33D4F0"/>
                                    <stop offset="100%" stop-color="#0090AD"/>
                                </radialGradient>
                                <radialGradient id="hubGrad" cx="35%" cy="35%" r="65%">
                                    <stop offset="0%" stop-color="#D4845A"/>
                                    <stop offset="100%" stop-color="#B85C38"/>
                                </radialGradient>
                                <linearGradient id="edgeGrad" x1="0%" y1="0%" x2="100%" y2="0%">
                                    <stop offset="0%" stop-color="#00B4D8" stop-opacity="0.3"/>
                                    <stop offset="50%" stop-color="#1F3A52" stop-opacity="0.6"/>
                                    <stop offset="100%" stop-color="#00B4D8" stop-opacity="0.3"/>
                                </linearGradient>
                                <filter id="glow" x="-50%" y="-50%" width="200%" height="200%">
                                    <feGaussianBlur stdDeviation="3" result="blur"/>
                                    <feMerge>
                                        <feMergeNode in="blur"/>
                                        <feMergeNode in="SourceGraphic"/>
                                    </feMerge>
                                </filter>
                            </defs>
                            // Edges
                            {move || {
                                let el = edges.get();
                                let p = positions.get();
                                let z = zoom.get();
                                let sw = (1.2 / z).to_string();
                                el.iter().filter_map(|(s, t)| {
                                    let sp = p.get(s)?;
                                    let tp = p.get(t)?;
                                    let sw = sw.clone();
                                    Some(view! {
                                        <line
                                            x1={sp.x.to_string()} y1={sp.y.to_string()}
                                            x2={tp.x.to_string()} y2={tp.y.to_string()}
                                            stroke="url(#edgeGrad)"
                                            stroke-width=sw
                                            opacity="0.6"
                                        />
                                    })
                                }).collect_view()
                            }}
                            // Nodes
                            {move || {
                                let p = positions.get();
                                let z = zoom.get();
                                p.iter().map(|(id, n)| {
                                    let nid = id.clone();
                                    let sc = state_c.clone();
                                    let label = id_to_path.get_untracked()
                                        .get(id)
                                        .and_then(|p| {
                                            p.rsplit(['/', '\\']).next().map(|f| {
                                                f.strip_suffix(".md").unwrap_or(f).to_string()
                                            })
                                        })
                                        .unwrap_or_else(|| id.clone());
                                    let base_r = 6.0 + (n.degree as f64).sqrt() * 3.0;
                                    let r = base_r / z;
                                    let is_hub = n.degree >= 5;
                                    let fill = if is_hub { "url(#hubGrad)" } else { "url(#nodeGrad)" };
                                    let stroke_c = if is_hub { "#D4845A" } else { "#0090AD" };
                                    let filter = if is_hub { "url(#glow)" } else { "" };
                                    let sw = (1.0 / z).to_string();
                                    let nid_for_drag = id.clone();
                                    view! {
                                        <circle
                                            cx={n.x.to_string()}
                                            cy={n.y.to_string()}
                                            r={r.to_string()}
                                            fill=fill
                                            stroke=stroke_c
                                            stroke-width=sw
                                            filter=filter
                                            style="cursor:pointer;"
                                            on:mousedown=move |ev: web_sys::MouseEvent| {
                                                    if ev.button() != 0 { return; }
                                                    ev.stop_propagation();
                                                    // Start node drag — record the node ID and
                                                    // use the drag threshold to distinguish
                                                    // click (open note) from drag (move node).
                                                    dragged_node_id.set(Some(nid_for_drag.clone()));
                                                    drag_moved.set(false);
                                                    drag_start_x.set(ev.client_x() as f64);
                                                    drag_start_y.set(ev.client_y() as f64);
                                                    drag_last_x.set(ev.client_x() as f64);
                                                    drag_last_y.set(ev.client_y() as f64);
                                                    is_dragging.set(true);

                                                    // Restart simulation if it had stopped,
                                                    // so connected nodes follow the drag.
                                                    if sim_handle_for_nodes.get_untracked().is_none() {
                                                        restart_sim_trigger.update(|n| *n += 1);
                                                    }
                                                }
                                            on:click=move |ev: web_sys::MouseEvent| {
                                                // Only navigate if the user didn't drag.
                                                if drag_moved.get_untracked() { return; }
                                                ev.stop_propagation();
                                                let path = id_to_path.get_untracked()
                                                    .get(&nid)
                                                    .cloned()
                                                    .unwrap_or_else(|| nid.clone());
                                                let sc2 = sc.clone();
                                                let vault_root = sc2.vault_path.get_untracked();
                                                let rel_path = if path.starts_with(&vault_root) {
                                                    path[vault_root.len()..].trim_start_matches(|c: char| c == '/' || c == '\\').to_string()
                                                } else {
                                                    path.clone()
                                                };
                                                spawn_local(async move {
                                                    match ipc::get_note(&path).await {
                                                        Ok(content) => {
                                                            tab_mgr.open(&path, &rel_path, &content);
                                                            sc2.active_view.set(
                                                                crate::app::ActiveView::Editor,
                                                            );
                                                        }
                                                        Err(e) => {
                                                            leptos::logging::warn!(
                                                                "Graph: failed to load note: {}", e
                                                            );
                                                        }
                                                    }
                                                });
                                            }
                                        >
                                            <title>{label}</title>
                                        </circle>
                                    }
                                }).collect_view()
                            }}
                        </svg>
                    </div>
                    <div class="forge-graph__controls">
                        <span class="forge-graph__info">{info}</span>
                        <button class="forge-btn forge-btn--small"
                                on:click=move |_| {
                                    auto_fit.set(false);
                                    zoom.set((zoom.get() * 1.25).min(8.0));
                                }>
                            "+"
                        </button>
                        <button class="forge-btn forge-btn--small"
                                on:click=move |_| {
                                    auto_fit.set(false);
                                    zoom.set((zoom.get() / 1.25).max(0.05));
                                }>
                            "-"
                        </button>
                        <button class="forge-btn forge-btn--small"
                                on:click=move |_| {
                                    // Fit to content.
                                    let pos = positions.get_untracked();
                                    let (z, px, py) = fit_view(&pos);
                                    zoom.set(z);
                                    pan_x.set(px);
                                    pan_y.set(py);
                                }>
                            "Reset"
                        </button>
                    </div>
                }.into_any()
            }}
        </div>
    }
}

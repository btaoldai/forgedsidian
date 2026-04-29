//! Leptos GPU graph view component and reactive effects.
//!
//! Main UI component integrating wgpu renderer with Leptos signals,
//! event handlers, and animation frame loop.
//!
//! NOTE: this file is compiled to wasm32 only in practice (forge-ui CSR crate).
//! `#[allow(unused)]` silences false-positive warnings from `cargo check` on native targets.
#![allow(unused_imports, unused_variables, dead_code)]

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

use crate::app::{AppOptions, AppState};
use crate::ipc;
use crate::tab_manager::TabManager;
use forge_renderer::{GraphRenderer, Vec2};

use super::colors::{color_for_degree, COLOR_EDGE};
use super::events::{canvas_coords, canvas_coords_wheel, DRAG_THRESHOLD};
use super::gpu::{
    bounding_box, build_edge_instances, build_node_instances, hit_test_node, kick_raf,
};
use super::simulation::{force_tick, NodePos, SimParams, MAX_SIM_TICKS};

/// GPU-accelerated graph view component.
///
/// Drop-in replacement for the SVG `GraphView`. Renders into a `<canvas>`
/// element using wgpu instanced rendering.
///
/// Accepts reactive signals from `AppState` and `AppOptions` to control
/// graph visibility, physics parameters, and node/edge rendering properties.
#[component]
pub fn GpuGraphView() -> impl IntoView {
    let state = use_context::<AppState>().expect("AppState not found");
    let tab_mgr = use_context::<TabManager>().expect("TabManager not found");
    let opts = use_context::<AppOptions>().expect("AppOptions not found");

    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();
    let is_loading = RwSignal::new(true);
    let error_msg = RwSignal::new(String::new());
    let node_count_info = RwSignal::new(String::new());

    // Tooltip state: label text + screen position.
    let tooltip_text = RwSignal::new(String::new());
    let tooltip_x = RwSignal::new(0.0_f64);
    let tooltip_y = RwSignal::new(0.0_f64);
    let tooltip_visible = RwSignal::new(false);

    // Shared mutable state for the renderer (non-reactive, Rc<RefCell>).
    let renderer: Rc<RefCell<Option<GraphRenderer>>> = Rc::new(RefCell::new(None));
    let positions: Rc<RefCell<HashMap<String, NodePos>>> = Rc::new(RefCell::new(HashMap::new()));
    let edge_list: Rc<RefCell<Vec<(String, String)>>> = Rc::new(RefCell::new(Vec::new()));
    let id_to_path: Rc<RefCell<HashMap<String, String>>> = Rc::new(RefCell::new(HashMap::new()));
    let node_ids: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let sim_tick: Rc<Cell<u32>> = Rc::new(Cell::new(0));
    let sim_running: Rc<Cell<bool>> = Rc::new(Cell::new(false));
    let dragged_node: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    // Stored rAF closure — kept alive by Rc so we can restart the loop
    // from event handlers (e.g., node drag start).
    let raf_closure: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));

    // Pointer state for pan/zoom/drag.
    let is_panning = Rc::new(Cell::new(false));
    let last_pointer_x = Rc::new(Cell::new(0.0_f32));
    let last_pointer_y = Rc::new(Cell::new(0.0_f32));
    let drag_start_x = Rc::new(Cell::new(0.0_f32));
    let drag_start_y = Rc::new(Cell::new(0.0_f32));
    let drag_moved = Rc::new(Cell::new(false));
    // Node ID that was under the pointer at pointerdown (for click-to-open).
    let pointer_hit_node: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    // ── Initialize wgpu on canvas mount, then start animation loop ──
    {
        let renderer = renderer.clone();
        let positions = positions.clone();
        let edge_list = edge_list.clone();
        let id_to_path = id_to_path.clone();
        let node_ids_effect = node_ids.clone();
        let sim_tick = sim_tick.clone();
        let sim_running = sim_running.clone();
        let dragged_node = dragged_node.clone();
        let raf_closure = raf_closure.clone();

        Effect::new(move |_| {
            let vault = state.vault_path.get();
            if vault.is_empty() {
                return;
            }

            let _canvas_el = match canvas_ref.get() {
                Some(el) => el,
                None => return,
            };

            is_loading.set(true);
            error_msg.set(String::new());

            // GPU renderer init and animation loop use WASM-only APIs (wgpu canvas surface,
            // ResizeObserver, requestAnimationFrame). Gate the entire block so `cargo check`
            // on native targets does not fail — forge-ui is always compiled to wasm32.
            #[cfg(target_arch = "wasm32")]
            {
            let renderer = renderer.clone();
            let positions = positions.clone();
            let edge_list = edge_list.clone();
            let id_to_path = id_to_path.clone();
            let node_ids = node_ids_effect.clone();
            let sim_tick = sim_tick.clone();
            let sim_running = sim_running.clone();
            let dragged_node = dragged_node.clone();
            let raf_closure = raf_closure.clone();

            spawn_local(async move {
                // Fetch graph data.
                let snapshot = match ipc::get_graph_snapshot().await {
                    Ok(s) => s,
                    Err(e) => {
                        error_msg.set(format!("Graph error: {e}"));
                        is_loading.set(false);
                        return;
                    }
                };

                let total = snapshot.nodes.len();
                let total_edges = snapshot.edges.len();
                *id_to_path.borrow_mut() = snapshot.id_to_path;

                // Filter to connected nodes only.
                let mut connected = std::collections::HashSet::new();
                for (src, tgt) in &snapshot.edges {
                    connected.insert(src.clone());
                    connected.insert(tgt.clone());
                }

                let n = connected.len();
                if n == 0 {
                    node_count_info.set(format!("{total} notes -- no links"));
                    is_loading.set(false);
                    return;
                }

                node_count_info.set(format!("{n} linked / {total} total -- {total_edges} links"));

                // Compute degrees.
                let mut deg: HashMap<String, usize> = HashMap::new();
                for (src, tgt) in &snapshot.edges {
                    *deg.entry(src.clone()).or_insert(0) += 1;
                    *deg.entry(tgt.clone()).or_insert(0) += 1;
                }

                // Initial layout: circular with jitter.
                let params = SimParams::for_count(n);
                let nodes: Vec<_> = connected.into_iter().collect();
                let mut pm = HashMap::new();
                let mut seed: u64 = 42;
                for (i, nid) in nodes.iter().enumerate() {
                    let a = i as f32 * 2.0 * std::f32::consts::PI / n as f32;
                    seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                    let jitter = ((seed >> 33) as f32 / (1u64 << 31) as f32 - 0.5)
                        * params.radius
                        * 0.3;
                    let r = params.radius + jitter;
                    pm.insert(
                        nid.clone(),
                        NodePos {
                            x: r * a.cos(),
                            y: r * a.sin(),
                            vx: 0.0,
                            vy: 0.0,
                            degree: *deg.get(nid).unwrap_or(&0),
                        },
                    );
                }

                let fe: Vec<_> = snapshot
                    .edges
                    .into_iter()
                    .filter(|(s, t)| pm.contains_key(s) && pm.contains_key(t))
                    .collect();

                *positions.borrow_mut() = pm;
                *edge_list.borrow_mut() = fe;
                *node_ids.borrow_mut() = nodes;
                sim_tick.set(0);

                // Initialize wgpu renderer.
                let html_canvas: web_sys::HtmlCanvasElement = web_sys::window()
                    .and_then(|w| w.document())
                    .and_then(|d| d.get_element_by_id("forge-gpu-canvas"))
                    .expect("canvas element not in DOM")
                    .unchecked_into();

                let w = html_canvas.client_width().max(1) as u32;
                let h = html_canvas.client_height().max(1) as u32;
                html_canvas.set_width(w);
                html_canvas.set_height(h);

                match GraphRenderer::from_canvas(html_canvas.clone(), w, h).await {
                    Ok(r) => {
                        *renderer.borrow_mut() = Some(r);
                        is_loading.set(false);
                        sim_running.set(true);
                        leptos::logging::log!("[gpu] renderer initialized ({n} nodes, {w}x{h})");
                    }
                    Err(e) => {
                        error_msg.set(format!("GPU init failed: {e}"));
                        is_loading.set(false);
                        leptos::logging::warn!("[gpu] init error: {e}");
                        return;
                    }
                }

                // ── ResizeObserver: reconfigure surface on canvas resize ──
                {
                    let renderer = renderer.clone();
                    let resize_cb = Closure::<dyn FnMut(js_sys::Array)>::new(
                        move |entries: js_sys::Array| {
                            let entry: web_sys::ResizeObserverEntry =
                                entries.get(0).unchecked_into();
                            let rect = entry.content_rect();
                            let nw = (rect.width() as u32).max(1);
                            let nh = (rect.height() as u32).max(1);

                            // Guard: ignore degenerate sizes (canvas hidden during view switch).
                            if nw < 4 || nh < 4 {
                                leptos::logging::log!(
                                    "[gpu] resize ignored ({nw}x{nh} too small)"
                                );
                                return;
                            }

                            // Update the canvas backing store to match CSS size.
                            if let Some(canvas) =
                                entry.target().dyn_ref::<web_sys::HtmlCanvasElement>()
                            {
                                canvas.set_width(nw);
                                canvas.set_height(nh);
                            }

                            if let Some(ref mut r) = *renderer.borrow_mut() {
                                r.resize(nw, nh);
                                let _ = r.render();
                            }
                            leptos::logging::log!("[gpu] resized to {nw}x{nh}");
                        },
                    );

                    if let Ok(observer) =
                        web_sys::ResizeObserver::new(resize_cb.as_ref().unchecked_ref())
                    {
                        observer.observe(&html_canvas);
                        // Leak the closure and observer so they live as long as the page.
                        resize_cb.forget();
                        std::mem::forget(observer);
                    }
                }

                // ── Start animation loop (rAF) ──
                // raf_closure is the shared Rc that holds the Closure.
                // The closure captures a clone (self_ref) for the self-referential
                // request_animation_frame call. Event handlers can restart the
                // loop via kick_raf(&raf_closure) after it has stopped.
                let self_ref = raf_closure.clone();

                *raf_closure.borrow_mut() = Some(Closure::new(move || {
                    let t = sim_tick.get();
                    let node_pinned = dragged_node.borrow().is_some();

                    // Read user settings (reactive signals, cheap read).
                    let size_mult = opts.graph_node_size.get_untracked();
                    let repulsion_mult = opts.graph_repulsion.get_untracked();
                    let attraction_mult = opts.graph_attraction.get_untracked();
                    let edge_thick_mult = opts.graph_edge_thickness.get_untracked();
                    let cam_smooth = opts.graph_camera_smoothing.get_untracked();

                    if t < MAX_SIM_TICKS || node_pinned {
                        let mut pos = positions.borrow_mut();
                        let el = edge_list.borrow();
                        let pinned = dragged_node.borrow();
                        let mut params = SimParams::for_count(pos.len());
                        // Apply user multipliers.
                        params.repulsion *= repulsion_mult;
                        params.attraction *= attraction_mult;
                        let current_tick = if node_pinned && t > 50 { 50 } else { t };
                        let settled =
                            force_tick(&mut pos, &el, &params, current_tick, pinned.as_deref());
                        drop(pinned);
                        drop(el);

                        // Build GPU data with user-controlled sizes.
                        let node_instances = build_node_instances(&pos, size_mult);
                        let edge_instances = build_edge_instances(
                            &pos,
                            &edge_list.borrow(),
                            edge_thick_mult,
                        );
                        drop(pos);

                        // Upload to GPU and render.
                        if let Some(ref mut r) = *renderer.borrow_mut() {
                            r.set_nodes(&node_instances);
                            r.set_edges(&edge_instances);

                            // Smooth camera tracking throughout the entire simulation.
                            let (min, max) = bounding_box(&node_instances);
                            r.camera_mut().smooth_fit_to_bounds(min, max, 80.0, cam_smooth);

                            if let Err(e) = r.render() {
                                leptos::logging::warn!("[gpu] render error: {e:?}");
                            }
                        }

                        sim_tick.set(t + 1);

                        if settled && !node_pinned && t > 30 {
                            sim_running.set(false);
                            leptos::logging::log!("[gpu] simulation settled at tick {t}");
                            return; // stop rAF (closure stays alive in raf_closure)
                        }
                    } else {
                        sim_running.set(false);
                        leptos::logging::log!("[gpu] simulation completed (max ticks)");
                        return; // stop rAF
                    }

                    // Request next frame (self-reference via the shared Rc).
                    if let Some(win) = web_sys::window() {
                        if let Some(ref cb) = *self_ref.borrow() {
                            let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
                        }
                    }
                }));

                // Kick off the rAF loop.
                kick_raf(&raf_closure);
            });
            } // end #[cfg(target_arch = "wasm32")]
        });
    }

    // ── Reactive Effect: re-render when graph VISUAL settings change ──
    //
    // Subscribes to node size, edge thickness, camera smoothing.
    // On change: re-build GPU instances and render one frame (no sim restart).
    {
        let renderer = renderer.clone();
        let positions = positions.clone();
        let edge_list = edge_list.clone();

        Effect::new(move |_| {
            // Subscribe to visual-only settings (reactive .get()).
            let size_mult = opts.graph_node_size.get();
            let edge_thick_mult = opts.graph_edge_thickness.get();
            let _cam_smooth = opts.graph_camera_smoothing.get();

            // Don't run before renderer is ready.
            if renderer.borrow().is_none() {
                return;
            }

            let pos = positions.borrow();
            if pos.is_empty() {
                return;
            }

            let node_instances = build_node_instances(&pos, size_mult);
            let edge_instances = build_edge_instances(&pos, &edge_list.borrow(), edge_thick_mult);
            drop(pos);

            if let Some(ref mut r) = *renderer.borrow_mut() {
                r.set_nodes(&node_instances);
                r.set_edges(&edge_instances);

                // Refit camera so larger/smaller nodes are actually visible.
                let (min, max) = bounding_box(&node_instances);
                r.camera_mut().fit_to_bounds(min, max, 80.0);

                let _ = r.render();
            }
        });
    }

    // ── Reactive Effect: restart simulation when PHYSICS settings change ──
    //
    // Subscribes to repulsion and attraction only.
    // On change: restart the force-directed layout so the graph re-flows.
    {
        let sim_tick = sim_tick.clone();
        let sim_running = sim_running.clone();
        let raf_closure = raf_closure.clone();

        // Track previous values to skip the initial fire.
        let prev_repulsion = Rc::new(Cell::new(opts.graph_repulsion.get_untracked()));
        let prev_attraction = Rc::new(Cell::new(opts.graph_attraction.get_untracked()));

        Effect::new(move |_| {
            // Subscribe to physics settings (reactive .get()).
            let rep = opts.graph_repulsion.get();
            let att = opts.graph_attraction.get();

            // Only restart if the value actually changed (skip initial fire).
            let rep_changed = (rep - prev_repulsion.get()).abs() > 0.001;
            let att_changed = (att - prev_attraction.get()).abs() > 0.001;
            prev_repulsion.set(rep);
            prev_attraction.set(att);

            if !rep_changed && !att_changed {
                return;
            }

            // Restart sim — sim_running is Rc<Cell<bool>>, not a Leptos signal,
            // so .get() is plain Cell::get (no reactive subscription).
            if !sim_running.get() {
                sim_running.set(true);
                sim_tick.set(sim_tick.get().min(50));
                kick_raf(&raf_closure);
            }
        });
    }

    // ── Pointer event handlers ──

    let on_pointerdown = {
        let is_panning = is_panning.clone();
        let last_pointer_x = last_pointer_x.clone();
        let last_pointer_y = last_pointer_y.clone();
        let drag_start_x = drag_start_x.clone();
        let drag_start_y = drag_start_y.clone();
        let drag_moved = drag_moved.clone();
        let pointer_hit_node = pointer_hit_node.clone();
        let dragged_node = dragged_node.clone();
        let renderer = renderer.clone();
        let positions = positions.clone();
        let sim_running = sim_running.clone();
        let sim_tick = sim_tick.clone();
        let raf_closure = raf_closure.clone();
        move |ev: web_sys::PointerEvent| {
            if ev.button() != 0 {
                return;
            }

            let (cx, cy) = canvas_coords(&ev);
            let x = ev.client_x() as f32;
            let y = ev.client_y() as f32;
            drag_moved.set(false);
            drag_start_x.set(x);
            drag_start_y.set(y);
            last_pointer_x.set(x);
            last_pointer_y.set(y);

            // CPU hit-test: find node under cursor.
            let size_mult = opts.graph_node_size.get_untracked();
            let hit = {
                let r_borrow = renderer.borrow();
                if let Some(ref r) = *r_borrow {
                    let graph_pt = r.camera.screen_to_graph(Vec2::new(cx, cy));
                    hit_test_node(&positions.borrow(), graph_pt, size_mult)
                } else {
                    None
                }
            };

            if let Some(ref nid) = hit {
                // Start node drag.
                *dragged_node.borrow_mut() = Some(nid.clone());
                *pointer_hit_node.borrow_mut() = Some(nid.clone());

                // Restart simulation if it had stopped.
                if !sim_running.get() {
                    sim_running.set(true);
                    sim_tick.set(sim_tick.get().min(50));
                    // Restart the rAF loop.
                    kick_raf(&raf_closure);
                }
            } else {
                // No node hit: start camera pan.
                *pointer_hit_node.borrow_mut() = None;
                is_panning.set(true);
            }
        }
    };

    let on_pointermove = {
        let is_panning = is_panning.clone();
        let last_pointer_x = last_pointer_x.clone();
        let last_pointer_y = last_pointer_y.clone();
        let drag_start_x = drag_start_x.clone();
        let drag_start_y = drag_start_y.clone();
        let drag_moved = drag_moved.clone();
        let renderer = renderer.clone();
        let dragged_node = dragged_node.clone();
        let positions = positions.clone();
        let id_to_path = id_to_path.clone();
        move |ev: web_sys::PointerEvent| {
            let x = ev.client_x() as f32;
            let y = ev.client_y() as f32;

            // Detect drag threshold.
            let dx = x - drag_start_x.get();
            let dy = y - drag_start_y.get();
            if (dx * dx + dy * dy).sqrt() > DRAG_THRESHOLD {
                drag_moved.set(true);
            }

            let delta_x = x - last_pointer_x.get();
            let delta_y = y - last_pointer_y.get();
            last_pointer_x.set(x);
            last_pointer_y.set(y);

            // Node drag: update node position in graph space.
            if let Some(ref nid) = *dragged_node.borrow() {
                tooltip_visible.set(false);
                let (cx, cy) = canvas_coords(&ev);
                if let Some(ref r) = *renderer.borrow() {
                    let graph_pt = r.camera.screen_to_graph(Vec2::new(cx, cy));
                    if let Some(np) = positions.borrow_mut().get_mut(nid) {
                        np.x = graph_pt.x;
                        np.y = graph_pt.y;
                        np.vx = 0.0;
                        np.vy = 0.0;
                    }
                }
                return;
            }

            // Camera pan.
            if is_panning.get() {
                tooltip_visible.set(false);
                if let Some(ref mut r) = *renderer.borrow_mut() {
                    r.camera_mut().pan(Vec2::new(delta_x, delta_y));
                    let _ = r.render();
                }
                return;
            }

            // Hover hit-test: show tooltip when hovering a node.
            let (cx, cy) = canvas_coords(&ev);
            let size_mult = opts.graph_node_size.get_untracked();
            let hit = {
                let r_borrow = renderer.borrow();
                if let Some(ref r) = *r_borrow {
                    let graph_pt = r.camera.screen_to_graph(Vec2::new(cx, cy));
                    hit_test_node(&positions.borrow(), graph_pt, size_mult)
                } else {
                    None
                }
            };

            if let Some(ref nid) = hit {
                // Extract the note name from the path.
                let label = id_to_path
                    .borrow()
                    .get(nid)
                    .and_then(|p| p.rsplit(['/', '\\']).next().map(String::from))
                    .unwrap_or_else(|| nid.clone());
                tooltip_text.set(label);
                tooltip_x.set(ev.client_x() as f64 + 12.0);
                tooltip_y.set(ev.client_y() as f64 - 8.0);
                tooltip_visible.set(true);
            } else {
                tooltip_visible.set(false);
            }
        }
    };

    let on_pointerup = {
        let is_panning = is_panning.clone();
        let drag_moved = drag_moved.clone();
        let dragged_node = dragged_node.clone();
        let pointer_hit_node = pointer_hit_node.clone();
        let id_to_path = id_to_path.clone();
        let state_clone = state.clone();
        let tab_mgr = tab_mgr.clone();
        move |_ev: web_sys::PointerEvent| {
            is_panning.set(false);

            // Release node drag.
            *dragged_node.borrow_mut() = None;

            // Click-to-open: if we hit a node and didn't drag, open the note.
            if !drag_moved.get() {
                if let Some(nid) = pointer_hit_node.borrow_mut().take() {
                    let path = id_to_path
                        .borrow()
                        .get(&nid)
                        .cloned()
                        .unwrap_or_else(|| nid.clone());
                    let vault_root = state_clone.vault_path.get_untracked();
                    let rel_path = if path.starts_with(&vault_root) {
                        path[vault_root.len()..]
                            .trim_start_matches(|c: char| c == '/' || c == '\\')
                            .to_string()
                    } else {
                        path.clone()
                    };
                    let sc = state_clone.clone();
                    let tm = tab_mgr.clone();
                    spawn_local(async move {
                        match ipc::get_note(&path).await {
                            Ok(content) => {
                                tm.open(&path, &rel_path, &content);
                                sc.active_view.set(crate::app::ActiveView::Editor);
                            }
                            Err(e) => {
                                leptos::logging::warn!("[gpu] failed to load note: {e}");
                            }
                        }
                    });
                }
            }

            *pointer_hit_node.borrow_mut() = None;
        }
    };

    let on_wheel = {
        let renderer = renderer.clone();
        move |ev: web_sys::WheelEvent| {
            ev.prevent_default();
            let delta = -ev.delta_y() as f32 * 0.001;
            let (cx, cy) = canvas_coords_wheel(&ev);
            if let Some(ref mut r) = *renderer.borrow_mut() {
                r.camera_mut().zoom_at(delta, Vec2::new(cx, cy));
                let _ = r.render();
            }
        }
    };

    // ── Control button callbacks ──

    // Fit the camera to show all nodes with padding.
    let on_fit_view = {
        let renderer = renderer.clone();
        let positions = positions.clone();
        move |_: web_sys::MouseEvent| {
            let pos = positions.borrow();
            if pos.is_empty() {
                return;
            }
            let size_mult = opts.graph_node_size.get_untracked();
            let node_instances = build_node_instances(&pos, size_mult);
            let (min, max) = bounding_box(&node_instances);
            drop(pos);
            if let Some(ref mut r) = *renderer.borrow_mut() {
                r.camera_mut().fit_to_bounds(min, max, 80.0);
                let _ = r.render();
            }
        }
    };

    // Zoom in by a fixed step, centered on the viewport.
    let on_zoom_in = {
        let renderer = renderer.clone();
        move |_: web_sys::MouseEvent| {
            if let Some(ref mut r) = *renderer.borrow_mut() {
                let vw = r.camera.viewport_width;
                let vh = r.camera.viewport_height;
                r.camera_mut().zoom_at(0.15, Vec2::new(vw / 2.0, vh / 2.0));
                let _ = r.render();
            }
        }
    };

    // Zoom out by a fixed step, centered on the viewport.
    let on_zoom_out = {
        let renderer = renderer.clone();
        move |_: web_sys::MouseEvent| {
            if let Some(ref mut r) = *renderer.borrow_mut() {
                let vw = r.camera.viewport_width;
                let vh = r.camera.viewport_height;
                r.camera_mut().zoom_at(-0.15, Vec2::new(vw / 2.0, vh / 2.0));
                let _ = r.render();
            }
        }
    };

    // Restart the force-directed simulation from tick 0.
    let on_reset_layout = {
        let sim_tick = sim_tick.clone();
        let sim_running = sim_running.clone();
        let raf_closure = raf_closure.clone();
        let positions = positions.clone();
        move |_: web_sys::MouseEvent| {
            // Reset velocities.
            for np in positions.borrow_mut().values_mut() {
                np.vx = 0.0;
                np.vy = 0.0;
            }
            sim_tick.set(0);
            if !sim_running.get() {
                sim_running.set(true);
                kick_raf(&raf_closure);
            }
        }
    };

    // ── View ──

    // Button style shared by graph controls.
    let btn_style = "padding:4px 8px;font-size:14px;cursor:pointer;background:var(--trl-abyss);color:var(--trl-text);border:1px solid var(--trl-abyss-light);border-radius:4px;line-height:1;";

    view! {
        <div class="forge-graph" style="position:relative;width:100%;height:100%;">
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

                view! {
                    <div style="display:none;"></div>
                }.into_any()
            }}
            <canvas
                id="forge-gpu-canvas"
                node_ref=canvas_ref
                style="width:100%;height:100%;display:block;touch-action:none;cursor:grab;"
                on:pointerdown=on_pointerdown
                on:pointermove=on_pointermove
                on:pointerup=on_pointerup
                on:wheel=on_wheel
            />
            // Graph control buttons (top-right overlay)
            <div style="position:absolute;top:8px;right:8px;display:flex;gap:4px;z-index:10;">
                <button style=btn_style on:click=on_fit_view title="Fit to view">{"\u{2922}"}</button>
                <button style=btn_style on:click=on_zoom_in title="Zoom in">"+"</button>
                <button style=btn_style on:click=on_zoom_out title="Zoom out">"-"</button>
                <button style=btn_style on:click=on_reset_layout title="Reset layout">{"\u{21BB}"}</button>
            </div>
            <div class="forge-graph__controls" style="position:absolute;bottom:8px;left:8px;right:8px;display:flex;justify-content:space-between;align-items:center;">
                <span class="forge-graph__info">{move || node_count_info.get()}</span>
            </div>
            // Tooltip overlay (follows cursor on node hover).
            <div
                style=move || {
                    if tooltip_visible.get() {
                        format!(
                            "position:fixed;left:{}px;top:{}px;background:var(--trl-abyss);color:var(--trl-text);\
                             padding:4px 8px;border-radius:4px;font-size:12px;pointer-events:none;\
                             z-index:100;border:1px solid var(--trl-abyss-light);white-space:nowrap;",
                            tooltip_x.get(), tooltip_y.get()
                        )
                    } else {
                        "display:none;".to_string()
                    }
                }
            >
                {move || tooltip_text.get()}
            </div>
        </div>
    }
}

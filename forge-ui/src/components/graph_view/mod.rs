//! GPU-accelerated graph view using wgpu instanced rendering.
//!
//! Renders the vault note-graph into a `<canvas>` element via wgpu.
//! Scales to 1000+ nodes thanks to instanced SDF circles and oriented-quad edges.
//!
//! # Architecture
//!
//! Leptos `<canvas>` element -> wgpu Surface -> instanced draw calls.
//! Pointer events (unified mouse/touch/stylus) control zoom, pan, and node drag.
//! Physics simulation (Fruchterman-Reingold) runs in a requestAnimationFrame loop
//! and uploads positions to the GPU each frame.
//! Settings (node size, repulsion, etc.) are reactive via `AppOptions` signals.
//!
//! # Module Organization
//!
//! - `simulation`: Force-directed layout physics (NodePos, SimParams, force_tick).
//! - `events`: Pointer and control event handlers (canvas_coords, event closures).
//! - `colors`: Node color palette and mapping (COLOR_NODE, COLOR_HUB, color_for_degree).
//! - `gpu`: GPU instance builders and hit-tests (build_node_instances, build_edge_instances, hit_test_node, etc.).
//! - `view`: Leptos component GpuGraphView + reactive effects.

mod colors;
mod events;
mod gpu;
mod simulation;
mod view;

// Re-export public types and the main component.
pub use colors::{color_for_degree, COLOR_EDGE, COLOR_HUB, COLOR_NODE, HUB_THRESHOLD};
pub use events::{canvas_coords, canvas_coords_wheel, DRAG_THRESHOLD};
pub use gpu::{bounding_box, build_edge_instances, build_node_instances, hit_test_node, kick_raf};
pub use simulation::{force_tick, NodePos, SimParams, MAX_SIM_TICKS};
pub use view::GpuGraphView;

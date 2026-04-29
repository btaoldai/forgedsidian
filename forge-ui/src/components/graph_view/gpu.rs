//! GPU-accelerated rendering helpers.
//!
//! Low-level utilities for building GPU instances from simulation state,
//! performing CPU hit-tests, and managing the animation frame loop.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use forge_renderer::{EdgeInstance, NodeInstance, Vec2};

use super::colors::color_for_degree;
use super::simulation::NodePos;

// ───────────────────────── Constants ─────────────────────────

/// Base node radius in graph-space units (before multiplier).
const NODE_RADIUS_BASE: f32 = 40.0;
/// Extra radius per sqrt(degree).
const NODE_RADIUS_DEGREE_SCALE: f32 = 15.0;
/// Base edge thickness in graph-space units (before multiplier).
/// Scaled up so edges remain visible relative to node radii (~100 units).
const EDGE_THICKNESS: f32 = 16.0;

// ───────────────────────── Helpers ─────────────────────────

/// Compute display radius for a node given its degree and the user's size multiplier.
fn node_radius(degree: usize, size_mult: f32) -> f32 {
    (NODE_RADIUS_BASE + (degree as f32).sqrt() * NODE_RADIUS_DEGREE_SCALE) * size_mult
}

/// Build NodeInstance array from current positions.
///
/// Creates GPU instances for all nodes in the graph, assigning colors based on degree
/// and computing display radius from node properties and user multipliers.
pub fn build_node_instances(
    positions: &HashMap<String, NodePos>,
    size_mult: f32,
) -> Vec<NodeInstance> {
    let mut instances = Vec::with_capacity(positions.len());
    let mut pick_id: u32 = 0;
    for (_id, np) in positions.iter() {
        instances.push(NodeInstance {
            position: [np.x, np.y],
            radius: node_radius(np.degree, size_mult),
            color: color_for_degree(np.degree),
            pick_id,
        });
        pick_id += 1;
    }
    instances
}

/// Build EdgeInstance array from current positions and edge list.
///
/// Creates GPU instances for all edges, using node positions and the provided thickness multiplier.
pub fn build_edge_instances(
    positions: &HashMap<String, NodePos>,
    edges: &[(String, String)],
    thickness_mult: f32,
) -> Vec<EdgeInstance> {
    let mut instances = Vec::with_capacity(edges.len());
    for (src, tgt) in edges {
        if let (Some(a), Some(b)) = (positions.get(src), positions.get(tgt)) {
            instances.push(EdgeInstance {
                start: [a.x, a.y],
                end: [b.x, b.y],
                thickness: EDGE_THICKNESS * thickness_mult,
                color: [1.0, 1.0, 1.0, 0.2], // COLOR_EDGE
                _pad: [0.0],
            });
        }
    }
    instances
}

/// Restart the rAF animation loop by requesting a frame with the stored closure.
pub fn kick_raf(raf_closure: &Rc<RefCell<Option<Closure<dyn FnMut()>>>>) {
    if let Some(ref cb) = *raf_closure.borrow() {
        if let Some(win) = web_sys::window() {
            let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
        }
    }
}

/// CPU hit-test: find the node under a graph-space point.
///
/// Returns the node ID if the point is within the node's visual radius.
/// Checks all nodes (O(n)); fine for <10k nodes at 60fps pointer events.
///
/// # Arguments
///
/// * `positions` - Current node positions.
/// * `graph_pt` - Point in graph space (already converted from screen).
/// * `size_mult` - User size multiplier for node radius calculation.
///
/// # Returns
///
/// The closest node ID under the point (if any), or None.
pub fn hit_test_node(
    positions: &HashMap<String, NodePos>,
    graph_pt: Vec2,
    size_mult: f32,
) -> Option<String> {
    let mut best: Option<(String, f32)> = None;
    for (id, np) in positions.iter() {
        let dx = graph_pt.x - np.x;
        let dy = graph_pt.y - np.y;
        let dist_sq = dx * dx + dy * dy;
        let r = node_radius(np.degree, size_mult);
        if dist_sq <= r * r {
            let dist = dist_sq.sqrt();
            if best.as_ref().map_or(true, |(_, bd)| dist < *bd) {
                best = Some((id.clone(), dist));
            }
        }
    }
    best.map(|(id, _)| id)
}

/// Compute bounding box of node instances.
///
/// Returns the axis-aligned bounding box (min, max) that contains all nodes
/// including their radii. Returns (ZERO, ZERO) if the node list is empty.
pub fn bounding_box(nodes: &[NodeInstance]) -> (Vec2, Vec2) {
    if nodes.is_empty() {
        return (Vec2::ZERO, Vec2::ZERO);
    }
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    for n in nodes {
        min_x = min_x.min(n.position[0] - n.radius);
        min_y = min_y.min(n.position[1] - n.radius);
        max_x = max_x.max(n.position[0] + n.radius);
        max_y = max_y.max(n.position[1] + n.radius);
    }
    (Vec2::new(min_x, min_y), Vec2::new(max_x, max_y))
}

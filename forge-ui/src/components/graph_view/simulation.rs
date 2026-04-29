//! Force-directed layout simulation (Fruchterman-Reingold).
//!
//! Manages per-node position and velocity state, simulation parameters tuned to node count,
//! and the core physics tick that computes repulsive/attractive forces and integrates motion.
//!
//! The simulation runs in a requestAnimationFrame loop and updates node positions
//! every frame, which are then uploaded to the GPU renderer.

use std::collections::HashMap;

/// Maximum number of simulation ticks before the layout is considered settled.
pub const MAX_SIM_TICKS: u32 = 250;

/// Per-node mutable state for the force-directed simulation.
///
/// Tracks 2D position, 2D velocity, and the node's degree (for scaling forces and visual size).
#[derive(Clone, Debug)]
pub struct NodePos {
    /// X-coordinate in graph space.
    pub x: f32,
    /// Y-coordinate in graph space.
    pub y: f32,
    /// X-component of velocity.
    pub vx: f32,
    /// Y-component of velocity.
    pub vy: f32,
    /// Number of edges connected to this node.
    pub degree: usize,
}

/// Force-directed layout parameters (Fruchterman-Reingold algorithm).
///
/// Parameters are tuned based on node count to balance attractive/repulsive forces
/// appropriately for dense or sparse graphs.
pub struct SimParams {
    /// Ideal spring length; larger for sparse graphs.
    pub k: f32,
    /// Velocity damping (0..1); lower = faster dissipation.
    pub damping: f32,
    /// Initial temperature; controls motion amplitude at tick 0.
    pub initial_temp: f32,
    /// Repulsion multiplier; higher = nodes push apart more.
    pub repulsion: f32,
    /// Attraction multiplier; higher = connected nodes pull together more.
    pub attraction: f32,
    /// Circular layout radius for initial placement.
    pub radius: f32,
}

impl SimParams {
    /// Create simulation parameters tuned to the node count.
    ///
    /// Small graphs (<=50 nodes) are dense; medium/large graphs need more space.
    /// Parameters scale to prevent nodes from clumping or flying apart.
    ///
    /// # Arguments
    ///
    /// * `n` - Number of nodes in the graph.
    ///
    /// # Returns
    ///
    /// Tuned `SimParams` for the given node count.
    pub fn for_count(n: usize) -> Self {
        if n <= 50 {
            Self {
                k: 90.0,
                damping: 0.90,
                initial_temp: 1.0,
                repulsion: 1.2,
                attraction: 0.06,
                radius: 180.0,
            }
        } else if n <= 200 {
            Self {
                k: 110.0,
                damping: 0.88,
                initial_temp: 1.0,
                repulsion: 1.5,
                attraction: 0.04,
                radius: 250.0,
            }
        } else {
            Self {
                k: 140.0,
                damping: 0.85,
                initial_temp: 1.0,
                repulsion: 2.0,
                attraction: 0.03,
                radius: 350.0,
            }
        }
    }
}

/// One tick of the Fruchterman-Reingold force-directed layout.
///
/// Computes repulsive forces between all node pairs, attractive forces along edges,
/// applies gravity toward the origin, and integrates velocities into new positions.
///
/// Returns `true` when the simulation has settled (maximum displacement < 0.5).
///
/// # Arguments
///
/// * `positions` - Mutable map of node ID → `NodePos`; updated in-place.
/// * `edges` - List of (source, target) node ID pairs defining the graph topology.
/// * `p` - Simulation parameters tuned to the graph size.
/// * `tick` - Current simulation iteration (0..MAX_SIM_TICKS); used for temperature scheduling.
/// * `pinned_node` - If Some, this node ID is held in place by external drag; it still exerts forces.
///
/// # Returns
///
/// `true` if the largest node displacement was < 0.5 (settled); `false` if still moving.
pub fn force_tick(
    positions: &mut HashMap<String, NodePos>,
    edges: &[(String, String)],
    p: &SimParams,
    tick: u32,
    pinned_node: Option<&str>,
) -> bool {
    let temp = p.initial_temp * (1.0 - tick as f32 / MAX_SIM_TICKS as f32).max(0.05);
    let use_temp = if pinned_node.is_some() { 0.3_f32 } else { temp };

    let ids: Vec<String> = positions.keys().cloned().collect();

    // Repulsive forces (all pairs).
    let mut forces: HashMap<String, (f32, f32)> = ids
        .iter()
        .map(|id| (id.clone(), (0.0, 0.0)))
        .collect();

    for i in 0..ids.len() {
        for j in (i + 1)..ids.len() {
            let a = &positions[&ids[i]];
            let b = &positions[&ids[j]];
            let dx = a.x - b.x;
            let dy = a.y - b.y;
            let dist = (dx * dx + dy * dy).sqrt().max(1.0);
            let force = p.repulsion * p.k * p.k / dist;
            let fx = dx / dist * force;
            let fy = dy / dist * force;
            forces.get_mut(&ids[i]).unwrap().0 += fx;
            forces.get_mut(&ids[i]).unwrap().1 += fy;
            forces.get_mut(&ids[j]).unwrap().0 -= fx;
            forces.get_mut(&ids[j]).unwrap().1 -= fy;
        }
    }

    // Attractive forces (edges).
    for (src, tgt) in edges {
        if let (Some(a), Some(b)) = (positions.get(src), positions.get(tgt)) {
            let dx = a.x - b.x;
            let dy = a.y - b.y;
            let dist = (dx * dx + dy * dy).sqrt().max(1.0);
            let force = p.attraction * dist / p.k;
            let fx = dx / dist * force;
            let fy = dy / dist * force;
            if let Some(f) = forces.get_mut(src) {
                f.0 -= fx;
                f.1 -= fy;
            }
            if let Some(f) = forces.get_mut(tgt) {
                f.0 += fx;
                f.1 += fy;
            }
        }
    }

    // Gravity toward center.
    let gravity = 0.01_f32;
    for id in &ids {
        if let Some(n) = positions.get(id) {
            let dist = (n.x * n.x + n.y * n.y).sqrt().max(1.0);
            if let Some(f) = forces.get_mut(id) {
                f.0 -= gravity * n.x / dist * p.k;
                f.1 -= gravity * n.y / dist * p.k;
            }
        }
    }

    // Integration.
    let mut max_disp: f32 = 0.0;
    for id in &ids {
        let is_pinned = pinned_node.map_or(false, |pid| pid == id);
        if is_pinned {
            continue;
        }

        let (fx, fy) = forces[id];
        if let Some(n) = positions.get_mut(id) {
            n.vx = (n.vx + fx) * p.damping;
            n.vy = (n.vy + fy) * p.damping;
            let disp = (n.vx * n.vx + n.vy * n.vy).sqrt();
            let limited = disp.min(use_temp * p.k);
            if disp > 0.01 {
                n.x += n.vx / disp * limited;
                n.y += n.vy / disp * limited;
            }
            max_disp = max_disp.max(limited);
        }
    }

    max_disp < 0.5
}

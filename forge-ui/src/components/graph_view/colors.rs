//! Node color palette and mapping logic for the GPU-accelerated graph view.
//!
//! Defines the TheRustLab color scheme for nodes, hubs, and edges,
//! and provides utilities for node coloring based on graph properties (e.g., degree).

/// TheRustLab color palette (f32 RGBA for GPU).
/// Cyan: #0090AD -- trl-cyan standard color for regular nodes.
pub const COLOR_NODE: [f32; 4] = [0.0, 0.565, 0.678, 1.0];

/// TheRustLab accent/alert color for hub nodes (degree >= HUB_THRESHOLD).
/// Orange: #D4845A -- trl-alert for high-connectivity nodes.
pub const COLOR_HUB: [f32; 4] = [0.831, 0.518, 0.353, 1.0];

/// Edge color: cyan with 55% alpha for visibility without overwhelming the canvas.
pub const COLOR_EDGE: [f32; 4] = [0.0, 0.565, 0.678, 0.55];

/// Degree threshold to switch from cyan to hub (alert) color.
/// Nodes with degree >= 5 are considered "hubs" and rendered in alert color.
pub const HUB_THRESHOLD: usize = 5;

/// Select node color based on its degree.
///
/// Returns `COLOR_HUB` if degree >= `HUB_THRESHOLD`, else `COLOR_NODE`.
///
/// # Arguments
///
/// * `degree` - The number of edges connected to the node.
///
/// # Returns
///
/// An RGBA color array for GPU rendering.
pub fn color_for_degree(degree: usize) -> [f32; 4] {
    if degree >= HUB_THRESHOLD {
        COLOR_HUB
    } else {
        COLOR_NODE
    }
}

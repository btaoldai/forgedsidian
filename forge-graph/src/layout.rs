//! Force-directed graph layout — Phase 2 placeholder.
//!
//! This module will implement a force-directed layout algorithm (e.g.
//! Fruchterman-Reingold) to produce (x, y) coordinates for each node.
//! The coordinates are sent to the Leptos frontend via the IPC snapshot.
//!
//! Phase 1: stub only.

use forge_core::NoteId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 2D position of a note node on the canvas.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct NodePosition {
    pub x: f64,
    pub y: f64,
}

/// Compute a naive circular layout for the given node set.
///
/// A proper force-directed implementation is planned for Phase 2.
pub fn circular_layout(nodes: &[NoteId]) -> HashMap<NoteId, NodePosition> {
    use std::f64::consts::TAU;
    let n = nodes.len();
    let radius = 300.0_f64.max(n as f64 * 20.0);
    nodes
        .iter()
        .enumerate()
        .map(|(i, &id)| {
            let angle = TAU * i as f64 / n.max(1) as f64;
            (
                id,
                NodePosition {
                    x: radius * angle.cos(),
                    y: radius * angle.sin(),
                },
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circular_layout_empty() {
        // Circular layout of empty slice should return empty map
        let nodes = vec![];
        let layout = circular_layout(&nodes);
        assert_eq!(layout.len(), 0);
    }

    #[test]
    fn test_circular_layout_single_node() {
        // Single node should be placed at angle 0
        let id = NoteId::new();
        let nodes = vec![id];
        let layout = circular_layout(&nodes);
        assert_eq!(layout.len(), 1);
        let pos = layout.get(&id).expect("node should have position");
        // At angle 0: x = radius * cos(0) = radius, y = radius * sin(0) = 0
        let radius = 300.0_f64.max(1.0 * 20.0); // max(300, 20) = 300
        assert!((pos.x - radius).abs() < 1e-9);
        assert!(pos.y.abs() < 1e-9);
    }

    #[test]
    fn test_circular_layout_multiple_nodes() {
        // Multiple nodes should be distributed around circle
        let id1 = NoteId::new();
        let id2 = NoteId::new();
        let id3 = NoteId::new();
        let nodes = vec![id1, id2, id3];
        let layout = circular_layout(&nodes);
        assert_eq!(layout.len(), 3);
        // All nodes should have valid positions
        assert!(layout.contains_key(&id1));
        assert!(layout.contains_key(&id2));
        assert!(layout.contains_key(&id3));
    }

    #[test]
    fn test_circular_layout_radius_calculation() {
        // Radius should be max(300, n * 20) where n is node count
        let nodes: Vec<_> = (0..10).map(|_| NoteId::new()).collect();
        let layout = circular_layout(&nodes);
        // For 10 nodes, radius = max(300, 10*20) = max(300, 200) = 300
        for pos in layout.values() {
            let distance = (pos.x * pos.x + pos.y * pos.y).sqrt();
            assert!(
                (distance - 300.0).abs() < 1e-9,
                "node should be at radius distance"
            );
        }
    }

    #[test]
    fn test_circular_layout_large_graph() {
        // Large number of nodes should be handled
        let nodes: Vec<_> = (0..100).map(|_| NoteId::new()).collect();
        let layout = circular_layout(&nodes);
        assert_eq!(layout.len(), 100);
        // For 100 nodes, radius = max(300, 100*20) = max(300, 2000) = 2000
        for pos in layout.values() {
            let distance = (pos.x * pos.x + pos.y * pos.y).sqrt();
            assert!(
                (distance - 2000.0).abs() < 1e-9,
                "node should be at radius distance"
            );
        }
    }
}

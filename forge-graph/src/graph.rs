//! The `NoteGraph` — a directed graph of notes connected by wikilinks.

use forge_core::NoteId;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A directed graph where each node is a `NoteId` and each edge is a wikilink.
///
/// `NoteGraph` is the authoritative backlink structure for the vault.  It is
/// rebuilt from scratch whenever the index is refreshed (see `forge-vault`).
pub struct NoteGraph {
    /// The underlying petgraph directed graph.
    inner: DiGraph<NoteId, ()>,
    /// Maps a `NoteId` to its petgraph `NodeIndex` for O(1) lookups.
    index_map: HashMap<NoteId, NodeIndex>,
}

impl NoteGraph {
    /// Create an empty graph.
    pub fn new() -> Self {
        Self {
            inner: DiGraph::new(),
            index_map: HashMap::new(),
        }
    }

    /// Add a note node to the graph.  No-op if the note is already present.
    pub fn add_note(&mut self, id: NoteId) -> NodeIndex {
        if let Some(&idx) = self.index_map.get(&id) {
            return idx;
        }
        let idx = self.inner.add_node(id);
        self.index_map.insert(id, idx);
        idx
    }

    /// Add a directed wikilink edge from `from` to `to`.
    ///
    /// Both notes are added to the graph if they are not already present.
    pub fn add_link(&mut self, from: NoteId, to: NoteId) {
        let from_idx = self.add_note(from);
        let to_idx = self.add_note(to);
        self.inner.add_edge(from_idx, to_idx, ());
    }

    /// Return the `NodeIndex` for a note, or `None` if it is not in the graph.
    pub fn node_index(&self, id: &NoteId) -> Option<NodeIndex> {
        self.index_map.get(id).copied()
    }

    /// Return a reference to the underlying petgraph.
    pub fn inner(&self) -> &DiGraph<NoteId, ()> {
        &self.inner
    }

    /// Return the total number of nodes.
    pub fn node_count(&self) -> usize {
        self.inner.node_count()
    }

    /// Return the total number of edges (links).
    pub fn edge_count(&self) -> usize {
        self.inner.edge_count()
    }

    /// Remove all outgoing edges from a given node.
    ///
    /// Used during incremental re-indexing to clear stale wikilinks before
    /// re-adding the current ones.
    pub fn remove_note_edges(&mut self, note_id: NoteId) {
        if let Some(idx) = self.index_map.get(&note_id).copied() {
            // Collect edges to remove (cannot mutate while iterating).
            let edges_to_remove: Vec<_> = self.inner.edges(idx).map(|e| e.id()).collect();
            for edge_id in edges_to_remove {
                self.inner.remove_edge(edge_id);
            }
        }
    }

    /// Serialize the graph into a frontend-safe snapshot.
    ///
    /// Nodes are the string representation of each [`NoteId`];
    /// edges are `(from_id, to_id)` string pairs.
    pub fn snapshot(&self) -> GraphSnapshot {
        let nodes: Vec<String> = self.inner.node_weights().map(|id| id.to_string()).collect();

        let edges: Vec<(String, String)> = self
            .inner
            .edge_indices()
            .map(|e| {
                let (a, b) = self.inner.edge_endpoints(e).expect("valid edge");
                (self.inner[a].to_string(), self.inner[b].to_string())
            })
            .collect();

        GraphSnapshot {
            nodes,
            edges,
            id_to_path: HashMap::new(),
        }
    }
}

impl Default for NoteGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// A serializable snapshot of the graph for IPC transfer to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSnapshot {
    /// UUID strings — one per note node.
    pub nodes: Vec<String>,
    /// Directed wikilink pairs `(from_id, to_id)`.
    pub edges: Vec<(String, String)>,
    /// Mapping from UUID string to absolute file path (for click-to-open).
    #[serde(default)]
    pub id_to_path: std::collections::HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_empty_graph() {
        // Creating a new graph should result in an empty graph with 0 nodes and 0 edges
        let graph = NoteGraph::new();
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_default_identical_to_new() {
        // Default impl should be identical to new()
        let graph = NoteGraph::default();
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_add_single_note() {
        // Adding a single note should increase node_count to 1
        let mut graph = NoteGraph::new();
        let id = NoteId::new();
        graph.add_note(id);
        assert_eq!(graph.node_count(), 1);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_add_duplicate_note() {
        // Adding the same note twice should not increase node_count beyond 1
        let mut graph = NoteGraph::new();
        let id = NoteId::new();
        graph.add_note(id);
        graph.add_note(id);
        assert_eq!(graph.node_count(), 1);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_add_multiple_distinct_notes() {
        // Adding multiple distinct notes should increase node_count accordingly
        let mut graph = NoteGraph::new();
        let id1 = NoteId::new();
        let id2 = NoteId::new();
        let id3 = NoteId::new();
        graph.add_note(id1);
        graph.add_note(id2);
        graph.add_note(id3);
        assert_eq!(graph.node_count(), 3);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_add_link_between_two_notes() {
        // Adding a link from A to B should create an edge
        let mut graph = NoteGraph::new();
        let id_a = NoteId::new();
        let id_b = NoteId::new();
        graph.add_link(id_a, id_b);
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn test_add_link_auto_creates_notes() {
        // add_link should automatically create notes if they don't exist
        let mut graph = NoteGraph::new();
        let id_a = NoteId::new();
        let id_b = NoteId::new();
        graph.add_link(id_a, id_b);
        assert!(graph.node_index(&id_a).is_some());
        assert!(graph.node_index(&id_b).is_some());
    }

    #[test]
    fn test_add_multiple_links() {
        // Adding multiple distinct links should increase edge_count
        let mut graph = NoteGraph::new();
        let id_a = NoteId::new();
        let id_b = NoteId::new();
        let id_c = NoteId::new();
        graph.add_link(id_a, id_b);
        graph.add_link(id_b, id_c);
        graph.add_link(id_c, id_a);
        assert_eq!(graph.node_count(), 3);
        assert_eq!(graph.edge_count(), 3);
    }

    #[test]
    fn test_add_duplicate_link() {
        // Adding the same link twice creates two edges (petgraph allows multi-edges)
        let mut graph = NoteGraph::new();
        let id_a = NoteId::new();
        let id_b = NoteId::new();
        graph.add_link(id_a, id_b);
        graph.add_link(id_a, id_b);
        assert_eq!(graph.edge_count(), 2);
    }

    #[test]
    fn test_node_index_existing_note() {
        // node_index should return Some for a note that exists
        let mut graph = NoteGraph::new();
        let id = NoteId::new();
        graph.add_note(id);
        assert!(graph.node_index(&id).is_some());
    }

    #[test]
    fn test_node_index_nonexistent_note() {
        // node_index should return None for a note that does not exist
        let graph = NoteGraph::new();
        let id = NoteId::new();
        assert!(graph.node_index(&id).is_none());
    }

    #[test]
    fn test_snapshot_empty_graph() {
        // Snapshot of empty graph should have empty nodes and edges vectors
        let graph = NoteGraph::new();
        let snap = graph.snapshot();
        assert_eq!(snap.nodes.len(), 0);
        assert_eq!(snap.edges.len(), 0);
    }

    #[test]
    fn test_snapshot_single_node() {
        // Snapshot of graph with one node should contain one node string
        let mut graph = NoteGraph::new();
        let id = NoteId::new();
        graph.add_note(id);
        let snap = graph.snapshot();
        assert_eq!(snap.nodes.len(), 1);
        assert_eq!(snap.edges.len(), 0);
        assert_eq!(snap.nodes[0], id.to_string());
    }

    #[test]
    fn test_snapshot_single_link() {
        // Snapshot of graph with one link should contain two nodes and one edge
        let mut graph = NoteGraph::new();
        let id_a = NoteId::new();
        let id_b = NoteId::new();
        graph.add_link(id_a, id_b);
        let snap = graph.snapshot();
        assert_eq!(snap.nodes.len(), 2);
        assert_eq!(snap.edges.len(), 1);
        assert!(snap.nodes.contains(&id_a.to_string()));
        assert!(snap.nodes.contains(&id_b.to_string()));
        assert_eq!(snap.edges[0], (id_a.to_string(), id_b.to_string()));
    }

    #[test]
    fn test_snapshot_complex_graph() {
        // Snapshot of complex graph should serialize all nodes and edges correctly
        let mut graph = NoteGraph::new();
        let id_a = NoteId::new();
        let id_b = NoteId::new();
        let id_c = NoteId::new();
        graph.add_link(id_a, id_b);
        graph.add_link(id_b, id_c);
        graph.add_link(id_c, id_a);
        let snap = graph.snapshot();
        assert_eq!(snap.nodes.len(), 3);
        assert_eq!(snap.edges.len(), 3);
        // Verify all nodes are present (order may vary)
        assert!(snap.nodes.contains(&id_a.to_string()));
        assert!(snap.nodes.contains(&id_b.to_string()));
        assert!(snap.nodes.contains(&id_c.to_string()));
        // Verify edges are correct (petgraph iteration order may vary)
        let edges_set: std::collections::HashSet<_> = snap.edges.iter().cloned().collect();
        assert!(edges_set.contains(&(id_a.to_string(), id_b.to_string())));
        assert!(edges_set.contains(&(id_b.to_string(), id_c.to_string())));
        assert!(edges_set.contains(&(id_c.to_string(), id_a.to_string())));
    }

    #[test]
    fn test_add_note_returns_node_index() {
        // add_note should return a valid NodeIndex
        let mut graph = NoteGraph::new();
        let id = NoteId::new();
        let idx = graph.add_note(id);
        assert_eq!(graph.node_index(&id), Some(idx));
    }

    #[test]
    fn test_add_note_duplicate_returns_same_index() {
        // Adding the same note twice should return the same NodeIndex
        let mut graph = NoteGraph::new();
        let id = NoteId::new();
        let idx1 = graph.add_note(id);
        let idx2 = graph.add_note(id);
        assert_eq!(idx1, idx2);
    }
}

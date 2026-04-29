//! Backlink and forward-link queries on the `NoteGraph`.

use crate::{GraphError, NoteGraph};
use forge_core::NoteId;
use petgraph::Direction;

/// Return all notes that link *to* the given note (backlinks).
///
/// # Errors
/// Returns [`GraphError::NoteNotFound`] if `id` is not in the graph.
pub fn backlinks(graph: &NoteGraph, id: &NoteId) -> Result<Vec<NoteId>, GraphError> {
    let idx = graph
        .node_index(id)
        .ok_or(GraphError::NoteNotFound { id: *id })?;

    let backlinkers = graph
        .inner()
        .neighbors_directed(idx, Direction::Incoming)
        .map(|n| {
            *graph
                .inner()
                .node_weight(n)
                .expect("node weight always set")
        })
        .collect();

    Ok(backlinkers)
}

/// Return all notes that the given note links *to* (forward links).
///
/// # Errors
/// Returns [`GraphError::NoteNotFound`] if `id` is not in the graph.
pub fn forward_links(graph: &NoteGraph, id: &NoteId) -> Result<Vec<NoteId>, GraphError> {
    let idx = graph
        .node_index(id)
        .ok_or(GraphError::NoteNotFound { id: *id })?;

    let targets = graph
        .inner()
        .neighbors_directed(idx, Direction::Outgoing)
        .map(|n| {
            *graph
                .inner()
                .node_weight(n)
                .expect("node weight always set")
        })
        .collect();

    Ok(targets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backlinks_nonexistent_note() {
        // Querying backlinks for a non-existent note should return GraphError::NoteNotFound
        let graph = NoteGraph::new();
        let id = NoteId::new();
        let result = backlinks(&graph, &id);
        assert!(result.is_err());
        match result {
            Err(GraphError::NoteNotFound { .. }) => (),
            _ => panic!("expected NoteNotFound error"),
        }
    }

    #[test]
    fn test_backlinks_isolated_node() {
        // A note with no incoming edges should return empty vec
        let mut graph = NoteGraph::new();
        let id = NoteId::new();
        graph.add_note(id);
        let result = backlinks(&graph, &id).expect("note exists");
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_backlinks_single_backlink() {
        // A note with one incoming edge should return the source
        let mut graph = NoteGraph::new();
        let id_a = NoteId::new();
        let id_b = NoteId::new();
        graph.add_link(id_a, id_b); // a -> b
        let result = backlinks(&graph, &id_b).expect("note b exists");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], id_a);
    }

    #[test]
    fn test_backlinks_multiple() {
        // A note with multiple backlinks should return all sources
        let mut graph = NoteGraph::new();
        let id_a = NoteId::new();
        let id_b = NoteId::new();
        let id_c = NoteId::new();
        graph.add_link(id_a, id_c); // a -> c
        graph.add_link(id_b, id_c); // b -> c
        let result = backlinks(&graph, &id_c).expect("note c exists");
        assert_eq!(result.len(), 2);
        assert!(result.contains(&id_a));
        assert!(result.contains(&id_b));
    }

    #[test]
    fn test_forward_links_nonexistent_note() {
        // Querying forward links for a non-existent note should return GraphError::NoteNotFound
        let graph = NoteGraph::new();
        let id = NoteId::new();
        let result = forward_links(&graph, &id);
        assert!(result.is_err());
        match result {
            Err(GraphError::NoteNotFound { .. }) => (),
            _ => panic!("expected NoteNotFound error"),
        }
    }

    #[test]
    fn test_forward_links_isolated_node() {
        // A note with no outgoing edges should return empty vec
        let mut graph = NoteGraph::new();
        let id = NoteId::new();
        graph.add_note(id);
        let result = forward_links(&graph, &id).expect("note exists");
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_forward_links_single_link() {
        // A note with one outgoing edge should return the target
        let mut graph = NoteGraph::new();
        let id_a = NoteId::new();
        let id_b = NoteId::new();
        graph.add_link(id_a, id_b); // a -> b
        let result = forward_links(&graph, &id_a).expect("note a exists");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], id_b);
    }

    #[test]
    fn test_forward_links_multiple() {
        // A note with multiple outgoing edges should return all targets
        let mut graph = NoteGraph::new();
        let id_a = NoteId::new();
        let id_b = NoteId::new();
        let id_c = NoteId::new();
        graph.add_link(id_a, id_b); // a -> b
        graph.add_link(id_a, id_c); // a -> c
        let result = forward_links(&graph, &id_a).expect("note a exists");
        assert_eq!(result.len(), 2);
        assert!(result.contains(&id_b));
        assert!(result.contains(&id_c));
    }

    #[test]
    fn test_backlinks_and_forward_links_symmetry() {
        // If a -> b, then backlinks(b) should contain a and forward_links(a) should contain b
        let mut graph = NoteGraph::new();
        let id_a = NoteId::new();
        let id_b = NoteId::new();
        graph.add_link(id_a, id_b);

        let forward = forward_links(&graph, &id_a).expect("a exists");
        let backlinks_b = backlinks(&graph, &id_b).expect("b exists");

        assert_eq!(forward, vec![id_b]);
        assert_eq!(backlinks_b, vec![id_a]);
    }

    #[test]
    fn test_complex_graph_queries() {
        // Complex graph: a->b, a->c, b->c, c->a
        let mut graph = NoteGraph::new();
        let id_a = NoteId::new();
        let id_b = NoteId::new();
        let id_c = NoteId::new();
        graph.add_link(id_a, id_b);
        graph.add_link(id_a, id_c);
        graph.add_link(id_b, id_c);
        graph.add_link(id_c, id_a);

        // Check forward links
        let forward_a = forward_links(&graph, &id_a).expect("a exists");
        assert_eq!(forward_a.len(), 2);
        assert!(forward_a.contains(&id_b));
        assert!(forward_a.contains(&id_c));

        let forward_b = forward_links(&graph, &id_b).expect("b exists");
        assert_eq!(forward_b.len(), 1);
        assert_eq!(forward_b[0], id_c);

        let forward_c = forward_links(&graph, &id_c).expect("c exists");
        assert_eq!(forward_c.len(), 1);
        assert_eq!(forward_c[0], id_a);

        // Check backlinks
        let backlinks_a = backlinks(&graph, &id_a).expect("a exists");
        assert_eq!(backlinks_a.len(), 1);
        assert_eq!(backlinks_a[0], id_c);

        let backlinks_b = backlinks(&graph, &id_b).expect("b exists");
        assert_eq!(backlinks_b.len(), 1);
        assert_eq!(backlinks_b[0], id_a);

        let backlinks_c = backlinks(&graph, &id_c).expect("c exists");
        assert_eq!(backlinks_c.len(), 2);
        assert!(backlinks_c.contains(&id_a));
        assert!(backlinks_c.contains(&id_b));
    }
}

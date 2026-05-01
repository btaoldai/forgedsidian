//! Graph-specific errors.

use forge_core::NoteId;
use thiserror::Error;

/// Top-level error type for `forge-graph`.
#[derive(Debug, Error)]
pub enum GraphError {
    /// A queried note is not present in the graph.
    #[error("note {id} not found in graph")]
    NoteNotFound { id: NoteId },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_error_note_not_found() {
        // GraphError::NoteNotFound should contain the correct NoteId
        let id = NoteId::new();
        let error = GraphError::NoteNotFound { id };
        match error {
            GraphError::NoteNotFound { id: e_id } => {
                assert_eq!(e_id, id);
            }
        }
    }

    #[test]
    fn test_graph_error_display() {
        // Error message should be formatted correctly
        let id = NoteId::new();
        let error = GraphError::NoteNotFound { id };
        let msg = format!("{error}");
        assert!(msg.contains("not found in graph"));
    }
}

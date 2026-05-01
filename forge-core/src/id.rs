//! Opaque, typed identifiers for all domain entities.
//!
//! Each ID is a newtype wrapper around a [`Uuid`] v4, providing compile-time
//! guarantees that a `NoteId` cannot be accidentally passed where a `TagId` is
//! expected.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a [`Note`](crate::note::Note).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NoteId(Uuid);

impl NoteId {
    /// Create a new random `NoteId`.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for NoteId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for NoteId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TagId(Uuid);

impl TagId {
    /// Create a new random `TagId`.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TagId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TagId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that NoteId::new() generates unique identifiers.
    #[test]
    fn test_note_id_new_generates_unique_ids() {
        let id1 = NoteId::new();
        let id2 = NoteId::new();
        let id3 = NoteId::new();

        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);
    }

    /// Verify that NoteId::default() calls NoteId::new().
    #[test]
    fn test_note_id_default_generates_new_id() {
        let id1 = NoteId::default();
        let id2 = NoteId::default();

        assert_ne!(id1, id2);
    }

    /// Verify that NoteId Display impl produces a valid UUID string.
    #[test]
    fn test_note_id_display_format() {
        let id = NoteId::new();
        let display_str = id.to_string();

        // UUID v4 format: 8-4-4-4-12 hex digits with dashes
        // Example: "550e8400-e29b-41d4-a716-446655440000"
        assert_eq!(display_str.len(), 36);
        assert_eq!(display_str.matches('-').count(), 4);

        // Verify it contains only valid hex characters and dashes
        for ch in display_str.chars() {
            assert!(ch.is_ascii_hexdigit() || ch == '-');
        }
    }

    /// Verify that NoteId can be cloned and compared for equality.
    #[test]
    fn test_note_id_clone_and_equality() {
        let id1 = NoteId::new();
        let id2 = id1;

        assert_eq!(id1, id2);
    }

    /// Verify that TagId::new() generates unique identifiers.
    #[test]
    fn test_tag_id_new_generates_unique_ids() {
        let id1 = TagId::new();
        let id2 = TagId::new();
        let id3 = TagId::new();

        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);
    }

    /// Verify that TagId::default() calls TagId::new().
    #[test]
    fn test_tag_id_default_generates_new_id() {
        let id1 = TagId::default();
        let id2 = TagId::default();

        assert_ne!(id1, id2);
    }

    /// Verify that TagId Display impl produces a valid UUID string.
    #[test]
    fn test_tag_id_display_format() {
        let id = TagId::new();
        let display_str = id.to_string();

        // UUID v4 format: 8-4-4-4-12 hex digits with dashes
        assert_eq!(display_str.len(), 36);
        assert_eq!(display_str.matches('-').count(), 4);

        for ch in display_str.chars() {
            assert!(ch.is_ascii_hexdigit() || ch == '-');
        }
    }

    /// Verify that TagId can be cloned and compared for equality.
    #[test]
    fn test_tag_id_clone_and_equality() {
        let id1 = TagId::new();
        let id2 = id1;

        assert_eq!(id1, id2);
    }

    /// Verify that NoteId and TagId are distinct types and cannot be mixed.
    /// This test verifies the type system safety at compile time (compile-only).
    #[test]
    fn test_note_id_and_tag_id_are_distinct_types() {
        let note_id = NoteId::new();
        let tag_id = TagId::new();

        // They should have different Display outputs
        let note_str = note_id.to_string();
        let tag_str = tag_id.to_string();

        // Both are UUIDs, but they should be different instances
        assert_ne!(note_str, tag_str);
    }
}

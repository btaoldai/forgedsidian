//! Domain events emitted by the Forgedsidian engine.
//!
//! Events flow from the engine core → Tauri backend → Leptos frontend via the
//! IPC channel.  Keeping them in `forge-core` allows both the backend
//! (`src-tauri`) and the WASM frontend (`src`) to share the same types without
//! a circular dependency.

use crate::id::NoteId;
use serde::{Deserialize, Serialize};

/// An event produced by the PKM engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EngineEvent {
    /// A note was created or updated.
    NoteChanged { id: NoteId },
    /// A note was deleted from the vault.
    NoteDeleted { id: NoteId },
    /// The backlink graph was rebuilt.
    GraphRebuilt,
    /// Full-text index rebuild completed.
    IndexRebuilt,
}

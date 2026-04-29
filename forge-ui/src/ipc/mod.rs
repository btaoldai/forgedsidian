//! Tauri IPC bridge — typed wrappers around `window.__TAURI_INTERNALS__.invoke`.
//!
//! This module provides async functions to invoke Tauri commands from the WASM frontend.
//! Each function serializes arguments using `serde-wasm-bindgen` and deserializes responses.

pub mod vault_ops;
pub mod file_ops;
pub mod graph;
pub mod tags;

/// Reject paths containing traversal components or null bytes.
///
/// Called before every IPC invoke that accepts a file path argument.
/// This is a defense-in-depth measure; the backend also validates.
pub(crate) fn validate_path_helper(path: &str) -> Result<(), String> {
    if path.contains('\0') {
        return Err("path contains null byte".to_string());
    }
    // Reject `..` as a path component (handles both `/` and `\` separators).
    for segment in path.split(['/', '\\']) {
        if segment == ".." {
            return Err("path traversal not allowed".to_string());
        }
    }
    Ok(())
}

// Re-export types and functions for backward compatibility with existing call-sites.
pub use crate::ipc::vault_ops::{
    open_vault, pick_and_open_vault, listen_vault_events, listen_indexing_progress,
    list_notes, get_note, search_notes,
    get_graph_snapshot, save_canvas_drawings, load_canvas_drawings, export_canvas_svg,
    IndexingProgress,
};
pub use crate::ipc::file_ops::{
    read_file, save_note, create_note, create_folder, list_all_files, list_folders,
    delete_folder, move_file, move_folder,
};
pub use crate::ipc::graph::{resolve_wikilink, open_in_default_app};
pub use crate::ipc::tags::{list_tags as list_vault_tags, notes_by_tag};

// Public types (GraphSnapshot is used by both vault_ops and graph callers).
pub use crate::ipc::vault_ops::GraphSnapshot;

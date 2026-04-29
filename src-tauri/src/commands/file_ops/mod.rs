//! File CRUD commands: read, save, create, move, delete (files and folders).
//!
//! This module is split across sub-modules:
//! - [`read`] — read_file, save_note
//! - [`create`] — create_note, create_folder
//! - [`delete`] — delete_folder
//! - [`move_ops`] — move_file, move_folder

pub mod create;
pub mod delete;
pub mod move_ops;
pub mod read;

// Wildcard re-exports are required so that Tauri's `#[tauri::command]` macro-generated
// `__cmd__*` structs (used by `generate_handler![]`) are also visible at this path.
pub use self::create::*;
pub use self::delete::*;
pub use self::move_ops::*;
pub use self::read::*;

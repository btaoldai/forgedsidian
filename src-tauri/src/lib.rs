//! # forge-app (src-tauri)
//!
//! Tauri application entry point and IPC command layer.
//!
//! ## Architecture
//! ```text
//! Leptos WASM (src/)
//!   |  invoke("command_name", payload)
//!   v
//! Tauri IPC bridge
//!   |
//!   v
//! commands.rs  ← thin handlers, no business logic
//!   |
//!   v
//! forge-vault / forge-graph / forge-canvas / forge-editor
//! ```
//!
//! Command handlers must be thin: they validate the input, delegate to the
//! appropriate engine crate, and return a serialisable result.  No business
//! logic lives in this crate.

// Public for testing; individual command modules are still private to IPC.
pub mod commands;

use commands::ForgeState;
use tauri::Manager;
use tracing_subscriber::{fmt, EnvFilter};

/// Application entry point called from `main.rs`.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialise structured logging.  Level can be overridden via RUST_LOG.
    fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::new("forge_app_lib=debug,forge_vault=info,forge_graph=info")
        }))
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .setup(|app| {
            app.manage(ForgeState::new());

            // Show the main window AFTER window-state plugin has restored
            // saved dimensions.  The window starts hidden (visible: false
            // in tauri.conf.json) to avoid a fullscreen flash on startup.
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // vault_ops
            commands::vault_ops::open_vault,
            commands::vault_ops::pick_and_open_vault,
            commands::vault_ops::list_notes,
            commands::vault_ops::get_note,
            commands::vault_ops::search_notes,
            commands::vault_ops::get_graph_snapshot,
            commands::vault_ops::get_canvas,
            commands::vault_ops::save_canvas_drawings,
            commands::vault_ops::load_canvas_drawings,
            commands::vault_ops::export_canvas_svg,
            commands::vault_ops::resolve_wikilink,
            commands::vault_ops::open_in_default_app,
            // file_ops
            commands::file_ops::save_note,
            commands::file_ops::create_note,
            commands::file_ops::create_folder,
            commands::file_ops::delete_folder,
            commands::file_ops::move_file,
            commands::file_ops::move_folder,
            commands::file_ops::read_file,
            // scan
            commands::scan::list_folders,
            commands::scan::list_all_files,
            // tags
            commands::tags::list_tags,
            commands::tags::notes_by_tag,
        ])
        // CSP is enforced by tauri.conf.json "security.csp" (HTTP header).
        // No JS-level CSP injection needed.
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}

//! # forge-ui
//!
//! Leptos CSR (Client-Side Rendering) frontend for Forgexalith.
//!
//! This crate compiles to WASM and is embedded in the Tauri WebView.
//! All communication with the Rust backend goes through the Tauri IPC
//! bridge (`invoke` / `listen`).
//!
//! ## Build
//! Built with `cargo-leptos build` (not plain `cargo build`).
//! Do NOT add this crate to the native workspace members.
//!
//! ## Module layout
//! - [`app`]        — root `<App />` component and global state
//! - [`components`] — reusable UI components (Sidebar, Editor, GraphView, Canvas)

pub mod app;
pub mod components;
pub mod ipc;
pub mod settings;
pub mod tab_manager;

use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// WASM entry point — called by the generated JS glue code.
#[wasm_bindgen(start)]
pub fn main() {
    // Install a panic hook that forwards Rust panics to the browser console.
    console_error_panic_hook::set_once();

    // WebView2 (Windows) workaround: register global dragover + drop listeners on the
    // document so the browser never overrides the cursor with "forbidden" during in-page
    // DnD. Individual element handlers still fire and control actual drop logic.
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            let on_dragover =
                Closure::<dyn Fn(web_sys::DragEvent)>::new(|ev: web_sys::DragEvent| {
                    ev.prevent_default();
                });
            let _ = document.add_event_listener_with_callback(
                "dragover",
                on_dragover.as_ref().unchecked_ref(),
            );
            on_dragover.forget();

            // Also prevent the browser from navigating when a file is dropped outside
            // a designated drop zone (e.g., OS file accidentally dropped onto WebView).
            let on_drop =
                Closure::<dyn Fn(web_sys::DragEvent)>::new(|ev: web_sys::DragEvent| {
                    ev.prevent_default();
                });
            let _ = document.add_event_listener_with_callback(
                "drop",
                on_drop.as_ref().unchecked_ref(),
            );
            on_drop.forget();
        }
    }

    // Mount the root App component into <body>.
    mount_to_body(app::App);
}

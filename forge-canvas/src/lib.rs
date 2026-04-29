//! # forge-canvas
//!
//! Canvas rendering logic, hit-testing and ABCDE prioritization layout.
//!
//! This crate is intentionally **`no_std`-compatible** (it avoids `tokio` and
//! OS-level APIs) so that it can compile to both native and WASM targets.  The
//! Tauri backend uses it for hit-testing and state management; the Leptos
//! frontend imports it (via WASM) for rendering.
//!
//! ## Key design constraints
//! - No async, no `tokio` — synchronous, pure computation only
//! - Must compile to `wasm32-unknown-unknown` (no OS calls)
//! - Serde traits on all public types (IPC serialisation)
//!
//! ## Modules
//! - [`canvas`]  — the infinite canvas state machine
//! - [`item`]    — canvas items (notes, tasks, shapes)
//! - [`hit`]     — axis-aligned bounding-box hit testing
//! - [`abcde`]   — ABCDE prioritization logic
//! - [`error`]   — canvas-specific errors

pub mod abcde;
pub mod canvas;
pub mod error;
pub mod hit;
pub mod item;

pub use canvas::Canvas;
pub use error::CanvasError;

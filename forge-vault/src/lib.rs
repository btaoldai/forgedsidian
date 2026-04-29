//! # forge-vault
//!
//! Note storage, full-text indexing (Tantivy) and metadata search.
//!
//! ## Responsibilities
//! - Read/write Markdown files from the vault root directory
//! - Maintain a Tantivy full-text index of note bodies
//! - Expose query APIs used by the Tauri command layer
//! - Watch vault directory for file-system changes (Phase 3.3)
//!
//! ## Modules
//! - [`store`]          — VaultStore public API and orchestration
//! - [`scan`]           — filesystem scanning and wikilink extraction
//! - [`indexing`]       — incremental index and manifest updates
//! - [`graph_builder`]  — backlink graph construction from manifest
//! - [`index`]          — Tantivy index lifecycle and search
//! - [`manifest`]       — vault metadata cache with integrity checking
//! - [`error`]          — vault-specific errors
//! - [`audit`]          — operation logging and audit trail
//! - [`watcher`]        — file-system watcher for live reloading

pub mod audit;
pub mod error;
pub mod graph_builder;
pub mod index;
pub mod indexing;
pub mod manifest;
pub mod scan;
pub mod storage;
pub mod store;
pub mod watcher;

pub use audit::{AuditEvent, AuditLog};
pub use error::VaultError;
pub use storage::{RealFs, StorageBackend};
pub use store::{ProgressFn, ProgressStep, VaultStore};

//! # forge-core
//!
//! Shared foundation for the Forgedsidian PKM engine.
//!
//! This crate defines the **stable public types** that all other forge-* crates
//! depend on.  Nothing in this crate should import from sibling crates — it is
//! the dependency root of the workspace graph.
//!
//! ## Modules
//! - [`id`]      — opaque, typed identifiers (newtype wrappers around `Uuid`)
//! - [`note`]    — the canonical `Note` domain model
//! - [`event`]   — domain events emitted by the engine
//! - [`error`]   — shared error type hierarchy
//! - [`format`]  — file format constants and helpers (Markdown, frontmatter)
//! - [`frontmatter`] — zero-dependency YAML frontmatter parser

pub mod error;
pub mod event;
pub mod format;
pub mod frontmatter;
pub mod id;
pub mod link;
pub mod note;

// Re-export the most commonly used items at the crate root.
pub use error::CoreError;
pub use frontmatter::{parse_frontmatter, NoteFrontmatter};
pub use id::{NoteId, TagId};
pub use link::{Link, SimpleWikilinkExtractor, WikilinkExtractor};
pub use note::Note;

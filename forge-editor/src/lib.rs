//! # forge-editor
//!
//! Markdown AST, inline editing logic and YAML frontmatter parsing.
//!
//! ## Responsibilities
//! - Parse a raw Markdown string into a [`pulldown_cmark`] AST
//! - Extract and validate YAML frontmatter
//! - Extract wikilinks (`[[target]]`) and standard hyperlinks
//! - Provide a `diff` API for inline edits (Phase 2+)
//!
//! ## Modules
//! - [`parser`]      — Markdown → AST conversion
//! - [`frontmatter`] — YAML frontmatter extraction and validation
//! - [`links`]       — wikilink and hyperlink extraction
//! - [`error`]       — editor-specific errors

pub mod error;
pub mod frontmatter;
pub mod links;
pub mod parser;

pub use error::EditorError;
pub use links::PulldownWikilinkExtractor;

//! Canonical `Note` domain model.
//!
//! A `Note` represents a single Markdown document in the vault.  It is
//! intentionally kept flat — rendering, indexing and graph logic live in their
//! respective crates.

use crate::id::NoteId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// A note as stored in the vault.
///
/// The `body` field contains the raw Markdown source, including the YAML
/// frontmatter block.  Parsed views are produced by `forge-editor`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    /// Stable unique identifier, assigned once at creation.
    pub id: NoteId,
    /// Absolute path on the file system.
    pub path: PathBuf,
    /// Raw Markdown content (frontmatter included).
    pub body: String,
    /// Frontmatter key/value pairs parsed by `forge-editor`.
    /// Values are kept as raw JSON to avoid a schema commitment here.
    pub frontmatter: HashMap<String, serde_json::Value>,
    /// Wall-clock time of the last modification.
    pub modified_at: DateTime<Utc>,
    /// Wall-clock time of creation (frontmatter `created` field, or fs ctime).
    pub created_at: DateTime<Utc>,
}

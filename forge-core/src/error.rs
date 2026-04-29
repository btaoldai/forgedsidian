//! Shared error types for `forge-core`.

use thiserror::Error;

/// Top-level error type for the `forge-core` crate.
#[derive(Debug, Error)]
pub enum CoreError {
    /// An ID could not be parsed.
    #[error("invalid id: {0}")]
    InvalidId(String),

    /// A required field was absent from a note's frontmatter.
    #[error("missing frontmatter field: {field}")]
    MissingFrontmatterField { field: String },

    /// JSON (de)serialization failure.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

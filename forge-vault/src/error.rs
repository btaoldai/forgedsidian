//! Vault-specific errors.

use thiserror::Error;

/// Top-level error type for `forge-vault`.
#[derive(Debug, Error)]
pub enum VaultError {
    /// The vault root directory does not exist or is not readable.
    #[error("vault root not found or not readable: {path}")]
    RootNotFound { path: String },

    /// A file system operation failed.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// The Tantivy index directory could not be opened.
    #[error("cannot open index directory: {0}")]
    OpenDir(#[from] tantivy::directory::error::OpenDirectoryError),

    /// The Tantivy index is corrupt or cannot be opened.
    #[error("index error: {0}")]
    Index(#[from] tantivy::error::TantivyError),

    /// A full-text search query could not be parsed.
    #[error("query parse error: {0}")]
    QueryParse(#[from] tantivy::query::QueryParserError),

    /// A core domain error propagated from `forge-core`.
    #[error("core error: {0}")]
    Core(#[from] forge_core::CoreError),
}

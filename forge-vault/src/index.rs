//! Tantivy full-text index lifecycle and search.
//!
//! The index is stored alongside the vault root in a hidden `.forge-index/`
//! directory so it can be trivially rebuilt from the source Markdown files.
//!
//! ## Schema
//! | Field   | Options       | Purpose                                    |
//! |---------|---------------|--------------------------------------------|
//! | `id`    | STORED        | Note UUID (for dedup / delete-by-id)       |
//! | `path`  | STORED        | Absolute file path (returned in results)   |
//! | `title` | TEXT + STORED | Note filename stem (boosted in ranking)    |
//! | `body`  | TEXT          | Full Markdown body (not stored to save RAM)|
//! | `tags`  | TEXT + STORED | Space-separated frontmatter tags           |

use crate::VaultError;
use forge_core::{parse_frontmatter, Note};
use std::path::Path;
use tantivy::{
    collector::TopDocs,
    query::QueryParser,
    schema::{Field, Schema, Value, STORED, STRING, TEXT},
    Index, IndexReader, TantivyDocument,
};

/// Wraps a Tantivy index for the vault.
///
/// Holds field handles so callers never need to interact with the raw schema.
pub struct VaultIndex {
    inner: Index,
    reader: IndexReader,
    // Schema field handles — kept for document construction and querying.
    f_id: Field,
    f_path: Field,
    f_title: Field,
    f_body: Field,
    f_tags: Field,
}

impl VaultIndex {
    /// Open or create a Tantivy index at `<vault_root>/.forge-index/`.
    ///
    /// On Windows, if a previous process left stale memory-mapped files locked,
    /// Tantivy will fail to open or write to segment files. This method detects
    /// the error (PermissionDenied), deletes the corrupt index directory, and
    /// retries once from scratch — the index is always rebuildable from source
    /// Markdown files, so no data is lost.
    ///
    /// # Errors
    /// - [`VaultError::OpenDir`]  if the directory cannot be opened.
    /// - [`VaultError::Index`]    if Tantivy encounters an internal error.
    pub fn open_or_create(vault_root: &Path) -> Result<Self, VaultError> {
        let index_path = vault_root.join(".forge-index");

        match Self::try_open_index(&index_path) {
            Ok(idx) => Ok(idx),
            Err(e) => {
                // On Windows, stale mmap locks cause PermissionDenied.
                // Delete the index and retry — it's rebuildable.
                // Detection must handle multiple locales (English:
                // "Permission denied", French: "Accès refusé", etc.)
                // and multiple error formats ("code: 5", "os error 5").
                let err_str = format!("{e}");
                let err_lower = err_str.to_lowercase();
                let is_permission_err = err_str.contains("PermissionDenied")
                    || err_lower.contains("permission denied")
                    || err_lower.contains("accès refusé")
                    || err_lower.contains("acces refuse")
                    || err_str.contains("code: 5")
                    || err_str.contains("os error 5");
                if is_permission_err && index_path.exists() {
                    tracing::warn!(
                        path = %index_path.display(),
                        error = %e,
                        "stale index lock detected — deleting and rebuilding"
                    );
                    if let Err(rm_err) = std::fs::remove_dir_all(&index_path) {
                        tracing::error!(
                            error = %rm_err,
                            "failed to remove stale index directory"
                        );
                        return Err(e);
                    }
                    // Retry once after cleanup.
                    Self::try_open_index(&index_path)
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Internal helper: open or create the Tantivy index and verify the writer
    /// can acquire its lock.
    fn try_open_index(index_path: &Path) -> Result<Self, VaultError> {
        std::fs::create_dir_all(index_path)?;

        let mut sb = Schema::builder();
        let f_id = sb.add_text_field("id", STORED);
        let f_path = sb.add_text_field("path", STRING | STORED);
        let f_title = sb.add_text_field("title", TEXT | STORED);
        let f_body = sb.add_text_field("body", TEXT);
        let f_tags = sb.add_text_field("tags", TEXT | STORED);
        let schema = sb.build();

        let inner =
            Index::open_or_create(tantivy::directory::MmapDirectory::open(index_path)?, schema)?;

        // Probe the writer to detect stale locks early (before indexing
        // thousands of files). Drop the writer immediately — we only need
        // to verify the lock is acquirable.
        {
            let writer = inner.writer::<TantivyDocument>(15_000_000)?;
            drop(writer);
        }

        let reader = inner.reader()?;

        Ok(Self {
            inner,
            reader,
            f_id,
            f_path,
            f_title,
            f_body,
            f_tags,
        })
    }

    /// Replace the entire index with the given notes in a single commit.
    ///
    /// This is the preferred path at vault open time: one writer, one commit,
    /// one fsync — much faster than one-writer-per-note for large vaults.
    ///
    /// # Errors
    /// Returns [`VaultError::Index`] if Tantivy cannot write or commit.
    pub fn index_notes_batch(&self, notes: &[Note]) -> Result<(), VaultError> {
        let mut writer = self.inner.writer(50_000_000)?; // 50 MiB write buffer

        // Wipe any stale entries before re-indexing.
        writer.delete_all_documents()?;

        for note in notes {
            let title = note.path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

            // Extract tags from frontmatter for full-text tag search.
            let fm = parse_frontmatter(&note.body);
            let tags_str = fm.tags.join(" ");

            let mut doc = TantivyDocument::default();
            doc.add_text(self.f_id, note.id.to_string());
            doc.add_text(self.f_path, note.path.display().to_string());
            doc.add_text(self.f_title, title);
            doc.add_text(self.f_body, &note.body);
            doc.add_text(self.f_tags, &tags_str);

            writer.add_document(doc)?;
        }

        writer.commit()?;
        Ok(())
    }

    /// Full-text search over `title` and `body` fields.
    ///
    /// Returns up to `limit` file paths ranked by relevance.
    ///
    /// # Errors
    /// - [`VaultError::QueryParse`] if the query string is invalid.
    /// - [`VaultError::Index`]      on internal Tantivy errors.
    pub fn search(&self, query_str: &str, limit: usize) -> Result<Vec<String>, VaultError> {
        // Reload the reader so freshly committed segments are visible.
        self.reader.reload()?;
        let searcher = self.reader.searcher();

        let mut parser = QueryParser::for_index(&self.inner, vec![self.f_title, self.f_body]);
        // Title matches rank higher than body matches.
        parser.set_field_boost(self.f_title, 3.0);

        let query = parser.parse_query(query_str)?;
        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit).order_by_score())?;

        let mut results = Vec::with_capacity(top_docs.len());
        for (_, addr) in top_docs {
            let doc: TantivyDocument = searcher.doc(addr)?;
            // The `path` field is STORED — extract its string value.
            if let Some(field_val) = doc.get_first(self.f_path) {
                if let Some(s) = field_val.as_str() {
                    results.push(String::from(s));
                }
            }
        }

        Ok(results)
    }

    /// Return a reference to the underlying Tantivy index (low-level access).
    pub fn inner(&self) -> &Index {
        &self.inner
    }

    /// Index a single note (upsert: delete existing entry for this path, then add).
    ///
    /// Used for incremental re-indexing when a file changes on disk.
    pub fn index_single_note(&self, note: &Note) -> Result<(), VaultError> {
        let mut writer = self.inner.writer::<TantivyDocument>(15_000_000)?;

        // Delete any existing document with the same path.
        let path_str = note.path.display().to_string();
        let path_term = tantivy::Term::from_field_text(self.f_path, &path_str);
        writer.delete_term(path_term);

        let title = note.path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

        // Extract tags from frontmatter for full-text tag search.
        let fm = parse_frontmatter(&note.body);
        let tags_str = fm.tags.join(" ");

        let mut doc = TantivyDocument::default();
        doc.add_text(self.f_id, note.id.to_string());
        doc.add_text(self.f_path, &path_str);
        doc.add_text(self.f_title, title);
        doc.add_text(self.f_body, &note.body);
        doc.add_text(self.f_tags, &tags_str);

        writer.add_document(doc)?;
        writer.commit()?;
        Ok(())
    }

    /// Remove a note from the index by its absolute file path.
    ///
    /// Used when a Markdown file is deleted from the vault.
    pub fn remove_by_path(&self, path: &std::path::Path) -> Result<(), VaultError> {
        let mut writer = self.inner.writer::<TantivyDocument>(15_000_000)?;
        let path_str = path.display().to_string();
        let path_term = tantivy::Term::from_field_text(self.f_path, &path_str);
        writer.delete_term(path_term);
        writer.commit()?;
        Ok(())
    }
}

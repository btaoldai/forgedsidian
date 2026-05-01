//! File-system operations: read, write and scan vault notes.
//!
//! The [`VaultStore`] struct is the single entry point for all I/O on the
//! vault directory.  At [`VaultStore::open`] time it:
//!
//! 1. Loads the manifest (if any) to detect unchanged files.
//! 2. Walks the vault directory tree and diffs against the manifest.
//! 3. Only reads and indexes **new or modified** files (incremental).
//! 4. Removes **deleted** files from the Tantivy index.
//! 5. Rebuilds the backlink [`NoteGraph`] from the full set of known notes.
//! 6. Saves the updated manifest for next time.
//!
//! On a vault with no changes since last open, steps 3 and 4 are no-ops,
//! making the open time O(scan) instead of O(read-all + index-all).
//!
//! All paths returned by this module are absolute.

use crate::{
    audit::{AuditEvent, AuditLog},
    graph_builder,
    index::VaultIndex,
    indexing::{self, read_notes_from_paths},
    manifest::{self, Manifest, NoteEntry},
    scan::{self, extract_wikilink_targets},
    VaultError,
};
use forge_core::{Note, SimpleWikilinkExtractor, WikilinkExtractor};
use forge_graph::{graph::GraphSnapshot, NoteGraph};
use std::{
    collections::{BTreeSet, HashMap},
    path::{Path, PathBuf},
    time::SystemTime,
};
use tracing::instrument;

// ---------------------------------------------------------------------------
// Progress reporting
// ---------------------------------------------------------------------------

/// A progress update emitted during [`VaultStore::open_with_progress`].
///
/// Each step of the vault opening pipeline emits one or more updates so the
/// caller (typically the Tauri command layer) can relay them to the UI.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProgressStep {
    /// Current step number (1-based).
    pub step: u8,
    /// Total number of steps in the pipeline.
    pub total: u8,
    /// Short human-readable label, e.g. "Scanning files...".
    pub label: String,
    /// Optional detail, e.g. "7041 files found".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Callback type for progress reporting during vault open.
///
/// The callback is `Send` so it can be passed into `spawn_blocking`.
/// It is called synchronously from the vault open pipeline.
pub type ProgressFn = Box<dyn Fn(ProgressStep) + Send>;

// ---------------------------------------------------------------------------
// VaultStore
// ---------------------------------------------------------------------------

/// The on-disk vault storage, full-text index and backlink graph.
pub struct VaultStore {
    /// Absolute path to the vault root directory.
    root: PathBuf,
    /// Tantivy full-text index for note bodies and titles.
    index: VaultIndex,
    /// Directed backlink graph (wikilinks between notes).
    graph: NoteGraph,
    /// Persistent manifest — tracks path, NoteId, and mtime.
    manifest: Manifest,
    /// Append-only audit logger.
    audit: AuditLog,
    /// In-memory map: normalised tag → set of absolute note paths tagged with it.
    /// Built at open time and kept in sync by incremental updates.
    tag_index: HashMap<String, BTreeSet<PathBuf>>,
}

impl VaultStore {
    /// Open a vault rooted at `root` using the default wikilink extractor.
    ///
    /// Convenience method that calls [`open_with_extractor`] with a
    /// [`SimpleWikilinkExtractor`] — the dependency-free default from
    /// `forge-core`. If you need full Markdown parsing (external hyperlinks,
    /// not just wikilinks), use [`open_with_extractor`] with
    /// `forge_editor::PulldownWikilinkExtractor` instead.
    ///
    /// Uses the manifest for incremental indexing: only new/modified files
    /// are read and indexed.  If no manifest exists (first open), falls back
    /// to a full index build.
    ///
    /// This call is synchronous — wrap in `tokio::task::spawn_blocking` for
    /// non-blocking behaviour.
    ///
    /// # Errors
    /// - [`VaultError::RootNotFound`]  if the directory does not exist.
    /// - [`VaultError::Io`]            on any file-system error.
    /// - [`VaultError::Index`]         if Tantivy cannot open the index.
    ///
    /// [`open_with_extractor`]: Self::open_with_extractor
    pub fn open(root: impl AsRef<Path>) -> Result<Self, VaultError> {
        // Use the dependency-free default from forge-core.
        Self::open_with_extractor(root, &SimpleWikilinkExtractor)
    }

    /// Open a vault rooted at `root` with a custom wikilink extractor.
    ///
    /// This allows injecting a different [`WikilinkExtractor`] implementation
    /// (e.g. for testing or alternative parsing strategies).
    ///
    /// Uses the manifest for incremental indexing: only new/modified files
    /// are read and indexed.  If no manifest exists (first open), falls back
    /// to a full index build.
    ///
    /// This call is synchronous — wrap in `tokio::task::spawn_blocking` for
    /// non-blocking behaviour.
    ///
    /// # Arguments
    /// * `root` - Path to the vault root directory.
    /// * `extractor` - The [`WikilinkExtractor`] to use for link extraction.
    ///
    /// # Errors
    /// - [`VaultError::RootNotFound`]  if the directory does not exist.
    /// - [`VaultError::Io`]            on any file-system error.
    /// - [`VaultError::Index`]         if Tantivy cannot open the index.
    pub fn open_with_extractor(
        root: impl AsRef<Path>,
        extractor: &dyn WikilinkExtractor,
    ) -> Result<Self, VaultError> {
        // Delegate to the progress-aware version with a no-op callback.
        Self::open_with_progress(root, extractor, None)
    }

    /// Open a vault with optional progress reporting.
    ///
    /// Same as [`open_with_extractor`] but accepts an optional [`ProgressFn`]
    /// callback that is invoked at each step of the pipeline. This lets the
    /// caller (e.g. the Tauri command layer) relay real-time progress to the UI.
    ///
    /// Pass `None` for `progress` to get the same behaviour as
    /// [`open_with_extractor`].
    pub fn open_with_progress(
        root: impl AsRef<Path>,
        extractor: &dyn WikilinkExtractor,
        progress: Option<ProgressFn>,
    ) -> Result<Self, VaultError> {
        let root = root.as_ref().to_path_buf();
        let total: u8 = 6;

        // Helper: emit progress if a callback is provided.
        let emit = |step: u8, label: &str, detail: Option<String>| {
            if let Some(ref cb) = progress {
                cb(ProgressStep {
                    step,
                    total,
                    label: label.to_string(),
                    detail,
                });
            }
        };

        if !root.exists() {
            return Err(VaultError::RootNotFound {
                path: root.display().to_string(),
            });
        }

        // ── 1. Open (or create) the Tantivy index ──────────────────────────
        emit(1, "Opening search index...", None);
        let index = VaultIndex::open_or_create(&root)?;

        // ── 2. Load manifest (if any) ─────────────────────────────────────
        emit(2, "Loading manifest...", None);
        let mut manifest = Manifest::load(&root).unwrap_or_default();

        // ── 3. Scan disk and diff against manifest ────────────────────────
        emit(3, "Scanning files...", None);
        let on_disk = scan::scan_md_files(&root)?;
        let diff = manifest::diff(&on_disk, &manifest);

        emit(
            3,
            "Scanning files...",
            Some(format!(
                "{} files found — {} new, {} modified, {} deleted",
                on_disk.len(),
                diff.added.len(),
                diff.modified.len(),
                diff.deleted.len()
            )),
        );

        tracing::info!(
            root = %root.display(),
            on_disk = on_disk.len(),
            added = diff.added.len(),
            modified = diff.modified.len(),
            deleted = diff.deleted.len(),
            unchanged = diff.unchanged.len(),
            clean = diff.is_clean(),
            "vault diff computed",
        );

        // ── 4. Handle the diff ────────────────────────────────────────────
        // notes_for_tag_index accumulates all notes whose bodies we already
        // have in memory during this open, so we can build the tag index once.
        let mut notes_for_tag_index: Vec<forge_core::Note> = Vec::new();

        if diff.is_clean() {
            // Nothing changed — skip all I/O except graph rebuild.
            emit(4, "Indexing...", Some("No changes — skipping".to_string()));
            tracing::info!("vault unchanged — skipping re-index");
        } else {
            // Full re-index if this is the first open (no manifest).
            let is_fresh = manifest.notes.is_empty();

            if is_fresh {
                emit(
                    4,
                    "Indexing...",
                    Some(format!("Full index: {} files", on_disk.len())),
                );
                // First open: batch-index everything.
                let notes = read_notes_from_paths(&on_disk)?;
                index.index_notes_batch(&notes)?;

                // Populate manifest with wikilinks.
                for note in &notes {
                    let mtime = std::fs::metadata(&note.path)
                        .and_then(|m| m.modified())
                        .unwrap_or(SystemTime::UNIX_EPOCH);
                    let wikilinks = extract_wikilink_targets(&note.body, extractor);
                    manifest.upsert(
                        &note.path,
                        NoteEntry::with_wikilinks(note.id, mtime, wikilinks),
                    );
                }

                notes_for_tag_index = notes;
                tracing::info!(
                    notes = notes_for_tag_index.len(),
                    "initial full index complete"
                );
            } else {
                let changed = diff.added.len() + diff.modified.len();
                emit(
                    4,
                    "Indexing...",
                    Some(format!("Incremental: {changed} files")),
                );
                // Incremental: handle added + modified + deleted.
                indexing::apply_diff(&index, &mut manifest, &diff, extractor)?;
            }

            // Save updated manifest.
            manifest.save(&root)?;
        }

        // ── 5. Rebuild the backlink graph from manifest ───────────────────
        emit(
            5,
            "Building graph...",
            Some(format!("{} notes", manifest.notes.len())),
        );
        let graph = graph_builder::build_graph(&root, &manifest)?;

        // ── 6. Build the tag index ────────────────────────────────────────
        emit(6, "Building tag index...", None);
        // If we already loaded note bodies (first open), use them directly.
        // Otherwise (incremental or clean), read all manifest paths to build
        // the tag index.
        let mut tag_index: HashMap<String, BTreeSet<PathBuf>> = HashMap::new();

        if !notes_for_tag_index.is_empty() {
            // First open: use already-loaded note bodies.
            for note in &notes_for_tag_index {
                let fm = forge_core::parse_frontmatter(&note.body);
                for tag in fm.tags {
                    tag_index.entry(tag).or_default().insert(note.path.clone());
                }
            }
        } else {
            // Incremental or clean open: read bodies for all known notes.
            for path_str in manifest.notes.keys() {
                let path = PathBuf::from(path_str);
                if let Ok(body) = std::fs::read_to_string(&path) {
                    let fm = forge_core::parse_frontmatter(&body);
                    for tag in fm.tags {
                        tag_index.entry(tag).or_default().insert(path.clone());
                    }
                }
            }
        }

        emit(
            6,
            "Ready!",
            Some(format!(
                "{} notes, {} links, {} tags",
                manifest.notes.len(),
                graph.edge_count(),
                tag_index.len()
            )),
        );

        tracing::info!(
            root = %root.display(),
            notes = manifest.notes.len(),
            links = graph.edge_count(),
            tags = tag_index.len(),
            "vault opened",
        );

        let audit = AuditLog::new(&root);
        audit.log(
            AuditEvent::VaultOpened,
            Some(&format!(
                "notes={} links={}",
                manifest.notes.len(),
                graph.edge_count()
            )),
        );

        Ok(Self {
            root,
            index,
            graph,
            manifest,
            audit,
            tag_index,
        })
    }

    /// Return the vault root path.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Read and parse a single note by its absolute file path.
    ///
    /// Uses the live file system — always reflects the latest saved content.
    /// If the note is in the manifest, uses the persistent NoteId.
    ///
    /// # Errors
    /// Returns [`VaultError::Io`] on any file-system error.
    #[instrument(skip(self))]
    pub async fn read_note(&self, path: &Path) -> Result<Note, VaultError> {
        let body = tokio::fs::read_to_string(path).await?;
        let meta = tokio::fs::metadata(path).await?;
        let modified_at = meta
            .modified()
            .map(chrono::DateTime::<chrono::Utc>::from)
            .unwrap_or_else(|_| chrono::Utc::now());

        // Use persistent ID from manifest if available.
        let id = self.manifest.get(path).map(|e| e.id).unwrap_or_default();

        self.audit
            .log(AuditEvent::NoteRead, Some(&path.display().to_string()));

        Ok(Note {
            id,
            path: path.to_path_buf(),
            body,
            frontmatter: Default::default(),
            modified_at,
            created_at: modified_at,
        })
    }

    /// Full-text search over note titles and bodies.
    ///
    /// Returns up to `limit` absolute file paths ranked by relevance.
    pub fn search_notes(&self, query: &str, limit: usize) -> Result<Vec<String>, VaultError> {
        self.audit.log(AuditEvent::SearchQuery, Some(query));
        self.index.search(query, limit)
    }

    /// Return a serialisable snapshot of the backlink graph.
    ///
    /// Enriches the snapshot with an `id_to_path` mapping built from the
    /// manifest so the frontend can resolve UUIDs to file paths.
    pub fn graph_snapshot(&self) -> GraphSnapshot {
        let mut snap = self.graph.snapshot();
        // Build UUID → absolute path from the manifest.
        for (path, entry) in &self.manifest.notes {
            snap.id_to_path.insert(entry.id.to_string(), path.clone());
        }
        snap
    }

    /// Validate `path` canonicalises within the vault root, return safe canonical path.
    ///
    /// Resolves any `..`, `.`, or symlink components in both `path` and the
    /// vault root using [`dunce::canonicalize`] (which strips the Windows UNC
    /// `\\?\` prefix when not strictly required). Returns the canonical
    /// [`PathBuf`] on success, or [`VaultError::PathTraversal`] otherwise.
    ///
    /// **TOCTOU-safe usage**: callers MUST use the returned canonical path for
    /// all subsequent file-system operations (read, write, metadata) instead
    /// of the original `path` argument. Otherwise an attacker could swap a
    /// symlink between validation and file access.
    ///
    /// Rejects:
    /// - Relative-path traversals (`vault/../../../etc/passwd`).
    /// - Symlinks pointing outside the vault.
    /// - Non-existent paths (canonicalize fails -> rejected).
    fn validate_path_in_vault(&self, path: &Path) -> Result<PathBuf, VaultError> {
        let canonical_path = dunce::canonicalize(path).map_err(|_| VaultError::PathTraversal {
            path: path.display().to_string(),
            root: self.root.display().to_string(),
        })?;
        let canonical_root = dunce::canonicalize(&self.root).unwrap_or_else(|_| self.root.clone());
        if !canonical_path.starts_with(&canonical_root) {
            return Err(VaultError::PathTraversal {
                path: path.display().to_string(),
                root: self.root.display().to_string(),
            });
        }
        Ok(canonical_path)
    }

    /// Re-index a single note after it changed on disk using the default extractor.
    ///
    /// Convenience method that calls [`reindex_file_with_extractor`] with the
    /// default [`SimpleWikilinkExtractor`] from `forge-core`.
    ///
    /// Reads the file, updates the Tantivy index, updates the manifest,
    /// and rebuilds wikilinks for this note in the graph.
    #[instrument(skip(self))]
    pub fn reindex_file(&mut self, path: &Path) -> Result<(), VaultError> {
        self.reindex_file_with_extractor(path, &SimpleWikilinkExtractor)
    }

    /// Re-index a single note after it changed on disk with a custom extractor.
    ///
    /// Reads the file, updates the Tantivy index, updates the manifest,
    /// and rebuilds wikilinks for this note in the graph.
    ///
    /// # Arguments
    /// * `path` - Absolute path to the note file.
    /// * `extractor` - The [`WikilinkExtractor`] to use for link extraction.
    ///
    /// # Errors
    /// Returns [`VaultError::PathTraversal`] if `path` canonicalises to a
    /// location outside the vault root.
    #[instrument(skip(self, extractor))]
    pub fn reindex_file_with_extractor(
        &mut self,
        path: &Path,
        extractor: &dyn WikilinkExtractor,
    ) -> Result<(), VaultError> {
        // Defense in depth: validate the path stays within the vault root
        // BEFORE any file-system read. The canonical safe path is used ONLY
        // for the actual FS reads (read_to_string + metadata) -- this is the
        // attack surface for TOCTOU (a symlink swap between validate and
        // read). For manifest/index/audit, we keep the original `path` to
        // preserve key consistency (canonicalize on Windows can return a
        // slightly different path -- e.g. case normalisation -- which would
        // create duplicate manifest entries on every reindex).
        let safe_path = self.validate_path_in_vault(path)?;

        let body = std::fs::read_to_string(&safe_path)?;
        let meta = std::fs::metadata(&safe_path)?;
        let modified_at = meta
            .modified()
            .map(chrono::DateTime::<chrono::Utc>::from)
            .unwrap_or_else(|_| chrono::Utc::now());
        let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);

        // Use existing ID from manifest or generate new.
        let id = self.manifest.get(path).map(|e| e.id).unwrap_or_default();

        let note = Note {
            id,
            path: path.to_path_buf(),
            body,
            frontmatter: Default::default(),
            modified_at,
            created_at: modified_at,
        };

        // Update Tantivy index (upsert keyed by note.path).
        self.index.index_single_note(&note)?;

        // Update tag index for this note.
        self.index_tags_for_path(&note.path.clone(), &note.body.clone());

        // Extract and cache wikilinks.
        let wikilinks = extract_wikilink_targets(&note.body, extractor);

        // Update manifest with cached wikilinks (key = original path for
        // consistency with the initial scan_md_files indexation).
        self.manifest.upsert(
            path,
            NoteEntry::with_wikilinks(id, mtime, wikilinks.clone()),
        );
        if let Err(e) = self.manifest.save(&self.root) {
            tracing::warn!(error = %e, "failed to save manifest after reindex");
        }

        // Rebuild wikilinks for this note.
        self.graph.remove_note_edges(note.id);
        self.graph.add_note(note.id);

        let name_to_id = graph_builder::build_name_to_id(&self.manifest);
        for target in &wikilinks {
            if let Some(&target_id) = name_to_id.get(target.as_str()) {
                self.graph.add_link(note.id, target_id);
            }
        }

        self.audit
            .log(AuditEvent::NoteSaved, Some(&path.display().to_string()));
        tracing::info!(path = %path.display(), "note re-indexed");
        Ok(())
    }

    /// Remove a deleted note from the index, manifest, and graph.
    #[instrument(skip(self))]
    pub fn remove_file(&mut self, path: &Path) -> Result<(), VaultError> {
        self.index.remove_by_path(path)?;

        // Remove from graph if we know the ID.
        if let Some(entry) = self.manifest.get(path) {
            self.graph.remove_note_edges(entry.id);
        }

        // Remove from tag index.
        self.remove_tags_for_path(&path.to_path_buf());

        self.manifest.remove(path);
        if let Err(e) = self.manifest.save(&self.root) {
            tracing::warn!(error = %e, "failed to save manifest after remove");
        }

        self.audit
            .log(AuditEvent::NoteDeleted, Some(&path.display().to_string()));
        tracing::info!(path = %path.display(), "note removed from index");
        Ok(())
    }

    /// Return the relative paths of every `.md` file in the vault, sorted alphabetically.
    ///
    /// Reads from the in-memory manifest cache — O(n) with zero disk I/O.
    /// Hidden directories are excluded (they are never indexed into the manifest).
    pub fn list_note_paths(&self) -> Result<Vec<String>, VaultError> {
        let mut relative: Vec<String> = self
            .manifest
            .notes
            .keys()
            .filter_map(|abs| {
                Path::new(abs)
                    .strip_prefix(&self.root)
                    .ok()
                    .map(|rel| rel.to_string_lossy().to_string())
            })
            .collect();
        relative.sort_unstable();
        Ok(relative)
    }

    /// Resolve a wikilink target to an absolute file path.
    ///
    /// Performs case-insensitive matching on the file stem (without extension).
    /// Supports `[[target]]`, `[[target#heading]]` (heading ignored for path
    /// resolution), and `[[folder/target]]` (relative path matching).
    ///
    /// Returns `None` if no matching note is found in the vault.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let path = store.resolve_wikilink("my note");
    /// // Returns Some("/vault/subfolder/My Note.md") if a match exists
    /// ```
    pub fn resolve_wikilink(&self, target: &str) -> Option<String> {
        // Strip heading fragment: [[note#heading]] → "note"
        let target_stem = target.split('#').next().unwrap_or(target).trim();

        if target_stem.is_empty() {
            return None;
        }

        let target_lower = target_stem.to_lowercase();

        // Strategy 1: exact stem match (case-insensitive, most common)
        for path_str in self.manifest.notes.keys() {
            let stem = Path::new(path_str)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_lowercase();
            if stem == target_lower {
                return Some(path_str.clone());
            }
        }

        // Strategy 2: relative path match — "folder/note" matches "folder/note.md"
        if target_lower.contains('/') || target_lower.contains('\\') {
            let normalised = target_lower.replace('\\', "/");
            for path_str in self.manifest.notes.keys() {
                let rel = Path::new(path_str)
                    .strip_prefix(&self.root)
                    .ok()
                    .map(|p| p.to_string_lossy().to_lowercase().replace('\\', "/"))
                    .unwrap_or_default();
                // Match with or without .md extension
                if rel == normalised || rel == format!("{normalised}.md") {
                    return Some(path_str.clone());
                }
            }
        }

        None
    }

    // ── Tag index ─────────────────────────────────────────────────────────

    /// Returns all distinct tags present in the vault, sorted alphabetically.
    ///
    /// Tags are normalised (lowercase, trimmed) by [`forge_core::parse_frontmatter`].
    pub fn list_tags(&self) -> Vec<String> {
        let mut tags: Vec<String> = self.tag_index.keys().cloned().collect();
        tags.sort();
        tags
    }

    /// Returns absolute paths of all notes tagged with `tag` (case-insensitive).
    ///
    /// Returns an empty `Vec` if the tag does not exist in the vault.
    pub fn notes_by_tag(&self, tag: &str) -> Vec<PathBuf> {
        let normalised = tag.trim().to_lowercase();
        self.tag_index
            .get(&normalised)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Extract tags from a note body and upsert them into `self.tag_index`.
    fn index_tags_for_path(&mut self, path: &PathBuf, body: &str) {
        // Remove any existing tag associations for this path first.
        for set in self.tag_index.values_mut() {
            set.remove(path);
        }
        // Remove empty sets to keep the index clean.
        self.tag_index.retain(|_, set| !set.is_empty());

        // Parse tags from frontmatter and re-index.
        let fm = forge_core::parse_frontmatter(body);
        for tag in fm.tags {
            self.tag_index.entry(tag).or_default().insert(path.clone());
        }
    }

    /// Remove all tag associations for `path` (called on file deletion).
    fn remove_tags_for_path(&mut self, path: &PathBuf) {
        for set in self.tag_index.values_mut() {
            set.remove(path);
        }
        self.tag_index.retain(|_, set| !set.is_empty());
    }
}

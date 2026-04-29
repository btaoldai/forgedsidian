//! Incremental indexing and manifest updates.
//!
//! Handles applying a vault diff (added/modified/deleted files) to the Tantivy
//! index and manifest. This module bridges scanning decisions with actual
//! index mutations.

use crate::{
    index::VaultIndex,
    manifest::{Manifest, NoteEntry, VaultDiff},
    scan::extract_wikilink_targets,
    VaultError,
};
use forge_core::{Note, WikilinkExtractor};
use std::{path::Path, time::SystemTime};

/// Read a single note from disk into a [`Note`] with a fresh [`NoteId`].
pub(crate) fn read_single_note(path: &Path) -> Result<Note, VaultError> {
    use forge_core::NoteId;

    let body = std::fs::read_to_string(path)?;
    let meta = std::fs::metadata(path)?;
    let modified_at = meta
        .modified()
        .map(chrono::DateTime::<chrono::Utc>::from)
        .unwrap_or_else(|_| chrono::Utc::now());
    Ok(Note {
        id: NoteId::new(),
        path: path.to_path_buf(),
        body,
        frontmatter: Default::default(),
        modified_at,
        created_at: modified_at,
    })
}

/// Read multiple paths into `Note` objects.
pub(crate) fn read_notes_from_paths(paths: &[std::path::PathBuf]) -> Result<Vec<Note>, VaultError> {
    let mut notes = Vec::with_capacity(paths.len());
    for path in paths {
        notes.push(read_single_note(path)?);
    }
    Ok(notes)
}

/// Apply an incremental diff: index new/modified files, remove deleted.
///
/// # Arguments
/// * `index` - The Tantivy index to update.
/// * `manifest` - The in-memory manifest to update.
/// * `diff` - The diff (added/modified/deleted files) to apply.
/// * `extractor` - A [`WikilinkExtractor`] to extract wikilinks from note bodies.
pub(crate) fn apply_diff(
    index: &VaultIndex,
    manifest: &mut Manifest,
    diff: &VaultDiff,
    extractor: &dyn WikilinkExtractor,
) -> Result<(), VaultError> {
    // Index added files.
    for path in &diff.added {
        let note = read_single_note(path)?;
        index.index_single_note(&note)?;
        let mtime = std::fs::metadata(path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let wikilinks = extract_wikilink_targets(&note.body, extractor);
        manifest.upsert(path, NoteEntry::with_wikilinks(note.id, mtime, wikilinks));
    }

    // Re-index modified files (upsert: delete old + add new).
    for path in &diff.modified {
        let note = read_single_note(path)?;
        index.index_single_note(&note)?;
        let mtime = std::fs::metadata(path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let id = manifest.get(path).map(|e| e.id).unwrap_or(note.id);
        let wikilinks = extract_wikilink_targets(&note.body, extractor);
        manifest.upsert(path, NoteEntry::with_wikilinks(id, mtime, wikilinks));
    }

    // Remove deleted files.
    for path in &diff.deleted {
        index.remove_by_path(path)?;
        manifest.remove(path);
    }

    tracing::info!(
        added = diff.added.len(),
        modified = diff.modified.len(),
        deleted = diff.deleted.len(),
        "incremental index update complete",
    );

    Ok(())
}

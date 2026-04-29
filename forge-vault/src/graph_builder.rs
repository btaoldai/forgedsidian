//! Backlink graph construction from manifest.
//!
//! Rebuilds the [`NoteGraph`] purely from the manifest cache — zero file I/O.
//! Each note's wikilinks are pre-extracted and cached, so graph reconstruction
//! is O(notes + links) without touching the disk.

use crate::{manifest::Manifest, VaultError};
use forge_core::NoteId;
use forge_graph::NoteGraph;
use std::{collections::HashMap, path::Path};

/// Build the backlink graph purely from the manifest cache — zero file I/O.
///
/// Each `NoteEntry` stores its pre-extracted wikilink targets, so we only
/// need to resolve them against the name-to-id map.
pub(crate) fn build_graph(_root: &Path, manifest: &Manifest) -> Result<NoteGraph, VaultError> {
    // Build name-to-id map from manifest.
    let name_to_id: HashMap<String, NoteId> = manifest
        .notes
        .iter()
        .filter_map(|(path_str, entry)| {
            Path::new(path_str)
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|stem| (stem.to_lowercase(), entry.id))
        })
        .collect();

    let mut graph = NoteGraph::new();

    // Add all known notes as nodes.
    for entry in manifest.notes.values() {
        graph.add_note(entry.id);
    }

    // Resolve cached wikilinks — no file reads needed.
    for entry in manifest.notes.values() {
        for target in &entry.wikilinks {
            if let Some(&target_id) = name_to_id.get(target.as_str()) {
                graph.add_link(entry.id, target_id);
            }
        }
    }

    Ok(graph)
}

/// Build a lowercase-stem -> NoteId map from the manifest.
///
/// Much faster than reading from disk since it avoids I/O.
pub(crate) fn build_name_to_id(manifest: &Manifest) -> HashMap<String, NoteId> {
    manifest
        .notes
        .iter()
        .filter_map(|(path_str, entry)| {
            Path::new(path_str)
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|stem| (stem.to_lowercase(), entry.id))
        })
        .collect()
}

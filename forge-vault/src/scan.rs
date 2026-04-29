//! Scanning and diffing vault on-disk against manifest.
//!
//! Provides [`scan_md_files`] to recursively collect `*.md` files from the vault
//! directory (excluding hidden directories), and [`extract_wikilink_targets`] to
//! extract wikilink references from note bodies for graph construction.

use crate::VaultError;
use forge_core::{Link, WikilinkExtractor};
use std::path::{Path, PathBuf};

/// Recursively collect every `*.md` file under `root`.
///
/// Hidden directories (names starting with `.`) are skipped — this excludes
/// `.forge-index`, `.git`, `.obsidian`, etc.
pub fn scan_md_files(root: &Path) -> Result<Vec<PathBuf>, VaultError> {
    let mut out = Vec::new();
    scan_dir(root, &mut out)?;
    Ok(out)
}

fn scan_dir(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), VaultError> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();

        if path.is_dir() {
            // Skip hidden directories.
            if !name.to_string_lossy().starts_with('.') {
                scan_dir(&path, out)?;
            }
        } else if path.extension().is_some_and(|ext| ext == "md") {
            out.push(path);
        }
    }
    Ok(())
}

/// Extract lowercase wikilink target stems from a Markdown body.
///
/// Uses the provided `extractor` to parse links and filters for `Link::Wikilink`
/// variants, converting targets to lowercase for case-insensitive matching.
///
/// # Arguments
/// * `body` - The Markdown text to scan for wikilinks.
/// * `extractor` - A concrete [`WikilinkExtractor`] implementation (e.g. `PulldownWikilinkExtractor`).
///
/// # Returns
/// A `Vec<String>` of lowercase wikilink targets. Non-wikilink entries are filtered out.
pub fn extract_wikilink_targets(body: &str, extractor: &dyn WikilinkExtractor) -> Vec<String> {
    extractor
        .extract(body)
        .into_iter()
        .filter_map(|link| match link {
            Link::Wikilink { target, .. } => Some(target.to_lowercase()),
            _ => None,
        })
        .collect()
}

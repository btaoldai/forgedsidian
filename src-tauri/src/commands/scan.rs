//! Directory scanning commands with `.forgeignore` support.
//!
//! Provides `list_all_files` and `list_folders` — the two endpoints that power
//! the sidebar file tree.  Both respect `.forgeignore` to exclude build artifact
//! directories (`target/`, `node_modules/`, etc.).

use super::ForgeState;
use std::collections::HashSet;
use tauri::State;

// ---------------------------------------------------------------------------
// .forgeignore parser
// ---------------------------------------------------------------------------

/// Load the `.forgeignore` file from the vault root.
///
/// Each non-empty, non-comment line is a directory name to skip during scans.
/// Built-in defaults (`target`, `node_modules`) are always applied even without
/// a `.forgeignore` file, so Rust build artifacts never pollute the sidebar.
///
/// # Format
/// ```text
/// # Rust build artifacts (default -- always ignored)
/// target
/// # Node / JS
/// node_modules
/// # Custom entries
/// my-generated-dir
/// ```
fn load_forge_ignore(vault_root: &std::path::Path) -> HashSet<String> {
    let mut ignore: HashSet<String> = ["target", "node_modules"]
        .iter()
        .map(|s| s.to_string())
        .collect();

    let path = vault_root.join(".forgeignore");
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                ignore.insert(trimmed.to_string());
            }
        }
    }

    ignore
}

// ---------------------------------------------------------------------------
// Recursive scanners
// ---------------------------------------------------------------------------

/// Recursively collect all non-hidden file paths relative to root.
///
/// Skips directories that start with `.` or appear in `ignore`.
fn scan_all_files_recursive(
    base: &std::path::Path,
    dir: &std::path::Path,
    out: &mut Vec<String>,
    ignore: &HashSet<String>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if name_str.starts_with('.') {
            continue;
        }

        // Security: skip symlinks to prevent traversal outside the vault.
        if let Ok(meta) = std::fs::symlink_metadata(&path) {
            if meta.is_symlink() {
                continue;
            }
        }

        if path.is_dir() {
            if ignore.contains(name_str.as_ref()) {
                continue;
            }
            scan_all_files_recursive(base, &path, out, ignore)?;
        } else if let Ok(rel) = path.strip_prefix(base) {
            out.push(rel.to_string_lossy().to_string());
        }
    }
    Ok(())
}

/// Recursively collect non-hidden directory paths relative to root.
///
/// Skips directories that start with `.` or appear in `ignore`.
fn scan_folders_recursive(
    base: &std::path::Path,
    dir: &std::path::Path,
    out: &mut Vec<String>,
    ignore: &HashSet<String>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Security: skip symlinks.
        if let Ok(meta) = std::fs::symlink_metadata(&path) {
            if meta.is_symlink() {
                continue;
            }
        }

        if path.is_dir() && !name_str.starts_with('.') {
            if ignore.contains(name_str.as_ref()) {
                continue;
            }
            if let Ok(rel) = path.strip_prefix(base) {
                out.push(rel.to_string_lossy().to_string());
            }
            scan_folders_recursive(base, &path, out, ignore)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Return all non-hidden file paths (relative to vault root), sorted.
///
/// Unlike `list_notes` (which returns only `.md` files from the Tantivy index),
/// this returns ALL files: `.txt`, `.rs`, `.py`, `.pdf`, images, etc.
/// Hidden directories (starting with `.`) and `.forgeignore` entries are excluded.
#[tauri::command]
pub async fn list_all_files(state: State<'_, ForgeState>) -> Result<Vec<String>, String> {
    let vault_path = {
        let guard = state.vault_path.lock().await;
        guard.clone().ok_or("vault not open")?
    };

    let root = vault_path.clone();
    let files = tokio::task::spawn_blocking(move || -> Result<Vec<String>, String> {
        let ignore = load_forge_ignore(&root);
        let mut out = Vec::new();
        scan_all_files_recursive(&root, &root, &mut out, &ignore)
            .map_err(|e| format!("failed to scan files: {}", e))?;
        out.sort();
        Ok(out)
    })
    .await
    .map_err(|e| format!("spawn_blocking join error: {}", e))??;

    Ok(files)
}

/// Return all non-hidden directory paths (relative to vault root), sorted.
///
/// Used by the sidebar to display folders that may not contain any `.md` files yet.
/// Respects `.forgeignore` — build artifact directories are excluded.
#[tauri::command]
pub async fn list_folders(state: State<'_, ForgeState>) -> Result<Vec<String>, String> {
    let vault_path = {
        let guard = state.vault_path.lock().await;
        guard.clone().ok_or("vault not open")?
    };

    let root = vault_path.clone();
    let folders = tokio::task::spawn_blocking(move || -> Result<Vec<String>, String> {
        let ignore = load_forge_ignore(&root);
        let mut out = Vec::new();
        scan_folders_recursive(&root, &root, &mut out, &ignore)
            .map_err(|e| format!("failed to scan folders: {}", e))?;
        out.sort();
        Ok(out)
    })
    .await
    .map_err(|e| format!("spawn_blocking join error: {}", e))??;

    Ok(folders)
}

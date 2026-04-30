//! Create operations for notes and folders.

use super::super::{reject_traversal, ForgeState};
use std::path::PathBuf;
use tauri::{AppHandle, Manager, State};

/// Create a new empty note at the given relative path.
///
/// Returns the relative path of the created note.
/// Reindex is scheduled in background (same pattern as `save_note`).
#[tauri::command]
pub async fn create_note(
    folder: String,
    name: String,
    state: State<'_, ForgeState>,
    app: AppHandle,
) -> Result<String, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("note name cannot be empty".to_string());
    }

    let file_name = if name.ends_with(".md") {
        name.clone()
    } else {
        format!("{}.md", name)
    };

    let rel_path = if folder.is_empty() {
        file_name.clone()
    } else {
        let parent = PathBuf::from(folder.trim_end_matches(['/', '\\']));
        parent.join(&file_name).to_string_lossy().to_string()
    };

    let path = PathBuf::from(&rel_path);
    reject_traversal(&path)?;

    let vault_path = {
        let guard = state.vault_path.lock().await;
        guard.clone().ok_or("vault not open")?
    };

    // Canonicalize the vault root (must exist, since vault is open).
    // This is the "ground truth" for the path-traversal check below.
    let canonical_vault = dunce::canonicalize(&vault_path)
        .map_err(|e| format!("vault canonicalization failed: {}", e))?;

    let file_path = vault_path.join(&rel_path);

    if file_path.exists() {
        return Err(format!("note already exists: {}", rel_path));
    }

    // Create parent directories if they don't exist yet. We do this BEFORE
    // canonicalizing because dunce::canonicalize requires the path to exist.
    let parent = file_path
        .parent()
        .ok_or_else(|| "invalid path: no parent".to_string())?
        .to_path_buf();
    tokio::fs::create_dir_all(&parent)
        .await
        .map_err(|e| format!("failed to create directories: {}", e))?;

    // Now the parent exists -> canonicalize it and verify it lives inside
    // the vault. Defense-in-depth: blocks symlink-based escapes that
    // `reject_traversal` cannot detect (a folder argument that is itself a
    // symlink pointing outside the vault).
    let canonical_parent =
        dunce::canonicalize(&parent).map_err(|e| format!("path canonicalization failed: {}", e))?;
    if !canonical_parent.starts_with(&canonical_vault) {
        return Err(format!(
            "path traversal rejected: {} is outside vault root",
            file_path.display()
        ));
    }

    // Reconstruct the safe file path from the canonical parent + filename
    // so the actual write operation cannot be redirected by a TOCTOU swap.
    let safe_file_path = canonical_parent.join(&file_name);

    let title = file_name.trim_end_matches(".md");
    let content = format!("# {}\n\n", title);
    tokio::fs::write(&safe_file_path, &content)
        .await
        .map_err(|e| format!("failed to create note: {}", e))?;

    tracing::info!(path = %safe_file_path.display(), "note created");

    // Background reindex (same pattern as save_note).
    tokio::spawn(async move {
        let state: tauri::State<ForgeState> = app.state();
        let mut guard = state.store.lock().await;
        if let Some(store) = guard.as_mut() {
            if let Err(e) = store.reindex_file(&safe_file_path) {
                tracing::warn!(
                    error = %e,
                    path = %safe_file_path.display(),
                    "background reindex failed"
                );
            }
        }
    });

    Ok(rel_path)
}

/// Create a new folder at the given relative path.
///
/// Returns the relative path of the created folder.
#[tauri::command]
pub async fn create_folder(
    parent: String,
    name: String,
    state: State<'_, ForgeState>,
) -> Result<String, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("folder name cannot be empty".to_string());
    }

    let rel_path = if parent.is_empty() {
        name
    } else {
        let parent_dir = PathBuf::from(parent.trim_end_matches(['/', '\\']));
        parent_dir.join(&name).to_string_lossy().to_string()
    };

    let path = PathBuf::from(&rel_path);
    reject_traversal(&path)?;

    let vault_path = {
        let guard = state.vault_path.lock().await;
        guard.clone().ok_or("vault not open")?
    };

    // Canonicalize the vault root for the path-traversal check below.
    let canonical_vault = dunce::canonicalize(&vault_path)
        .map_err(|e| format!("vault canonicalization failed: {}", e))?;

    let dir_path = vault_path.join(&rel_path);

    if dir_path.exists() {
        return Err(format!("folder already exists: {}", rel_path));
    }

    // Create the folder chain, then canonicalize and verify the result
    // stays inside the vault. If a parent component is a symlink pointing
    // outside the vault, `create_dir_all` would silently follow it; we
    // detect this AFTER creation and roll back.
    tokio::fs::create_dir_all(&dir_path)
        .await
        .map_err(|e| format!("failed to create folder: {}", e))?;

    let canonical_dir = match dunce::canonicalize(&dir_path) {
        Ok(p) => p,
        Err(e) => {
            // Roll back the partial creation (best-effort).
            let _ = tokio::fs::remove_dir_all(&dir_path).await;
            return Err(format!("path canonicalization failed: {}", e));
        }
    };
    if !canonical_dir.starts_with(&canonical_vault) {
        // Roll back: this dir was created outside the vault via a symlink.
        let _ = tokio::fs::remove_dir_all(&dir_path).await;
        return Err(format!(
            "path traversal rejected: {} is outside vault root",
            dir_path.display()
        ));
    }

    tracing::info!(path = %canonical_dir.display(), "folder created");
    Ok(rel_path)
}

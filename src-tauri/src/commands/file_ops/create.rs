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

    let file_path = vault_path.join(&rel_path);

    if file_path.exists() {
        return Err(format!("note already exists: {}", rel_path));
    }

    if let Some(parent) = file_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("failed to create directories: {}", e))?;
    }

    let title = file_name.trim_end_matches(".md");
    let content = format!("# {}\n\n", title);
    tokio::fs::write(&file_path, &content)
        .await
        .map_err(|e| format!("failed to create note: {}", e))?;

    tracing::info!(path = %file_path.display(), "note created");

    // Background reindex (same pattern as save_note).
    tokio::spawn(async move {
        let state: tauri::State<ForgeState> = app.state();
        let mut guard = state.store.lock().await;
        if let Some(store) = guard.as_mut() {
            if let Err(e) = store.reindex_file(&file_path) {
                tracing::warn!(
                    error = %e,
                    path = %file_path.display(),
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

    let dir_path = vault_path.join(&rel_path);

    if dir_path.exists() {
        return Err(format!("folder already exists: {}", rel_path));
    }

    tokio::fs::create_dir_all(&dir_path)
        .await
        .map_err(|e| format!("failed to create folder: {}", e))?;

    tracing::info!(path = %dir_path.display(), "folder created");
    Ok(rel_path)
}

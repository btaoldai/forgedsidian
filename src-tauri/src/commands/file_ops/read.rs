//! Read and save operations for files and notes.

use super::super::{reject_traversal, ForgeState};
use std::path::PathBuf;
use tauri::{AppHandle, Manager, State};

/// Read any text file from the vault by its absolute path.
///
/// Unlike `get_note` (which reads from the index), this reads raw file content
/// directly from disk. Used for non-.md files (.rs, .py, .txt, etc.).
#[tauri::command]
pub async fn read_file(path: String, state: State<'_, ForgeState>) -> Result<String, String> {
    let vault_path = {
        let guard = state.vault_path.lock().await;
        guard.clone().ok_or("vault not open")?
    };

    let file_path = PathBuf::from(&path);

    // Security: reject explicit traversal components.
    reject_traversal(&file_path)?;

    // Security: reject symlinks (could point outside vault).
    if let Ok(meta) = std::fs::symlink_metadata(&file_path) {
        if meta.is_symlink() {
            return Err("path validation failed".to_string());
        }
    }

    // Security: ensure the resolved file path is within the vault root.
    let canonical =
        dunce::canonicalize(&file_path).map_err(|_| "path validation failed".to_string())?;
    let vault_canonical =
        dunce::canonicalize(&vault_path).map_err(|_| "vault path validation failed".to_string())?;

    if !canonical.starts_with(&vault_canonical) {
        return Err("path validation failed".to_string());
    }

    // Read file content (limit to 5MB to prevent loading huge binaries).
    let metadata = tokio::fs::metadata(&canonical)
        .await
        .map_err(|e| format!("failed to read file metadata: {e}"))?;

    const MAX_FILE_SIZE: u64 = 5 * 1024 * 1024;
    if metadata.len() > MAX_FILE_SIZE {
        return Err(format!(
            "file too large to preview ({:.1} MB, max {} MB)",
            metadata.len() as f64 / (1024.0 * 1024.0),
            MAX_FILE_SIZE / (1024 * 1024)
        ));
    }

    let content = tokio::fs::read_to_string(&canonical)
        .await
        .map_err(|e| format!("failed to read file (may be binary): {e}"))?;

    Ok(content)
}

/// Save a note to disk.
///
/// The note_id should be the relative file path (e.g., "folder/note.md").
/// Writes the content to disk **immediately** (data safety), then schedules
/// a Tantivy reindex in the background so the IPC response returns fast.
///
/// The VaultWatcher may also trigger a reindex for the same file; this is
/// harmless because `reindex_file` is idempotent.
#[tauri::command]
pub async fn save_note(
    note_id: String,
    body: String,
    state: State<'_, ForgeState>,
    app: AppHandle,
) -> Result<(), String> {
    let path = PathBuf::from(&note_id);
    reject_traversal(&path)?;

    let vault_path = {
        let guard = state.vault_path.lock().await;
        guard.clone().ok_or("vault not open")?
    };

    let file_path = vault_path.join(&note_id);

    // Security: ensure the parent directory exists and the resolved path stays
    // inside the vault.  We canonicalize the parent (the file itself may not
    // exist yet) and verify containment — same defence-in-depth pattern used by
    // read_file and get_note.
    let parent = file_path
        .parent()
        .ok_or_else(|| "path validation failed".to_string())?;
    let parent_canonical =
        dunce::canonicalize(parent).map_err(|_| "path validation failed".to_string())?;
    let vault_canonical =
        dunce::canonicalize(&vault_path).map_err(|_| "vault path validation failed".to_string())?;

    if !parent_canonical.starts_with(&vault_canonical) {
        return Err("path validation failed".to_string());
    }

    // Resolved path used for all subsequent operations.
    let file_path = parent_canonical.join(
        file_path
            .file_name()
            .ok_or_else(|| "path validation failed".to_string())?,
    );

    // Write to disk immediately — data safety first.
    tokio::fs::write(&file_path, &body)
        .await
        .map_err(|e| format!("failed to write note: {e}"))?;

    tracing::info!(path = %file_path.display(), "note saved to disk");

    // Reindex in background — don't block the IPC response.
    // The store mutex is only held inside the spawned task, freeing the
    // command handler to return "Saved" to the frontend immediately.
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

    Ok(())
}

//! Delete operations for folders.

use super::super::{reject_traversal, ForgeState};
use super::move_ops::walkdir_count;
use std::path::PathBuf;
use tauri::State;

/// Delete a folder and all its contents.
///
/// Requires explicit confirmation (`confirm` must be `true`).
/// Returns the number of files deleted.
#[tauri::command]
pub async fn delete_folder(
    path: String,
    confirm: bool,
    state: State<'_, ForgeState>,
) -> Result<String, String> {
    if !confirm {
        return Err("deletion not confirmed".to_string());
    }

    let path = path.trim().to_string();
    if path.is_empty() {
        return Err("folder path cannot be empty".to_string());
    }

    let rel = PathBuf::from(&path);
    reject_traversal(&rel)?;

    let vault_path = {
        let guard = state.vault_path.lock().await;
        guard.clone().ok_or("vault not open")?
    };

    let dir_path = vault_path.join(&path);

    if !dir_path.exists() {
        return Err(format!("folder does not exist: {path}"));
    }
    if !dir_path.is_dir() {
        return Err(format!("path is not a folder: {path}"));
    }

    // Security: ensure the path is within the vault.
    let canonical =
        dunce::canonicalize(&dir_path).map_err(|e| format!("failed to resolve path: {e}"))?;
    let vault_canonical = dunce::canonicalize(&vault_path)
        .map_err(|e| format!("failed to resolve vault path: {e}"))?;

    if !canonical.starts_with(&vault_canonical) || canonical == vault_canonical {
        return Err("cannot delete vault root or paths outside vault".to_string());
    }

    let count = walkdir_count(&dir_path);

    tokio::fs::remove_dir_all(&dir_path)
        .await
        .map_err(|e| format!("failed to delete folder: {e}"))?;

    tracing::info!(path = %dir_path.display(), files = count, "folder deleted");
    Ok(format!("Deleted {count} files"))
}

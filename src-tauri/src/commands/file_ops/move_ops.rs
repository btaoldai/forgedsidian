//! Move operations for files and folders.

use super::super::ForgeState;
use std::path::{Component, PathBuf};
use tauri::State;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Count files recursively in a directory (synchronous, for pre-delete info).
pub(crate) fn walkdir_count(dir: &std::path::Path) -> usize {
    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                count += walkdir_count(&path);
            } else {
                count += 1;
            }
        }
    }
    count
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Move a file from one relative path to another within the vault.
///
/// Both `from` and `to` are relative to the vault root.
/// `to` is the destination **directory** — the file name is preserved.
/// Returns the new relative path of the moved file.
#[tauri::command]
pub async fn move_file(
    from: String,
    to: String,
    state: State<'_, ForgeState>,
) -> Result<String, String> {
    let from = from.trim().to_string();
    let to = to.trim().to_string();

    if from.is_empty() {
        return Err("source path cannot be empty".to_string());
    }

    // Validate no path traversal
    for component in PathBuf::from(&from).components() {
        if matches!(component, Component::ParentDir) {
            return Err("path traversal not allowed in source".to_string());
        }
    }
    for component in PathBuf::from(&to).components() {
        if matches!(component, Component::ParentDir) {
            return Err("path traversal not allowed in destination".to_string());
        }
    }

    let vault_path = {
        let guard = state.vault_path.lock().await;
        guard.clone().ok_or("vault not open")?
    };

    let src_path = vault_path.join(&from);
    if !src_path.exists() {
        return Err(format!("source file does not exist: {}", from));
    }
    if !src_path.is_file() {
        return Err(format!("source is not a file: {}", from));
    }

    // Security: ensure source is within vault
    let src_canonical = src_path
        .canonicalize()
        .map_err(|e| format!("failed to resolve source: {}", e))?;
    let vault_canonical = vault_path
        .canonicalize()
        .map_err(|e| format!("failed to resolve vault: {}", e))?;
    if !src_canonical.starts_with(&vault_canonical) {
        return Err("source is outside vault".to_string());
    }

    // Destination directory (relative). Empty string = vault root.
    let dest_dir = if to.is_empty() {
        vault_path.clone()
    } else {
        vault_path.join(&to)
    };

    if !dest_dir.exists() {
        tokio::fs::create_dir_all(&dest_dir)
            .await
            .map_err(|e| format!("failed to create destination directory: {}", e))?;
    }

    let file_name = src_path
        .file_name()
        .ok_or("source has no file name")?
        .to_string_lossy()
        .to_string();

    let dest_path = dest_dir.join(&file_name);

    if dest_path.exists() {
        return Err(format!(
            "destination already exists: {}",
            dest_path.display()
        ));
    }

    // Security: ensure destination is within vault
    let dest_dir_canonical = dest_dir
        .canonicalize()
        .map_err(|e| format!("failed to resolve destination: {}", e))?;
    if !dest_dir_canonical.starts_with(&vault_canonical) {
        return Err("destination is outside vault".to_string());
    }

    tokio::fs::rename(&src_path, &dest_path)
        .await
        .map_err(|e| format!("failed to move file: {}", e))?;

    let new_rel = dest_path
        .strip_prefix(&vault_path)
        .map_err(|_| "failed to compute relative path")?
        .to_string_lossy()
        .to_string();

    tracing::info!(from = %from, to = %new_rel, "file moved");
    Ok(new_rel)
}

/// Move a folder from one relative path to another within the vault.
///
/// Both `from` and `to` are relative to the vault root.
/// `to` is the destination **parent directory** — the folder name is preserved.
/// Returns the new relative path of the moved folder.
#[tauri::command]
pub async fn move_folder(
    from: String,
    to: String,
    state: State<'_, ForgeState>,
) -> Result<String, String> {
    let from = from.trim().to_string();
    let to = to.trim().to_string();

    if from.is_empty() {
        return Err("source path cannot be empty".to_string());
    }

    // Validate no path traversal
    for component in PathBuf::from(&from).components() {
        if matches!(component, Component::ParentDir) {
            return Err("path traversal not allowed in source".to_string());
        }
    }
    for component in PathBuf::from(&to).components() {
        if matches!(component, Component::ParentDir) {
            return Err("path traversal not allowed in destination".to_string());
        }
    }

    let vault_path = {
        let guard = state.vault_path.lock().await;
        guard.clone().ok_or("vault not open")?
    };

    let src_path = vault_path.join(&from);
    if !src_path.exists() {
        return Err(format!("source folder does not exist: {}", from));
    }
    if !src_path.is_dir() {
        return Err(format!("source is not a folder: {}", from));
    }

    // Security: ensure source is within vault and not the vault root
    let src_canonical = src_path
        .canonicalize()
        .map_err(|e| format!("failed to resolve source: {}", e))?;
    let vault_canonical = vault_path
        .canonicalize()
        .map_err(|e| format!("failed to resolve vault: {}", e))?;
    if !src_canonical.starts_with(&vault_canonical) || src_canonical == vault_canonical {
        return Err("cannot move vault root or paths outside vault".to_string());
    }

    // Destination parent directory. Empty string = vault root.
    let dest_parent = if to.is_empty() {
        vault_path.clone()
    } else {
        vault_path.join(&to)
    };

    // Prevent moving a folder into itself
    if let Ok(dest_canonical) = dest_parent.canonicalize() {
        if dest_canonical.starts_with(&src_canonical) {
            return Err("cannot move a folder into itself".to_string());
        }
    }

    let folder_name = src_path
        .file_name()
        .ok_or("source has no folder name")?
        .to_string_lossy()
        .to_string();

    let dest_path = dest_parent.join(&folder_name);

    if dest_path.exists() {
        return Err(format!(
            "destination already exists: {}",
            dest_path.display()
        ));
    }

    if !dest_parent.exists() {
        tokio::fs::create_dir_all(&dest_parent)
            .await
            .map_err(|e| format!("failed to create destination directory: {}", e))?;
    }

    // Security: ensure destination is within vault
    let dest_parent_canonical = dest_parent
        .canonicalize()
        .map_err(|e| format!("failed to resolve destination: {}", e))?;
    if !dest_parent_canonical.starts_with(&vault_canonical) {
        return Err("destination is outside vault".to_string());
    }

    tokio::fs::rename(&src_path, &dest_path)
        .await
        .map_err(|e| format!("failed to move folder: {}", e))?;

    let new_rel = dest_path
        .strip_prefix(&vault_path)
        .map_err(|_| "failed to compute relative path")?
        .to_string_lossy()
        .to_string();

    tracing::info!(from = %from, to = %new_rel, "folder moved");
    Ok(new_rel)
}

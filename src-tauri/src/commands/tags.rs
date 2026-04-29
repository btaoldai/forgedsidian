//! Tag query commands — list all tags and find notes by tag.

use super::ForgeState;
use tauri::State;

/// Return all distinct tags present in the vault, sorted alphabetically.
///
/// Returns an empty list if no vault is open or if no notes have tags.
#[tauri::command]
pub async fn list_tags(state: State<'_, ForgeState>) -> Result<Vec<String>, String> {
    let guard = state.store.lock().await;
    let store = guard.as_ref().ok_or("vault not open")?;
    Ok(store.list_tags())
}

/// Return relative paths (from vault root) of notes tagged with `tag`.
///
/// The `tag` argument is matched case-insensitively.
/// Returns an empty list if no vault is open or if no notes carry the tag.
#[tauri::command]
pub async fn notes_by_tag(
    tag: String,
    state: State<'_, ForgeState>,
) -> Result<Vec<String>, String> {
    if tag.trim().is_empty() {
        return Err("tag cannot be empty".to_string());
    }
    let vault_path = {
        let guard = state.vault_path.lock().await;
        guard.clone().ok_or("vault not open")?
    };
    let guard = state.store.lock().await;
    let store = guard.as_ref().ok_or("vault not open")?;
    let abs_paths = store.notes_by_tag(&tag);
    // Convert absolute paths to relative paths (relative to vault root).
    let rel_paths: Vec<String> = abs_paths
        .iter()
        .filter_map(|p| {
            p.strip_prefix(&vault_path)
                .ok()
                .map(|rel| rel.to_string_lossy().to_string())
        })
        .collect();
    Ok(rel_paths)
}

//! Vault lifecycle commands: open, pick, list notes, search, graph, canvas.

use super::{harden_vault_index_permissions, validate_vault_path, ForgeState};
use forge_graph::graph::GraphSnapshot;
use forge_vault::{
    store::VaultStore,
    watcher::{VaultEvent, VaultWatcher},
    ProgressStep,
};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

// ---------------------------------------------------------------------------
// Shared watcher spawner (used by open_vault & pick_and_open_vault)
// ---------------------------------------------------------------------------

/// Start the file watcher and spawn a background task that relays events to
/// the frontend via Tauri's event system.
async fn start_watcher(
    vault_path: &Path,
    state: &State<'_, ForgeState>,
    app: &AppHandle,
) -> Result<(), String> {
    let (watcher, mut rx) =
        VaultWatcher::start(vault_path).map_err(|e| format!("failed to start watcher: {}", e))?;
    *state.watcher.lock().await = Some(watcher);

    let app = app.clone();
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let state: tauri::State<ForgeState> = app.state();
            match &event {
                VaultEvent::Changed(path) => {
                    tracing::debug!(path = %path.display(), "watcher: file changed");
                    let mut guard = state.store.lock().await;
                    if let Some(store) = guard.as_mut() {
                        if let Err(e) = store.reindex_file(path) {
                            tracing::error!(error = %e, "incremental re-index failed");
                        }
                    }
                    drop(guard);
                    let _ = app.emit("vault:file-changed", path.display().to_string());
                }
                VaultEvent::Removed(path) => {
                    tracing::debug!(path = %path.display(), "watcher: file removed");
                    let mut guard = state.store.lock().await;
                    if let Some(store) = guard.as_mut() {
                        if let Err(e) = store.remove_file(path) {
                            tracing::error!(error = %e, "remove from index failed");
                        }
                    }
                    drop(guard);
                    let _ = app.emit("vault:file-removed", path.display().to_string());
                }
                VaultEvent::Renamed { from, to } => {
                    tracing::debug!(from = %from.display(), to = %to.display(), "watcher: file renamed");
                    let mut guard = state.store.lock().await;
                    if let Some(store) = guard.as_mut() {
                        let _ = store.remove_file(from);
                        let _ = store.reindex_file(to);
                    }
                    drop(guard);
                    let _ = app.emit("vault:file-changed", to.display().to_string());
                }
            }
        }
        tracing::info!("watcher event loop ended");
    });

    Ok(())
}

// ---------------------------------------------------------------------------
// Progress relay — spawn_blocking → async → Tauri emit
// ---------------------------------------------------------------------------

/// Open a vault while relaying progress events to the frontend via Tauri events.
///
/// `app.emit()` from inside `spawn_blocking` may not be delivered to the webview
/// on Windows because the blocking thread bypasses the Tokio event loop.
/// Instead, we pipe progress updates through an `mpsc` channel: the blocking
/// thread sends `ProgressStep` values, and a concurrent async task reads them
/// and calls `app.emit()` from the async runtime where delivery is guaranteed.
async fn open_vault_with_progress(vault_path: &str, app: &AppHandle) -> Result<VaultStore, String> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<ProgressStep>();

    // Relay task: reads from the channel and emits Tauri events.
    let app_relay = app.clone();
    let relay = tokio::spawn(async move {
        while let Some(step) = rx.recv().await {
            let _ = app_relay.emit("vault:indexing-progress", &step);
        }
    });

    // Blocking task: opens the vault, sends progress through the channel.
    let path_owned = vault_path.to_string();
    let store = tokio::task::spawn_blocking(move || {
        let progress: forge_vault::ProgressFn = Box::new(move |step: ProgressStep| {
            let _ = tx.send(step);
        });
        VaultStore::open_with_progress(
            &path_owned,
            &forge_core::SimpleWikilinkExtractor,
            Some(progress),
        )
    })
    .await
    .map_err(|e| format!("spawn_blocking join error: {}", e))?
    .map_err(|e| e.to_string())?;

    // tx is dropped when spawn_blocking completes → channel closes → relay ends.
    let _ = relay.await;

    Ok(store)
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Open (or switch to) a vault at the given path.
///
/// Scans all Markdown files, rebuilds the Tantivy index and the backlink graph.
/// Starts a file watcher that emits `vault:file-changed` events to the frontend
/// when Markdown files are created, modified or deleted.
///
/// **Security**: Path is validated to prevent traversal attacks.
#[tauri::command]
pub async fn open_vault(
    path: String,
    state: State<'_, ForgeState>,
    app: AppHandle,
) -> Result<(), String> {
    let vault_path = validate_vault_path(&path)?;

    let vault_path_str = vault_path
        .to_str()
        .ok_or_else(|| "vault path contains invalid UTF-8".to_string())?
        .to_string();

    let store = open_vault_with_progress(&vault_path_str, &app).await?;
    *state.store.lock().await = Some(store);
    *state.vault_path.lock().await = Some(vault_path.clone());

    harden_vault_index_permissions(&vault_path)?;
    start_watcher(&vault_path, &state, &app).await?;

    Ok(())
}

/// Open a native folder picker, then open the vault at the selected path.
///
/// Returns the selected vault path on success, or `None` if the user cancelled.
/// This is the primary entry point for the "Open vault" button in the UI.
#[tauri::command]
pub async fn pick_and_open_vault(
    state: State<'_, ForgeState>,
    app: AppHandle,
) -> Result<Option<String>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("Select a vault folder")
        .pick_folder(move |folder| {
            let _ = tx.send(folder);
        });

    let folder = rx.await.map_err(|_| "dialog channel closed")?;

    let folder_path = match folder {
        Some(fp) => fp,
        None => return Ok(None),
    };

    let path_str = folder_path
        .as_path()
        .ok_or("invalid folder path")?
        .to_string_lossy()
        .to_string();

    tracing::info!(path = %path_str, "pick_and_open_vault: folder selected");

    let vault_path = PathBuf::from(&path_str);

    let path_clone = path_str.clone();
    let store = open_vault_with_progress(&path_clone, &app).await?;
    *state.store.lock().await = Some(store);
    *state.vault_path.lock().await = Some(vault_path.clone());

    harden_vault_index_permissions(&vault_path)?;
    start_watcher(&vault_path, &state, &app).await?;

    Ok(Some(path_str))
}

/// Return every note path in the vault (relative to vault root), sorted alphabetically.
///
/// Used by the sidebar to display the file tree when no search query is active.
#[tauri::command]
pub async fn list_notes(state: State<'_, ForgeState>) -> Result<Vec<String>, String> {
    let t0 = std::time::Instant::now();
    let guard = state.store.lock().await;
    let store = guard.as_ref().ok_or("vault not open")?;
    let result = store.list_note_paths().map_err(|e| e.to_string());
    tracing::info!(
        cmd = "list_notes",
        elapsed_ms = t0.elapsed().as_millis() as u64,
        "IPC"
    );
    result
}

/// Return the raw Markdown content of a single note by its absolute file path.
///
/// **Security**: Path is validated to prevent traversal attacks. The provided
/// path must resolve within the vault root directory.
#[tauri::command]
pub async fn get_note(path: String, state: State<'_, ForgeState>) -> Result<String, String> {
    let t0 = std::time::Instant::now();

    // Security: validate path to prevent traversal attacks.
    let file_path = PathBuf::from(&path);
    super::reject_traversal(&file_path)?;

    // Security: reject symlinks (could point outside vault).
    if let Ok(meta) = std::fs::symlink_metadata(&file_path) {
        if meta.is_symlink() {
            return Err("path validation failed".to_string());
        }
    }

    // Security: ensure the resolved file path is within the vault root.
    let vault_path = {
        let guard = state.vault_path.lock().await;
        guard.clone().ok_or("vault not open")?
    };

    let canonical = dunce::canonicalize(&file_path)
        .map_err(|_| "path validation failed".to_string())?;
    let vault_canonical = dunce::canonicalize(&vault_path)
        .map_err(|_| "vault path validation failed".to_string())?;

    if !canonical.starts_with(&vault_canonical) {
        return Err("path validation failed".to_string());
    }

    let guard = state.store.lock().await;
    let store = guard.as_ref().ok_or("vault not open")?;
    let note = store
        .read_note(std::path::Path::new(&path))
        .await
        .map_err(|e| e.to_string())?;
    tracing::info!(
        cmd = "get_note",
        elapsed_ms = t0.elapsed().as_millis() as u64,
        "IPC"
    );
    Ok(note.body)
}

/// Full-text search across all indexed notes.
///
/// Returns up to 20 file paths ranked by relevance.
#[tauri::command]
pub async fn search_notes(
    query: String,
    state: State<'_, ForgeState>,
) -> Result<Vec<String>, String> {
    let t0 = std::time::Instant::now();
    let guard = state.store.lock().await;
    let store = guard.as_ref().ok_or("vault not open")?;
    let result = store.search_notes(&query, 20).map_err(|e| e.to_string());
    tracing::info!(
        cmd = "search_notes",
        query = %query,
        elapsed_ms = t0.elapsed().as_millis() as u64,
        "IPC",
    );
    result
}

/// Return a serialisable snapshot of the backlink graph.
///
/// Nodes are note UUIDs; edges are `(from_id, to_id)` wikilink pairs.
#[tauri::command]
pub async fn get_graph_snapshot(state: State<'_, ForgeState>) -> Result<GraphSnapshot, String> {
    let t0 = std::time::Instant::now();
    let guard = state.store.lock().await;
    let store = guard.as_ref().ok_or("vault not open")?;
    let snap = store.graph_snapshot();
    tracing::info!(
        cmd = "get_graph_snapshot",
        nodes = snap.nodes.len(),
        edges = snap.edges.len(),
        elapsed_ms = t0.elapsed().as_millis() as u64,
        "IPC",
    );
    Ok(snap)
}

/// Resolve a wikilink target to an absolute file path.
///
/// Performs case-insensitive stem matching against all notes in the vault.
/// Supports `[[target]]`, `[[target#heading]]` (heading stripped),
/// and `[[folder/target]]` (relative path matching).
///
/// Returns the absolute path if found, or `null` if no match exists.
#[tauri::command]
pub async fn resolve_wikilink(
    target: String,
    state: State<'_, ForgeState>,
) -> Result<Option<String>, String> {
    let t0 = std::time::Instant::now();
    let guard = state.store.lock().await;
    let store = guard.as_ref().ok_or("vault not open")?;
    let result = store.resolve_wikilink(&target);
    tracing::info!(
        cmd = "resolve_wikilink",
        target = %target,
        found = result.is_some(),
        elapsed_ms = t0.elapsed().as_millis() as u64,
        "IPC",
    );
    Ok(result)
}

/// Open a file or URL in the user's default application.
///
/// Cross-platform: delegates to `tauri-plugin-opener` which uses the OS
/// default handler (xdg-open on Linux, `open` on macOS, `start` on Windows).
/// Used for `.html` files in wikilinks and any non-`.md` file the app
/// cannot render natively.
///
/// **Security**: Path must be a valid local file within the vault.
/// Web URLs (http, https, javascript, data schemes, etc.) are rejected.
#[tauri::command]
pub async fn open_in_default_app(
    path: String,
    app: tauri::AppHandle,
    state: State<'_, ForgeState>,
) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let t0 = std::time::Instant::now();

    // Security: reject dangerous URL schemes.
    let lower_path = path.to_lowercase();
    let forbidden_schemes = ["http://", "https://", "javascript:", "data:", "file://"];
    for scheme in &forbidden_schemes {
        if lower_path.starts_with(scheme) {
            return Err(format!("URL scheme '{}' not allowed", scheme));
        }
    }

    // Security: validate path to prevent traversal attacks.
    let file_path = PathBuf::from(&path);
    super::reject_traversal(&file_path)?;

    // Security: ensure the resolved file path is within the vault root.
    let vault_path = {
        let guard = state.vault_path.lock().await;
        guard.clone().ok_or("vault not open")?
    };

    let canonical = dunce::canonicalize(&file_path)
        .map_err(|_| "path validation failed".to_string())?;
    let vault_canonical = dunce::canonicalize(&vault_path)
        .map_err(|_| "vault path validation failed".to_string())?;

    if !canonical.starts_with(&vault_canonical) {
        return Err("path validation failed".to_string());
    }

    app.opener()
        .open_path(&path, None::<&str>)
        .map_err(|e| format!("Failed to open '{}': {}", path, e))?;
    tracing::info!(
        cmd = "open_in_default_app",
        path = %path,
        elapsed_ms = t0.elapsed().as_millis() as u64,
        "IPC",
    );
    Ok(())
}

/// Return the current canvas state as a JSON value.
#[tauri::command]
pub async fn get_canvas(state: State<'_, ForgeState>) -> Result<serde_json::Value, String> {
    let canvas = state.canvas.lock().await;
    serde_json::to_value(&*canvas).map_err(|e| e.to_string())
}

/// Save canvas drawing elements to `.forgedsidian/canvas-drawings.json`
/// inside the vault directory.
///
/// The payload is a raw JSON value (array of drawing elements serialised
/// by the frontend). The backend writes it verbatim — no deserialisation
/// needed, keeping the backend agnostic of the drawing element schema.
#[tauri::command]
pub async fn save_canvas_drawings(
    state: State<'_, ForgeState>,
    drawings: serde_json::Value,
) -> Result<(), String> {
    let vault_path = state.vault_path.lock().await;
    let vault = vault_path.as_ref().ok_or("no vault open")?;
    let dir = vault.join(".forgedsidian");
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("failed to create .forgedsidian dir: {e}"))?;
    let file = dir.join("canvas-drawings.json");
    let json =
        serde_json::to_string_pretty(&drawings).map_err(|e| format!("serialization error: {e}"))?;
    std::fs::write(&file, json).map_err(|e| format!("failed to write canvas drawings: {e}"))?;
    tracing::debug!(path = %file.display(), "canvas drawings saved");
    Ok(())
}

/// Load canvas drawing elements from `.forgedsidian/canvas-drawings.json`.
///
/// Returns `null` if the file does not exist (first use, no drawings yet).
#[tauri::command]
pub async fn load_canvas_drawings(
    state: State<'_, ForgeState>,
) -> Result<serde_json::Value, String> {
    let vault_path = state.vault_path.lock().await;
    let vault = vault_path.as_ref().ok_or("no vault open")?;
    let file = vault.join(".forgedsidian").join("canvas-drawings.json");
    if !file.exists() {
        return Ok(serde_json::Value::Null);
    }
    let content = std::fs::read_to_string(&file)
        .map_err(|e| format!("failed to read canvas drawings: {e}"))?;
    let value: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("failed to parse canvas drawings: {e}"))?;
    tracing::debug!(path = %file.display(), "canvas drawings loaded");
    Ok(value)
}

/// Export canvas drawings as an SVG file.
///
/// Opens a native "Save As" dialog so the user picks the destination.
/// The `svg_content` parameter is the full `<svg>...</svg>` string
/// generated by the frontend.
#[tauri::command]
pub async fn export_canvas_svg(app: AppHandle, svg_content: String) -> Result<(), String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("Exporter le canvas en SVG")
        .set_file_name("canvas-export.svg")
        .add_filter("SVG", &["svg"])
        .save_file(move |path| {
            let _ = tx.send(path);
        });

    let chosen = rx.await.map_err(|_| "dialog channel closed")?;
    match chosen {
        Some(fp) => {
            let p = fp.as_path().ok_or("invalid file path")?.to_path_buf();
            std::fs::write(&p, &svg_content).map_err(|e| format!("failed to write SVG: {e}"))?;
            tracing::info!(path = %p.display(), "canvas SVG exported");
            Ok(())
        }
        None => Ok(()),
    }
}

//! Vault operations — opening vaults, listing notes, searching, and graph snapshots.

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Graph snapshot mirroring `forge_graph::graph::GraphSnapshot`.
///
/// Represents a read-only view of the knowledge graph: nodes (note IDs) and edges (links).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GraphSnapshot {
    /// List of all node identifiers (note UUIDs).
    pub nodes: Vec<String>,
    /// List of all edges as tuples `(source, target)`.
    pub edges: Vec<(String, String)>,
    /// Mapping from UUID to absolute file path (for click-to-open).
    #[serde(default)]
    pub id_to_path: std::collections::HashMap<String, String>,
}

/// Raw JS binding to Tauri 2 IPC.
///
/// Invokes a Tauri backend command with JSON-serializable arguments.
#[wasm_bindgen(inline_js = "
export async function tauri_invoke(cmd, args) {
    return await window.__TAURI_INTERNALS__.invoke(cmd, args);
}
")]
extern "C" {
    #[wasm_bindgen(catch)]
    async fn tauri_invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

/// Raw JS binding to Tauri 2 event system.
///
/// Subscribes to a Tauri backend event and calls `callback` on each occurrence.
/// Returns a no-op if `__TAURI_INTERNALS__` is not available (e.g. in tests).
#[wasm_bindgen(inline_js = "
export async function tauri_listen(event_name, callback) {
    const ti = window.__TAURI_INTERNALS__;
    if (!ti || !ti.invoke) return;
    // Tauri 2: use transformCallback + plugin:event|listen invoke.
    // ti.listen may not exist as a direct function in all Tauri 2 builds.
    if (typeof ti.listen === 'function') {
        await ti.listen(event_name, callback);
    } else {
        const handler = ti.transformCallback(callback);
        await ti.invoke('plugin:event|listen', {
            event: event_name,
            target: { kind: 'Any' },
            handler: handler,
        });
    }
}
")]
extern "C" {
    #[wasm_bindgen(catch)]
    async fn tauri_listen(event_name: &str, callback: &JsValue) -> Result<JsValue, JsValue>;
}

/// Progress update received from the backend during vault indexing.
///
/// Mirrors `forge_vault::ProgressStep` for frontend deserialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingProgress {
    /// Current step number (1-based).
    pub step: u8,
    /// Total number of steps.
    pub total: u8,
    /// Short label, e.g. "Scanning files...".
    pub label: String,
    /// Optional detail, e.g. "7041 files found".
    pub detail: Option<String>,
}

/// Subscribe to vault indexing progress events.
///
/// Calls `on_progress` every time the backend emits `vault:indexing-progress`
/// during [`VaultStore::open_with_progress`].
///
/// Should be called *before* invoking `pick_and_open_vault` or `open_vault`
/// so the listener is ready when progress events start arriving.
pub async fn listen_indexing_progress<F>(on_progress: F)
where
    F: Fn(IndexingProgress) + 'static,
{
    use wasm_bindgen::closure::Closure;

    let cb = Closure::<dyn Fn(JsValue)>::new(move |event: JsValue| {
        // Tauri event: { event: string, payload: T, id: number }
        let payload = js_sys::Reflect::get(&event, &"payload".into())
            .ok()
            .unwrap_or(JsValue::NULL);
        if let Ok(progress) = serde_wasm_bindgen::from_value::<IndexingProgress>(payload) {
            on_progress(progress);
        }
    });

    let _ = tauri_listen("vault:indexing-progress", cb.as_ref()).await;
    cb.forget();
}

/// Subscribe to Tauri vault file-change events.
///
/// Calls `on_event` every time the backend emits `vault:file-changed` or
/// `vault:file-removed`. The callback receives the affected file path as a string.
///
/// # Usage
///
/// ```ignore
/// spawn_local(async {
///     listen_vault_events(move || { ... }).await;
/// });
/// ```
pub async fn listen_vault_events<F>(on_event: F)
where
    F: Fn(String) + 'static,
{
    use wasm_bindgen::closure::Closure;

    let on_event_rc = std::rc::Rc::new(on_event);

    // Closure called by Tauri when vault:file-changed fires.
    // The Tauri event payload is `{ event, payload, id }`.
    let on_event_clone = on_event_rc.clone();
    let cb_changed = Closure::<dyn Fn(JsValue)>::new(move |event: JsValue| {
        let path = js_sys::Reflect::get(&event, &"payload".into())
            .ok()
            .and_then(|p| p.as_string())
            .unwrap_or_default();
        on_event_clone(path);
    });

    let on_event_clone2 = on_event_rc.clone();
    let cb_removed = Closure::<dyn Fn(JsValue)>::new(move |event: JsValue| {
        let path = js_sys::Reflect::get(&event, &"payload".into())
            .ok()
            .and_then(|p| p.as_string())
            .unwrap_or_default();
        on_event_clone2(path);
    });

    // Register both listeners; ignore errors (not in Tauri context = no-op).
    let _ = tauri_listen("vault:file-changed", cb_changed.as_ref()).await;
    let _ = tauri_listen("vault:file-removed", cb_removed.as_ref()).await;

    // Leak the closures so they stay alive for the app lifetime.
    cb_changed.forget();
    cb_removed.forget();
}

/// Opens a vault at the given path.
///
/// Invokes the `open_vault` Tauri command with the vault path.
/// Returns `Ok(())` on success, or an error message on failure.
pub async fn open_vault(path: &str) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        path: String,
    }

    let args = Args {
        path: path.to_string(),
    };

    let js_args = serde_wasm_bindgen::to_value(&args)
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

    tauri_invoke("open_vault", js_args)
        .await
        .map_err(|e| format!("Tauri invoke failed: {:?}", e))?;

    Ok(())
}

/// Opens a native folder picker and opens the vault at the selected path.
///
/// Returns `Some(path)` if a folder was selected and the vault opened,
/// or `None` if the user cancelled the dialog.
pub async fn pick_and_open_vault() -> Result<Option<String>, String> {
    #[derive(Serialize)]
    struct Args {}

    let args = Args {};

    let js_args = serde_wasm_bindgen::to_value(&args)
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = tauri_invoke("pick_and_open_vault", js_args)
        .await
        .map_err(|e| format!("Tauri invoke failed: {:?}", e))?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize response: {}", e))
}

/// Lists all note paths in the vault (relative to vault root).
///
/// Returns a sorted vector of relative paths like `"subfolder/note.md"`.
pub async fn list_notes() -> Result<Vec<String>, String> {
    #[derive(Serialize)]
    struct Args {}

    let args = Args {};

    let js_args = serde_wasm_bindgen::to_value(&args)
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = tauri_invoke("list_notes", js_args)
        .await
        .map_err(|e| format!("Tauri invoke failed: {:?}", e))?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize response: {}", e))
}

/// Fetches the content (Markdown body) of a note at the given path.
///
/// Invokes the `get_note` Tauri command.
/// Returns the note body on success, or an error message on failure.
pub async fn get_note(path: &str) -> Result<String, String> {
    super::validate_path_helper(path)?;
    #[derive(Serialize)]
    struct Args {
        path: String,
    }

    let args = Args {
        path: path.to_string(),
    };

    let js_args = serde_wasm_bindgen::to_value(&args)
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = tauri_invoke("get_note", js_args)
        .await
        .map_err(|e| format!("Tauri invoke failed: {:?}", e))?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize response: {}", e))
}

/// Searches notes by a query string.
///
/// Invokes the `search_notes` Tauri command with a search query.
/// Returns a list of matching note paths on success.
pub async fn search_notes(query: &str) -> Result<Vec<String>, String> {
    #[derive(Serialize)]
    struct Args {
        query: String,
    }

    let args = Args {
        query: query.to_string(),
    };

    let js_args = serde_wasm_bindgen::to_value(&args)
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = tauri_invoke("search_notes", js_args)
        .await
        .map_err(|e| format!("Tauri invoke failed: {:?}", e))?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize response: {}", e))
}

/// Save canvas drawing elements to the vault's `.forgedsidian/` directory.
///
/// The `drawings_json` is a raw `JsValue` (JSON array serialised from
/// `Vec<DrawEl>` on the caller side) — passed through verbatim to the
/// backend which writes it to `canvas-drawings.json`.
pub async fn save_canvas_drawings(drawings_json: JsValue) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        drawings: serde_json::Value,
    }

    let drawings: serde_json::Value = serde_wasm_bindgen::from_value(drawings_json)
        .map_err(|e| format!("Failed to convert drawings: {e}"))?;
    let args = Args { drawings };
    let js_args = serde_wasm_bindgen::to_value(&args)
        .map_err(|e| format!("Failed to serialize args: {e}"))?;
    tauri_invoke("save_canvas_drawings", js_args)
        .await
        .map_err(|e| format!("save_canvas_drawings failed: {e:?}"))?;
    Ok(())
}

/// Load canvas drawing elements from the vault's `.forgedsidian/` directory.
///
/// Returns `Ok(JsValue)` — either a JSON array or `null` if no file exists.
pub async fn load_canvas_drawings() -> Result<JsValue, String> {
    #[derive(Serialize)]
    struct Args {}
    let args = Args {};
    let js_args = serde_wasm_bindgen::to_value(&args)
        .map_err(|e| format!("Failed to serialize args: {e}"))?;
    let result = tauri_invoke("load_canvas_drawings", js_args)
        .await
        .map_err(|e| format!("load_canvas_drawings failed: {e:?}"))?;
    Ok(result)
}

/// Export canvas as SVG. Opens a native Save dialog on the backend.
pub async fn export_canvas_svg(svg_content: String) -> Result<(), String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        svg_content: String,
    }
    let args = Args { svg_content };
    let js_args = serde_wasm_bindgen::to_value(&args)
        .map_err(|e| format!("Failed to serialize args: {e}"))?;
    tauri_invoke("export_canvas_svg", js_args)
        .await
        .map_err(|e| format!("export_canvas_svg failed: {e:?}"))?;
    Ok(())
}

/// Fetches the current graph snapshot (all nodes and edges).
///
/// Invokes the `get_graph_snapshot` Tauri command.
/// Returns a `GraphSnapshot` on success.
pub async fn get_graph_snapshot() -> Result<GraphSnapshot, String> {
    #[derive(Serialize)]
    struct Args {}

    let args = Args {};

    let js_args = serde_wasm_bindgen::to_value(&args)
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = tauri_invoke("get_graph_snapshot", js_args)
        .await
        .map_err(|e| format!("Tauri invoke failed: {:?}", e))?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize response: {}", e))
}

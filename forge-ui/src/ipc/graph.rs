//! Graph operations — resolve wikilinks and open files in default applications.

use serde::Serialize;
use wasm_bindgen::prelude::*;

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

/// Resolve a wikilink target to an absolute file path.
///
/// Invokes the `resolve_wikilink` Tauri command. Returns `Some(path)` if the
/// target matches a note in the vault (case-insensitive stem match), or `None`
/// if no match is found.
///
/// Supports `[[target]]`, `[[target#heading]]`, and `[[folder/target]]` syntax.
pub async fn resolve_wikilink(target: &str) -> Result<Option<String>, String> {
    #[derive(Serialize)]
    struct Args {
        target: String,
    }

    let args = Args {
        target: target.to_string(),
    };

    let js_args = serde_wasm_bindgen::to_value(&args)
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = tauri_invoke("resolve_wikilink", js_args)
        .await
        .map_err(|e| format!("Tauri invoke failed: {:?}", e))?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize response: {}", e))
}

/// Open a file or URL in the user's default application.
///
/// Cross-platform: delegates to the OS shell via `tauri-plugin-shell`.
/// Used for `.html` wikilinks and any file the app cannot render natively.
pub async fn open_in_default_app(path: &str) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        path: String,
    }

    let args = Args {
        path: path.to_string(),
    };

    let js_args = serde_wasm_bindgen::to_value(&args)
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

    tauri_invoke("open_in_default_app", js_args)
        .await
        .map_err(|e| format!("Tauri invoke failed: {:?}", e))?;

    Ok(())
}

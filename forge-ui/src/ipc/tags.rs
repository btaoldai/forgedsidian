//! IPC wrappers for tag operations.

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

/// Returns all distinct tags in the vault, sorted alphabetically.
///
/// Returns an empty list if no vault is open or if no notes have tags.
pub async fn list_tags() -> Vec<String> {
    #[derive(Serialize)]
    struct Args {}

    let args = Args {};
    let js_args = match serde_wasm_bindgen::to_value(&args) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    match tauri_invoke("list_tags", js_args).await {
        Ok(result) => serde_wasm_bindgen::from_value(result).unwrap_or_default(),
        Err(_) => vec![],
    }
}

/// Returns relative note paths tagged with `tag` (case-insensitive).
///
/// Returns an empty list if no vault is open or if no notes carry the tag.
pub async fn notes_by_tag(tag: &str) -> Vec<String> {
    #[derive(Serialize)]
    struct Args<'a> {
        tag: &'a str,
    }

    let args = Args { tag };
    let js_args = match serde_wasm_bindgen::to_value(&args) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    match tauri_invoke("notes_by_tag", js_args).await {
        Ok(result) => serde_wasm_bindgen::from_value(result).unwrap_or_default(),
        Err(_) => vec![],
    }
}

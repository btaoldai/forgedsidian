//! File operations — create, read, move, delete, and list files and folders.

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

/// Reads any text file from the vault by its absolute path.
///
/// Unlike `get_note` (which reads from the Tantivy index), this reads raw
/// file content directly from disk. Used for non-.md files.
pub async fn read_file(path: &str) -> Result<String, String> {
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

    let result = tauri_invoke("read_file", js_args)
        .await
        .map_err(|e| format!("Tauri invoke failed: {:?}", e))?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize response: {}", e))
}

/// Saves a note to disk.
///
/// Invokes the `save_note` Tauri command with the note ID and new body content.
/// Returns `Ok(())` on success.
pub async fn save_note(note_id: &str, body: &str) -> Result<(), String> {
    super::validate_path_helper(note_id)?;
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        note_id: String,
        body: String,
    }

    let args = Args {
        note_id: note_id.to_string(),
        body: body.to_string(),
    };

    let js_args = serde_wasm_bindgen::to_value(&args)
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

    tauri_invoke("save_note", js_args)
        .await
        .map_err(|e| format!("Tauri invoke failed: {:?}", e))?;

    Ok(())
}

/// Creates a new note in the vault.
///
/// Returns the relative path of the created note.
pub async fn create_note(folder: &str, name: &str) -> Result<String, String> {
    super::validate_path_helper(folder)?;
    super::validate_path_helper(name)?;
    #[derive(Serialize)]
    struct Args {
        folder: String,
        name: String,
    }

    let args = Args {
        folder: folder.to_string(),
        name: name.to_string(),
    };

    let js_args = serde_wasm_bindgen::to_value(&args)
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = tauri_invoke("create_note", js_args)
        .await
        .map_err(|e| format!("Tauri invoke failed: {:?}", e))?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize response: {}", e))
}

/// Creates a new folder in the vault.
///
/// Returns the relative path of the created folder.
pub async fn create_folder(parent: &str, name: &str) -> Result<String, String> {
    super::validate_path_helper(parent)?;
    super::validate_path_helper(name)?;
    #[derive(Serialize)]
    struct Args {
        parent: String,
        name: String,
    }

    let args = Args {
        parent: parent.to_string(),
        name: name.to_string(),
    };

    let js_args = serde_wasm_bindgen::to_value(&args)
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = tauri_invoke("create_folder", js_args)
        .await
        .map_err(|e| format!("Tauri invoke failed: {:?}", e))?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize response: {}", e))
}

/// Lists ALL non-hidden file paths in the vault (relative to vault root).
///
/// Unlike `list_notes` (`.md` only), this returns every file type:
/// `.txt`, `.rs`, `.py`, `.pdf`, images, etc.
pub async fn list_all_files() -> Result<Vec<String>, String> {
    #[derive(Serialize)]
    struct Args {}

    let js_args = serde_wasm_bindgen::to_value(&Args {})
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = tauri_invoke("list_all_files", js_args)
        .await
        .map_err(|e| format!("Tauri invoke failed: {:?}", e))?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize response: {}", e))
}

/// Lists all non-hidden folder paths in the vault (relative to vault root).
///
/// Returns a sorted vector of relative paths like `"subfolder/deep"`.
pub async fn list_folders() -> Result<Vec<String>, String> {
    #[derive(Serialize)]
    struct Args {}

    let js_args = serde_wasm_bindgen::to_value(&Args {})
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = tauri_invoke("list_folders", js_args)
        .await
        .map_err(|e| format!("Tauri invoke failed: {:?}", e))?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize response: {}", e))
}

/// Deletes a folder and all its contents.
///
/// Requires explicit confirmation. Returns a status message.
pub async fn delete_folder(path: &str, confirm: bool) -> Result<String, String> {
    super::validate_path_helper(path)?;
    #[derive(Serialize)]
    struct Args {
        path: String,
        confirm: bool,
    }

    let args = Args {
        path: path.to_string(),
        confirm,
    };

    let js_args = serde_wasm_bindgen::to_value(&args)
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = tauri_invoke("delete_folder", js_args)
        .await
        .map_err(|e| format!("Tauri invoke failed: {:?}", e))?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize response: {}", e))
}

/// Moves a file from one relative path to a destination folder within the vault.
///
/// `from` is the relative path of the source file (e.g. "folder/note.md").
/// `to` is the destination directory (e.g. "other_folder"). Empty string = vault root.
/// Returns the new relative path of the moved file.
pub async fn move_file(from: &str, to: &str) -> Result<String, String> {
    #[derive(Serialize)]
    struct Args {
        from: String,
        to: String,
    }

    let args = Args {
        from: from.to_string(),
        to: to.to_string(),
    };

    let js_args = serde_wasm_bindgen::to_value(&args)
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = tauri_invoke("move_file", js_args)
        .await
        .map_err(|e| format!("Tauri invoke failed: {:?}", e))?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize response: {}", e))
}

/// Moves a folder from one relative path to a destination parent within the vault.
///
/// `from` is the relative path of the source folder (e.g. "subfolder").
/// `to` is the destination parent directory (e.g. "other_parent"). Empty string = vault root.
/// Returns the new relative path of the moved folder.
pub async fn move_folder(from: &str, to: &str) -> Result<String, String> {
    #[derive(Serialize)]
    struct Args {
        from: String,
        to: String,
    }

    let args = Args {
        from: from.to_string(),
        to: to.to_string(),
    };

    let js_args = serde_wasm_bindgen::to_value(&args)
        .map_err(|e| format!("Failed to serialize args: {}", e))?;

    let result = tauri_invoke("move_folder", js_args)
        .await
        .map_err(|e| format!("Tauri invoke failed: {:?}", e))?;

    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("Failed to deserialize response: {}", e))
}

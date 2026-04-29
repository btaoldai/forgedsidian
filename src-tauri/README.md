# forge-app (src-tauri)

Tauri application entry point and IPC command layer.

## Purpose

Bridges the Leptos WASM frontend and the Rust backend engine. Exposes thin command handlers that validate input, delegate to engine crates (forge-vault, forge-graph, forge-editor, forge-canvas), and return serialisable results. No business logic lives in this crate — it is purely an IPC boundary.

## Key modules

- `commands` — command handlers organized by domain (vault_ops, file_ops, scan)

## Architecture

```
Leptos WASM (src/)
  |  invoke("command_name", payload)
  v
Tauri IPC bridge
  |
  v
commands.rs  (thin handlers, no business logic)
  |
  v
forge-vault / forge-graph / forge-canvas / forge-editor
```

## Dependencies

- Internal: forge-vault, forge-graph, forge-canvas, forge-editor
- External: tauri, tokio, tracing, tracing-subscriber

## Usage

Command handlers are generated automatically by Tauri's `#[tauri::command]` macro and invoked from the frontend via Leptos invoke. Example:

```rust
#[tauri::command]
pub async fn list_notes(state: State<'_, ForgeState>) -> Result<Vec<Note>, String> {
    // Validate, delegate, return
}
```

## Related docs

- Frontend: Leptos WASM in `src/` directory
- Configuration: `tauri.conf.json` and CSP settings
- Command registration and IPC contract documented in this crate's commands module

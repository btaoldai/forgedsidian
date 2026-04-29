# forge-ui

Leptos CSR frontend for Forgedsidian.

## Purpose

Compiles to WASM and runs inside the Tauri desktop application's WebView. Implements the full user interface: editor panel, graph visualization, sidebar, settings, and canvas whiteboard. All communication with the native Rust backend flows through the Tauri IPC bridge (`invoke` commands and `listen` event subscriptions).

## Key modules

- `app` — root `<App />` component, global state management, and tab routing
- `components` — reusable UI components: `Editor`, `GraphView`, `Sidebar`, `Canvas`, `Settings`
- `ipc` — Tauri command wrappers and type-safe message serialization
- `tab_manager` — multi-tab session state and tab lifecycle
- `settings` — user preferences and theme configuration

## Stack

- **Leptos 0.7 CSR** — reactive frontend framework compiling to WebAssembly
- **Trunk** — WASM bundler and dev server
- **web-sys** — low-level Web API bindings
- **serde** — serialization for IPC payloads

## Dependencies

- Internal: forge-core, forge-canvas, forge-renderer
- External: leptos, wasm-bindgen, serde, pulldown-cmark, ammonia, web-sys

## Build and test

```bash
# Development server (file watcher + hot reload)
trunk serve

# Build production WASM
trunk build --release

# Run WASM tests (requires Chrome)
wasm-pack test --headless --chrome forge-ui
```

## Architecture

```
Leptos UI (WASM)
  └─ Tauri IPC (invoke/listen)
       └─ Rust backend (forge-vault, forge-graph)
```

## Related docs

- Architecture Decision Record: `docs/adr/0002-tauri-leptos-gui-runtime.md`
- Tauri integration: `src-tauri/src/commands.rs`
- Component library and Leptos patterns

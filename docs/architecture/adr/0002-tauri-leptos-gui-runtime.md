# ADR 0002 — Tauri 2 + Leptos CSR WASM GUI runtime

**Status**: Accepted
**Date**: 2026-04-09

## Context

Forgedsidian requires a modern desktop application framework that supports:
1. Cross-platform native windows (Windows, macOS, Linux)
2. Rich interactive UI (canvas, graph visualization, editor)
3. Deep integration with a Rust backend (storage, indexing, graph algorithms)
4. Fast iteration during development (hot reload, debugging)

Three options were evaluated:

- **Tauri + Svelte**: Proven pattern, fast builds, large ecosystem. Svelte is JS-based (adds JS dependencies, test matrix complexity)
- **egui (immediate-mode GUI)**: Pure Rust, no JS complexity, but harder to achieve pixel-perfect layouts and responsive design
- **Tauri + Leptos (CSR WASM)**: Tauri as native shell, Leptos compiles Rust to WASM for the UI

## Decision

Adopted **Tauri 2.0.2 + Leptos 0.7 CSR (Client-Side Rendering)** compiled to WASM.

### Architecture

```
Desktop App (Tauri 2.0.2)
  ├─ Main Process (Rust)
  │   ├─ forge-vault (storage & indexing)
  │   ├─ forge-graph (graph queries)
  │   └─ Tauri IPC server (invoke handlers)
  │
  └─ WebView (WRY)
      ├─ forge-ui (Leptos CSR → WASM)
      └─ Tauri IPC client (invoke/listen)
```

### Build pipeline

1. **WASM compilation** (Trunk): `forge-ui` → WebAssembly (WASM) + JS glue code
2. **Tauri packaging**: Embed compiled WASM in Tauri's WebView
3. **IPC serialization** (serde): Type-safe RPC between UI and backend

## Consequences

**Positive:**
- **No JavaScript build tools** — `forge-ui` is pure Rust, compiles via Trunk (rustup + wasm-pack)
- **Excellent type safety** — Leptos types and Tauri command handlers share domain types from forge-core
- **Hot reload during dev** — Trunk serves WASM with file watchers; backend separately with `cargo run`
- **WASM sandbox** — UI code runs isolated in the WebView; malicious UI code cannot directly access the file system
- **Code reuse** — Canvas and renderer logic (forge-canvas, forge-renderer) share source between UI and tests

**Negative / Trade-offs:**
- **WASM startup latency** — ~500ms–2s initial load time depending on build size and network
- **Larger binary size** — WASM modules add ~3–5 MB (gzip ~1–2 MB) to the distributable
- **Two build systems** — backend uses Cargo, frontend uses Trunk; CI/CD must invoke both
- **Debugging WASM** — browser DevTools required; Rust stack traces are obfuscated

## Open questions

1. **Code-splitting and lazy loading** — should heavy components (GraphView, Canvas) load on-demand to reduce startup latency?
2. **State synchronization** — how to handle offline edits if the IPC bridge disconnects?
3. **Performance at scale** — wgpu renderer handles 1000+ nodes; what about 10,000+?

## References

- Tauri command architecture: `src-tauri/src/commands.rs`
- Leptos frontend structure: `forge-ui/src/lib.rs`
- Trunk build config: `forge-ui/Trunk.toml`
- Phase 3.1–3.2 development notes: GUI runtime selection

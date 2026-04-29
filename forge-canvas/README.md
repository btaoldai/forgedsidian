# forge-canvas

Infinite canvas rendering primitives, hit-testing and ABCDE layout.

## Purpose

Provides the core canvas abstraction for the Forgedsidian whiteboard: managing an infinite 2D canvas with arbitrary items (notes, tasks, shapes, embedded references), computing viewport-relative hit regions, and implementing the ABCDE prioritization framework for task organization. Intentionally `no_std`-compatible to support both native and WASM targets.

## Key modules

- `canvas` — `Canvas` state machine managing item collection and viewport state
- `item` — `CanvasItem` type hierarchy and item metadata (position, bounds, priority)
- `hit` — axis-aligned bounding-box hit-testing and intersection queries
- `abcde` — ABCDE prioritization logic (translate user priorities to item rendering order)
- `error` — canvas-specific error types

## Design constraints

- **No async, no tokio** — purely synchronous computation
- **WASM-compatible** — compiles to `wasm32-unknown-unknown` without OS-level APIs
- **Serde on all public types** — serialization for Tauri IPC and persistence
- **No heavyweight dependencies** — only core utilities for serialization and math

## Dependencies

- Internal: forge-core
- External: serde, serde_json, uuid, thiserror

## Usage

```rust
use forge_canvas::{Canvas, CanvasItem, Viewport};

let mut canvas = Canvas::new();
canvas.upsert_item(item_id, CanvasItem::default());
let hits = canvas.hit_test(&viewport, x, y)?;
```

## Related docs

- Architecture Decision Record: `docs/adr/0002-tauri-leptos-gui-runtime.md`
- Viewport and camera behavior: see ABCDE layout algorithm in `src/abcde.rs`
- Integration with Leptos frontend for rendering and interaction

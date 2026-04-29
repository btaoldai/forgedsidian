# forge-renderer

GPU-accelerated graph renderer using wgpu.

## Purpose

Replaces the SVG-based graph visualization with a high-performance GPU renderer. Uses wgpu to render thousands of nodes and edges via instanced circle and line primitives, supporting arbitrary zoom and pan. Targets 60 FPS for graphs exceeding 1000 nodes.

## Key modules

- `context` — `GpuContext` wrapping wgpu `Device`, `Queue`, and `Surface`
- `pipeline` — `GraphPipeline` managing shader programs, vertex/instance buffers, and draw commands
- `camera` — `Camera2D` orthographic projection with zoom, pan, and viewport-relative hit-testing
- `instance` — node and edge instance data (position, color, size) for GPU streaming
- `error` — renderer-specific error types

## Stack

- **wgpu 24** — WebGPU abstraction (Vulkan/DX12/Metal on native, WebGPU/WebGL2 on WASM)
- **glam** — lightweight linear algebra (Mat4, Vec2) for camera math
- **bytemuck** — zero-copy struct-to-GPU-buffer serialization
- **tracing** — debug/perf logging

## Dependencies

- Internal: none (standalone, can be imported independently)
- External: wgpu, glam, bytemuck, tracing, web-sys (WASM target only)

## Usage

```rust
use forge_renderer::GraphRenderer;

let renderer = GraphRenderer::from_canvas(canvas_elem, width, height).await?;
renderer.set_nodes(&node_instances);
renderer.set_edges(&edge_instances);
renderer.render();
```

## Related docs

- Architecture Decision Record: `docs/adr/0003-wgpu-graph-renderer.md`
- Integration with Leptos canvas element in forge-ui
- Force-directed layout algorithms: forge-graph

# ADR 0003 — wgpu for GPU-accelerated graph rendering

**Status**: Accepted
**Date**: 2026-04-12

## Context

Forgedsidian displays knowledge graphs (notes as nodes, wikilinks as edges) in real-time. Early prototypes using SVG + D3 rendered smoothly up to ~200 nodes, but degraded to <30 FPS for 1000+ nodes.

Three rendering strategies were considered:

- **SVG + canvas libraries (D3, Cytoscape)**: Familiar, easy to prototype. Bottleneck: DOM overhead and CPU-side layout math at every frame.
- **Canvas 2D + custom force-directed layout**: Avoids DOM overhead; physics computation remains CPU-bound
- **wgpu + GPU instancing + force-directed layout**: Offload rendering to GPU; compute layout updates on CPU at ~60 Hz

## Decision

Adopted **wgpu 24** with **instanced SDF (Signed Distance Field) rendering** for circles and lines.

### Architecture

```
forge-graph (CPU physics loop)
  ├─ Fruchterman-Reingold force-directed layout
  └─ per-frame node/edge position updates (60 Hz)

forge-renderer (GPU pipeline)
  ├─ GpuContext (wgpu device, queue, surface)
  ├─ GraphPipeline (shader programs, instance buffers)
  ├─ Camera2D (orthographic projection, zoom/pan)
  └─ Render each frame (bind buffers, dispatch draw calls)

Leptos UI (WASM) / forge-ui
  └─ <canvas> HTML element + event handlers
```

### Technical approach

1. **Instanced rendering**: Node/edge positions stored in GPU instance buffers, updated each physics frame
2. **SDF shaders**: Fragment shaders compute circle/line distance fields for smooth anti-aliased geometry
3. **Camera2D**: Orthographic matrix handles zoom and pan without re-uploading geometry
4. **Hit-testing**: Transform mouse click coordinates through camera inverse projection; compare against node bounding circles

## Consequences

**Positive:**
- **60 FPS for 1000+ nodes** — GPU parallelism handles thousands of primitives
- **WebGPU on WASM** — wgpu bindings work in the browser via WebGPU (Chromium 113+) and WebGL2 fallback
- **Hot reload friendly** — forge-renderer is standalone; UI and backend can iterate independently
- **Scalability headroom** — 10,000+ nodes feasible with optimization (LOD, frustum culling)

**Negative / Trade-offs:**
- **Browser compatibility** — WebGPU still experimental; WebGL2 fallback has lower performance
- **Shader authoring** — GLSL/WGSL knowledge required for future optimizations
- **Memory overhead** — GPU buffers consume VRAM; large graphs may need streaming/LOD
- **Migration from SVG** — existing SVG-based graph view must be replaced (Phase 15)

## Migration path

- Phase 15: Replace `forge-graph`'s SVG rendering with `forge-renderer` as the primary graph view
- Deprecate SVG graph view or retain as low-end fallback
- Integrate physics simulation (forge-graph layout module) with renderer event loop

## Open questions

1. **Force-directed layout on GPU?** — should we move physics computation to compute shaders for 10,000+ node graphs?
2. **Cluster rendering** — how to efficiently render subgraphs (collapsed clusters) at different scales?
3. **Custom node shapes** — currently hardcoded to circles; extensible to arbitrary shapes?

## References

- wgpu documentation: https://github.com/gfx-rs/wgpu
- glam math library: https://github.com/bitshifter/glam-rs
- Force-directed layout: forge-graph (Phase 2)
- Leptos canvas integration: forge-ui `GraphView` component

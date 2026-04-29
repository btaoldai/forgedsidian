# Changelog

All notable changes to `forge-renderer` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] - 2026-04-12

### Added

- `GpuContext`: wgpu initialization (Instance, Adapter, Device, Queue, Surface)
- `Camera2D`: orthographic projection with zoom, pan, fit-to-bounds
- `GraphPipeline`: instanced rendering pipeline for nodes and edges
- `NodeInstance`: GPU-friendly node data (`#[repr(C)]` + bytemuck)
- `EdgeInstance`: GPU-friendly edge data (`#[repr(C)]` + bytemuck)
- `GraphRenderer`: high-level API combining GPU context + pipeline + camera
- WGSL shaders: `node.wgsl` (SDF circles), `edge.wgsl` (oriented quads)
- WASM support via `web_sys::HtmlCanvasElement` surface target
- ADR-010: architecture decision record for wgpu migration

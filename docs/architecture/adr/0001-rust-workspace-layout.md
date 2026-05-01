# ADR 0001 — Rust workspace layout

**Status**: Accepted
**Date**: 2026-04-13

## Context

Forgexalith is a modular personal knowledge management (PKM) system with multiple architectural layers: a storage and indexing backend, a graph computation engine, editing and rendering capabilities, and a Tauri desktop application frontend. Early in the project, a monolithic crate structure became unwieldy and prevented clear separation of concerns and independent testing.

The challenge: how to organize Rust code into crates that respect clean architecture, minimize circular dependencies, and allow parallel development without re-compiling the entire workspace?

## Decision

We adopted a **workspace of 8 focused crates**, arranged in a strict dependency hierarchy:

1. **forge-core** (root, no dependencies on siblings)
   - Stable public types: `Note`, `NoteId`, domain events, error hierarchy, format constants
   - Imported by all other crates

2. **forge-vault**
   - Persistent storage: read/write Markdown files, Tantivy indexing, audit logging
   - Depends on: forge-core

3. **forge-editor**
   - Markdown AST parsing, YAML frontmatter extraction, wikilink detection
   - Depends on: forge-core

4. **forge-graph**
   - Directed note graph via petgraph, backlink/forward-link queries, force-directed layout
   - Depends on: forge-core, forge-editor (for link extraction), forge-vault (for graph syncing)

5. **forge-canvas**
   - Canvas/whiteboard rendering primitives (strokes, shapes, embedded references)
   - Depends on: forge-core

6. **forge-renderer**
   - Markdown to HTML rendering, KaTeX math, syntax highlighting
   - Depends on: forge-core, forge-editor

7. **forge-ui**
   - Reusable UI component library (buttons, modals, panels — not Leptos-specific)
   - Depends on: forge-core

8. **src-tauri** (Tauri desktop application)
   - IPC command handlers, Leptos integration, Tauri plugin setup
   - Depends on: forge-vault, forge-graph, forge-canvas, forge-renderer, forge-editor, forge-ui

## Consequences

**Positive:**
- Clear separation of concerns: storage, indexing, parsing, graph, rendering, UI are independent
- `forge-core` is stable and backward-compatible; other crates evolve without destabilizing the root
- Each crate is independently testable and reusable (e.g., forge-vault as a library)
- Parallel compilation: crates with no dependencies on each other compile in parallel
- Dependencies form a DAG (directed acyclic graph) — no cycles
- Easy to identify where business logic lives (e.g., graph algorithms in forge-graph, indexing in forge-vault)

**Negative / Trade-offs:**
- Ceremony: every small change may span multiple crate boundaries, requiring careful API design
- Over-modularity risk: risk of creating "junk" crates that only wrap a few functions
- Duplicate error types: each crate has its own error type for clarity, adding boilerplate
- Re-export verbosity: frequently re-export common types at crate root to reduce import paths

**Open questions:**
- **forge-vault currently depends on forge-editor and forge-graph** — this creates an inversion; vault should be a pure storage/index layer. Planned refactor for next sprint: extract link-resolution logic from vault into forge-graph as a post-processing step.
- **forge-ui and rendering strategy**: forge-renderer outputs HTML; forge-ui is a component library. Integration path with Leptos frontend to be finalized in Phase 2.
- **Cross-crate error handling**: currently each crate converts its errors to a Result<T, String> for Tauri. Consider a unified error bridge in Phase 2.

## References

- Dependency graph: `Cargo.toml` workspace definition
- Individual crate READMEs: each forge-* directory
- Tauri command architecture: `src-tauri/src/commands.rs`

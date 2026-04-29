# forge-core

Shared foundation for the Forgedsidian PKM engine.

## Purpose

This crate defines the **stable public types** that all other forge-* crates depend on. It is the dependency root of the workspace graph — nothing in this crate imports from sibling crates. Every domain concept (Note, NoteId, domain events, error types) starts here.

## Key modules

- `id` — opaque, typed identifiers (newtype wrappers around `Uuid`) for notes, tags, and other entities
- `note` — the canonical `Note` domain model with metadata and content
- `event` — domain events emitted by the engine (note created, updated, linked, etc.)
- `error` — shared error type hierarchy used across all crates
- `format` — file format constants and helpers (Markdown, YAML frontmatter conventions)

## Dependencies

- Internal: none (root of dependency tree)
- External: uuid, serde, serde_json, chrono

## Usage

```rust
use forge_core::{Note, NoteId, CoreError};

let note_id = NoteId::new();
// Construct a Note struct or handle CoreError from parsing
```

## Related docs

- Architecture Decision Record: `docs/adr/0001-rust-workspace-layout.md`
- Workspace layout and dependency graph explained in parent README

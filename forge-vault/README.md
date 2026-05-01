# forge-vault

Note storage, full-text indexing and metadata search.

## Purpose

Manages persistent vault state: reading and writing Markdown files from disk, building and maintaining a Tantivy full-text index, and exposing query APIs for note retrieval and search. Implements the storage layer and indexing backend that underpins the Forgexalith engine.

## Key modules

- `store` — file system read/write operations for Markdown notes
- `index` — Tantivy index lifecycle, rebuilding, and full-text search queries
- `storage` — abstract storage backend trait and filesystem implementation
- `manifest` — vault metadata and manifest tracking
- `audit` — audit logging for vault operations
- `watcher` — file-system watcher for live reloading (Phase 3.3)
- `error` — vault-specific error types

## Dependencies

- Internal: forge-core
- External: tantivy, tokio, serde, walkdir

## Usage

```rust
use forge_vault::{VaultStore, RealFs};

let store = VaultStore::new("path/to/vault", RealFs).await?;
let notes = store.list_all_notes().await?;
let results = store.search("query text").await?;
```

## Related docs

- Full-text search design: consult `docs/adr/` for indexing strategy
- Watcher implementation planned for Phase 3.3
- Integration with forge-editor for YAML frontmatter parsing

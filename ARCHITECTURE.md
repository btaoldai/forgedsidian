# Architecture -- Forgexalith

> Synthesis document. Detailed Architecture Decision Records (ADRs) and C4 diagrams live in [docs/architecture/](docs/architecture/).

## TL;DR

Forgexalith is a single Tauri 2 desktop application backed by a Rust monorepo workspace of 8 crates. The backend (Rust, native) handles vault scanning, full-text indexing (Tantivy), graph data structures (petgraph), file watching, IPC, and security primitives (HMAC manifest, append-only audit log). The frontend (Leptos 0.7 CSR, compiled to WebAssembly) handles all UI. Communication is exclusively through Tauri's IPC layer with hardened path validation.

```
+---------------------------+        IPC commands         +---------------------------+
|   forge-ui (Leptos CSR)   |  <----------------------->  |   src-tauri (backend)     |
|   - File tree             |   serde-serialized JSON     |   - 22+ IPC commands       |
|   - Editor (Preview/Edit) |                              |   - Path traversal guard   |
|   - Graph view (SVG/wgpu) |                              |   - CSP injection          |
|   - Command palette       |                              |   - Symlink rejection     |
|   - Canvas + drawing      |                              |                            |
+---------------------------+                              +-------------+-------------+
                                                                          |
                                                                          | depends on
                                                                          v
                                  +--------------------------------------------------+
                                  |            Rust crates (native workspace)         |
                                  +-------------+-------------+----------+------------+
                                  | forge-core  | forge-vault | forge-   | forge-     |
                                  | (types,     | (Tantivy,   | editor   | graph      |
                                  | traits)     | watcher,    | (links,  | (Note-     |
                                  |             | manifest)   | md)      | Graph)     |
                                  +-------------+-------------+----------+------------+
                                  | forge-canvas | forge-renderer (wgpu) |
                                  +--------------+-----------------------+
                                                                          |
                                                                          v
                                  +--------------------------------------------------+
                                  |   Local filesystem : vault dir + .forge-index/    |
                                  |   - markdown notes (.md)                          |
                                  |   - frontmatter YAML                              |
                                  |   - .forge-index/manifest.json + .sig + .hmac-key |
                                  |   - .forge-index/audit.jsonl                      |
                                  |   - .forge-index/tantivy/...                      |
                                  +--------------------------------------------------+
```

## Crate-level breakdown

| Crate | Responsibility | Key types | Tests |
|---|---|---|---|
| **forge-core** | Shared primitives across all crates (no third-party deps beyond serde/uuid). | `NoteId`, `VaultPath`, `Wikilink`, `WikilinkExtractor` trait | 20 |
| **forge-vault** | Vault store (filesystem scan), Tantivy full-text index, file watcher (notify v7), HMAC-signed manifest, audit log. | `VaultStore`, `VaultIndex`, `StorageBackend` trait, `Manifest` | 80+ |
| **forge-editor** | Markdown helpers (pulldown-cmark), wikilink extraction (regex + pulldown), HTML sanitization (ammonia). | `PulldownWikilinkExtractor`, `markdown_to_html` | 56 |
| **forge-graph** | Note graph data structure (petgraph wrapper), serializable snapshots for the frontend, force-directed layout simulation. | `NoteGraph`, `GraphSnapshot`, `ForceDirected` | 18 |
| **forge-canvas** | Canvas / spatial outliner data model and rendering helpers. | `Canvas`, `Card`, `DrawingLayer` | TBD |
| **forge-renderer** | wgpu-based GPU renderer for the graph (work in progress, replaces SVG path for large graphs). | `GraphRenderer` | TBD |
| **forge-ui** | Leptos 0.7 CSR frontend, compiled to WebAssembly via Trunk. Excluded from the native workspace (built separately). | App components, IPC wrappers | E2E TBD |
| **src-tauri** | Tauri 2 backend: IPC commands (22+), CSP injection, plugin registration, error sanitization. | `commands::*`, `error::*` | integ |

## Key design decisions (ADRs)

| ADR | Title | Status |
|---|---|---|
| [0001](docs/architecture/adr/0001-rust-workspace-layout.md) | Rust workspace layout (monorepo, multi-crate) | Accepted |
| [0002](docs/architecture/adr/0002-tauri-leptos-gui-runtime.md) | Tauri 2 + Leptos as GUI runtime (vs Tauri+Svelte, vs egui) | Accepted |
| [0003](docs/architecture/adr/0003-wgpu-graph-renderer.md) | wgpu for graph rendering at scale (vs SVG-only) | Accepted |
| [0004](docs/architecture/adr/0004-calendar-as-pkm-entity.md) | Calendar as a first-class PKM entity (`.ics` + RRULE) | Proposed |

## Data flow

### Read path : open vault -> render

```
User -> "Open vault" button -> IPC: open_vault(path)
  -> src-tauri: commands::vault_ops::open_vault
    -> path canonicalization + harden_vault_index_permissions
    -> forge-vault: VaultStore::open(path)
      -> filesystem scan (recursive, skip symlinks)
      -> for each .md : parse frontmatter, extract wikilinks
      -> Tantivy: build/load index
      -> manifest: HMAC verify (re-index on tamper)
      -> file watcher: spawn task on notify::Watcher
    -> return VaultSnapshot { notes_count, last_indexed, ... }
  -> IPC response (serde_json) -> Leptos frontend
    -> AppState::vault_snapshot.set(...)
    -> components react: file tree + graph + tag panel
```

### Write path : save note

```
User edits -> Leptos editor -> 500ms debounce
  -> IPC: save_note(path, content)
    -> src-tauri: commands::file_ops::save_note
      -> reject_traversal(path) + canonicalize + starts_with(vault_root)
      -> std::fs::write
      -> forge-vault: index update (single doc reindex)
      -> manifest: rewrite + HMAC sign
      -> audit log: append { ts, action, path }
    -> IPC ack
  -> file watcher detects -> graph snapshot recomputed
  -> frontend re-fetches snapshot delta
```

## Security model

Forgexalith is local-first; there is no server, no telemetry, no external network call by the app itself (dependencies may fetch updates -- see SBOM). The threat model focuses on local filesystem integrity and WebView-level attack surface.

| Layer | Hardening |
|---|---|
| **IPC commands** | All path arguments go through `reject_traversal()` + `canonicalize()` + `starts_with(vault_root)`. 22 commands audited. |
| **WebView CSP** | Locked-down CSP in `tauri.conf.json` (`default-src 'self'`). External links open in user's default browser. |
| **Markdown rendering** | HTML output sanitized via `ammonia` crate (XSS prevention). |
| **Symlinks** | Symlinks detected and skipped during scan and read (prevent escape from vault). |
| **Manifest integrity** | HMAC-SHA256 signature on each save. Verification on load. Tampering triggers full re-index. Key generated at vault creation, stored in `.forge-index/.hmac-key` with restricted permissions. |
| **Audit log** | Append-only JSON Lines in `.forge-index/audit.jsonl`. |
| **Filesystem permissions** | `harden_vault_index_permissions` called in both vault open paths (chmod 600 on Unix, ACL hardening on Windows). |
| **localStorage** | Bounds clamping on deserialization (prevent corruption-induced crashes). |
| **Error messages** | Sanitized -- no full paths leaked to the frontend (privacy + reduce info disclosure). |

For the full threat model and the list of remaining findings (V1-V10), see [docs/security/threat-model.md](docs/security/threat-model.md).

## Performance budget (current baseline)

| Operation | Target | Measured (Phase 19, 700 notes / 17 209 files) |
|---|---|---|
| `cargo check` (incremental) | < 5 s | ~2.5 s |
| WASM build (`trunk build`) | < 60 s | ~28 s |
| Vault open + first index | < 1 s | ~500 ms |
| Tantivy search query | < 50 ms | < 10 ms |
| Graph build (700 nodes) | < 200 ms | ~50 ms |
| Force-directed convergence | < 5 s | ~2.3 s (87 ticks) |
| IPC roundtrip (typical) | < 5 ms | < 2 ms |

Performance is logged via `tracing` (backend, structured) and `performance.now()` (frontend, DevTools console). Criterion benchmarks live in `forge-vault/benches/` and `forge-graph/benches/`; HTML reports are generated in `target/criterion/`.

## Cross-platform stance

Cross-platform support is a workspace principle (see [ADR-0002](docs/architecture/adr/0002-tauri-leptos-gui-runtime.md)). All user-facing features must work on Windows, macOS, and Linux. Native-only code requires explicit justification.

Currently:
- Windows: primary development platform, fully tested.
- macOS: Tauri-supported, untested by maintainer (CI on `macos-latest` runs `cargo check`).
- Linux: Tauri-supported, planned milestone (CI on `ubuntu-latest` runs `cargo check`).

## See also

- [docs/architecture/c4/C4-L1-system-context.md](docs/architecture/c4/C4-L1-system-context.md)
- [docs/architecture/c4/C4-L2-containers.md](docs/architecture/c4/C4-L2-containers.md)
- [docs/security/threat-model.md](docs/security/threat-model.md)
- [docs/audits/LICENCE-AUDIT-REPORT.md](docs/audits/LICENCE-AUDIT-REPORT.md)
- [docs/development/testing-workflow.md](docs/development/testing-workflow.md)

# Changelog

All notable changes to **Forgexalith** (formerly Forgedsidian, renamed 2026-05-01 for trademark hygiene vs Obsidian.md / Dynalist Inc.) are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html). Pre-1.0 releases follow the convention that 0.x.0 may introduce breaking changes; 0.x.y are bug fixes only.

For per-crate changelogs, see each crate directory: `forge-core/CHANGELOG.md`, `forge-vault/CHANGELOG.md`, etc.

## [Unreleased]

(empty -- next changes go here)

## [0.2.0-alpha] - 2026-05-01

### BREAKING (project name only)

- Renamed **Forgedsidian -> Forgexalith**. Project rebranded to eliminate phonetic similarity with Obsidian.md (Dynalist Inc., trademark class 9 software). The new name preserves the metaphor "forge of obsidian-like volcanic glass" via prefix `Forge` + suffix `-lith` (Greek *lithos* = stone). No code change, no API change, no vault format change. All crate names unchanged (`forge-core`, `forge-vault`, `forge-editor`, `forge-graph`, `forge-canvas`, `forge-renderer`, `forge-ui`).

### Migration

- **No user action required**. Vault format unchanged. All public APIs unchanged.
- Download new binary from <https://github.com/btaoldai/forgexalith>. GitHub auto-redirects the old `btaoldai/forgedsidian` URL.
- Tauri identifier changed: `com.therustlab.forgedsidian` -> `com.therustlab.forgexalith`. Existing app data paths (Windows `%APPDATA%\com.therustlab.forgedsidian\`, etc.) will not be migrated automatically; users with prior alpha installs may need to copy their config manually.

### Changed

- Repository renamed: `btaoldai/forgedsidian` -> `btaoldai/forgexalith`.
- Tauri productName, identifier, window title -> `Forgexalith` / `com.therustlab.forgexalith`.
- All documentation, branding, ADRs, C4 diagrams, SBOMs, GitHub templates updated (54 files, 137 insertions / 137 deletions).
- **Dependabot batch** merged 2026-04-30 (5 PRs): uuid 1.23.1 (#4), petgraph (#5), notify v8.2 (#8), pulldown-cmark 0.13 (#9), sha2 0.11 + hmac 0.13 (#11).
- **Wave B1**: rand 0.8 -> 0.10 (commit `3a94903`, PR #13).
- **Wave B2**: tantivy 0.22 -> 0.26 (commit `62e43e6`, PR #14). API change: `TopDocs::with_limit().order_by_score()`.
- **Wave A**: wgpu 24 -> 27 (commit `e951d53`, PR #15). 4 breaking API changes in `forge-renderer` (request_adapter Result type, DeviceDescriptor experimental_features+trace fields, request_device 1-arg signature, RenderPassColorAttachment depth_slice field). Strategy D downgrade: targeting wgpu 27 (not 29) due to upstream windows-rs 0.61.3 vs 0.62.2 mismatch between wgpu-hal 29.0.1 and gpu-allocator 0.28.0 on Windows.

### Security

- **RUSTSEC-2026-0097** closed (rand 0.10, Wave B1).
- 2 advisories acknowledged in `deny.toml` (non-patchable upstream): glib RUSTSEC-2024-0429 (Moderate, transitive via tauri), lru RUSTSEC-2026-0002 (Low, transitive via tantivy).

### Deferred

- **wgpu 27 -> 29** target: blocked by upstream windows-rs version mismatch. Tracked for re-evaluation when wgpu 29.0.2+ resolves the dependency conflict (cf. Run-14 in roadmap).

### Tests

- Baseline 239 tests verts maintained across all 3 Waves and the rename sweep (zero regression).

### Acknowledgements

Trademark hygiene audit assisted by Claude (cf. `AI-CONTRIBUTORS.md`). The rename was a 0-line code-change exercise -- the metaphor and architecture survive intact.

## [0.1.1-alpha] - 2026-04-30

Retroactive entry: this release was tagged on the repository (commit `9c95b9b`) but had not been documented in this file at the time. Entry added 2026-05-01 during Run-13 backfill (CHANGELOG-driven doctrine adoption).

### Security

- **Path-traversal hardening** across all IPC commands and vault file operations (TOCTOU CWE-22, symlinks CWE-59, Windows UNC paths). All file accesses now:
  - Canonicalize before access (resolves `..` and symlinks).
  - Re-validate `starts_with(vault_root)` after canonicalization (TOCTOU defense).
  - Reject UNC paths (`\\?\`, `\\.\`) explicitly on Windows.
  - 20 dedicated regression tests in `src-tauri/tests/commands.rs` (test_reject_traversal_*, test_validate_vault_path_*).

### Tests

- Baseline 239 tests verts maintained.

## [0.1.0-alpha] - 2026-04-29

First public alpha release. The codebase was internally validated through phases 1 to 22b (cf. README "Project status").

> Note (2026-05-01 backfill): the original `[Unreleased]` content has been moved here, since these items shipped with the 2026-04-29 tag.

### Added

- Public release scaffolding: README refundo (origin story + two-repo + AS-IS), ARCHITECTURE.md, AI-CONTRIBUTORS.md, CONTRIBUTING.md, CODE_OF_CONDUCT.md, SECURITY.md, this CHANGELOG, .github/ templates and CI workflows.
- `branding/` directory with logo source and brand README.
- `scripts/prepare_icons.py` for Tauri icon generation.

### Changed

- Audits and SBOM moved to `docs/audits/` (was at workspace root in pre-release internal structure).
- ADRs consolidated under `docs/architecture/adr/` (0001-0004).
- C4 diagrams placed under `docs/architecture/c4/`.

### Security

- Manifest signing with HMAC-SHA256 (forge-vault).
- Append-only audit log in `.forge-index/audit.jsonl`.
- IPC path validation hardened across 22 commands (canonicalize + starts_with vault root + symlink rejection).
- HTML sanitization on markdown output via `ammonia` (XSS mitigation).
- Localstorage bounds clamping on deserialization.

### Highlights at first tag

- Workspace of 8 Rust crates, native Tauri 2 desktop binary plus Leptos 0.7 CSR frontend (compiled to WebAssembly).
- 239 tests green at baseline Phase 19.
- Vault open + full-text search + organic graph view + canvas spatial outliner + drawing layer.
- File watcher with incremental re-indexing.
- Multi-tab editing, command palette (Ctrl+P), wikilink navigation, frontmatter YAML + tag indexing.
- Dependency audit clean (0 CVE) with cargo-audit and cargo-deny. SBOM (SPDX 2.3) provided for both native and WASM artifacts.

### Known limitations

- Linux compatibility planned but not yet validated on the maintainer's setup.
- No theme system yet (dark only).
- No plugin system yet.
- No telemetry (intentional design choice).

[Unreleased]: https://github.com/btaoldai/forgexalith/compare/v0.2.0-alpha...HEAD
[0.2.0-alpha]: https://github.com/btaoldai/forgexalith/compare/v0.1.1-alpha...v0.2.0-alpha
[0.1.1-alpha]: https://github.com/btaoldai/forgexalith/compare/v0.1.0-alpha...v0.1.1-alpha
[0.1.0-alpha]: https://github.com/btaoldai/forgexalith/releases/tag/v0.1.0-alpha

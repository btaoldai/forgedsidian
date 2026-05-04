# Changelog

All notable changes to **Forgexalith** (formerly Forgedsidian, renamed 2026-05-01 for trademark hygiene vs Obsidian.md / Dynalist Inc.) are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html). Pre-1.0 releases follow the convention that 0.x.0 may introduce breaking changes; 0.x.y are bug fixes only.

For per-crate changelogs, see each crate directory: `forge-core/CHANGELOG.md`, `forge-vault/CHANGELOG.md`, etc.

## [Unreleased]

### Fixed

- **Splash progress label** -- Loading splash now displays indexing progress as `"Step X/N - YY%"` (single centered label) instead of the visually concatenated `"Step 4/666%"` produced by the previous flex layout. Root cause: a `<div class="forge-progress" style="width:320px">` nested inside an `align-items:center` flex parent shrunk to content width instead of honoring its declared `width:320px`, which collapsed the inner `display:flex; justify-content:space-between` container -- with no horizontal space left to distribute, the two `<span>` children rendered adjacent. Fix replaces the two-span flex layout with a single centered `<span>` containing the unified format. Resolves ANOM-020 (R4). (#PR-TBD)
- **`criterion::black_box` deprecation in benches (incidental)** -- After Dependabot bump #19 (`criterion 0.5.1 → 0.8.2`), `criterion::black_box` is deprecated in favor of `std::hint::black_box` (stable since Rust 1.66). Surfaced when running `cargo clippy --workspace --all-targets -- -D warnings` locally; the CI workflow `ci.yml` currently runs clippy WITHOUT `-D warnings`, so the deprecation slipped through #19 review. Switched the import in `forge-graph/benches/graph_bench.rs` and `forge-vault/benches/vault_bench.rs` to `use std::hint::black_box;`; the four call sites in each file are unchanged. Restores `cargo clippy --workspace --all-targets -- -D warnings` to a clean baseline. (#PR-TBD)

### Added

- `pub fn format_progress(step: u8, total: u8, pct: u32) -> String` in `forge-ui/src/app.rs` -- pure helper used by the splash screen, returns `"Step X/N - YY%"` or empty string on degenerate inputs (`step == 0` or `total == 0`). Documented + 5 unit tests in `mod splash_format_tests` (typical inputs / step zero / total zero / regression assertion against ANOM-020 separator presence / 3-digit percent edge case). NOTE: `forge-ui` is WASM-only (excluded from the native test workspace, cf. `lib.rs:11`); these tests are not exercised by `cargo test --workspace` and require a future `wasm-bindgen-test` setup to run in CI. Validation here was done via `cargo tauri dev` smoke test.

### Closed (no code change in this release)

- ANOM-017 (CI workflows missing `pull_request` trigger) -- already resolved silently in the v0.2.0-alpha squash merge `e2de989` (R7 closeout 2026-05-03 done-by-prior-fix). Workflows `audit.yml` and `ci.yml` already contain `pull_request: branches: [main]` on `origin/main`. The PR for this release serves as the operational in-vivo validation that GitHub Required status checks now report on PRs.
- ANOM-018 (vault name supposedly hardcoded in header) -- false positive of the 2026-05-01 visual audit (R2 closeout 2026-05-03 done-false-positive). The code in `forge-ui/src/components/toolbar.rs` already reads `state.vault_path` dynamically via `file_name_from_path()` (`forge-ui/src/components/folder_tree.rs`). The audit had observed the literal string "Forgedsidian" in the header and interpreted it as a hardcode, but it was the actual folder name of the vault under test (a vault stored at `Vault-Pro/Dev/Forgedsidian/` in the developer's local filesystem -- a separate planned rename, out of scope for this release).

### Internal

- Sprint UI/UX Audit Post-v0.2.0-alpha started 2026-05-03 (`00-control-center/Roadmap-ready-run/` in the developer vault, not shipped). 7 Runs scaffolded (R1-R7), this release closes 3: R7 done-by-prior-fix (no-op), R2 done-false-positive (no-op), R4 done (the only code change in this release).

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

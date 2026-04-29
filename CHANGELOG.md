# Changelog

All notable changes to Forgedsidian (the workspace) are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html). Pre-1.0 releases follow the convention that 0.x.0 may introduce breaking changes; 0.x.y are bug fixes only.

For per-crate changelogs, see each crate directory: `forge-core/CHANGELOG.md`, `forge-vault/CHANGELOG.md`, etc.

## [Unreleased]

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

## [0.1.0-alpha] -- TBD (first public tag)

First public alpha release. The codebase has been internally validated through phases 1 to 22b (cf. README "Project status").

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

[Unreleased]: https://github.com/btaoldai/forgedsidian/compare/v0.1.0-alpha...HEAD
[0.1.0-alpha]: https://github.com/btaoldai/forgedsidian/releases/tag/v0.1.0-alpha

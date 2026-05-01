# Forgexalith 🦀

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust 1.88+](https://img.shields.io/badge/Rust-1.88%2B-orange.svg)](https://www.rust-lang.org/)
[![Tauri 2](https://img.shields.io/badge/Tauri-2-blue.svg)](https://v2.tauri.app/)
[![Leptos 0.7](https://img.shields.io/badge/Leptos-0.7-purple.svg)](https://leptos.dev/)
[![Status: Early Alpha](https://img.shields.io/badge/Status-Early%20Alpha-red.svg)](#project-status)
[![Built with claude-root-orchestrator](https://img.shields.io/badge/Built_with-claude--root--orchestrator-9146FF.svg)](https://github.com/btaoldai/claude-root-orchestrator)
[![Contributions Welcome](https://img.shields.io/badge/Contributions-Welcome-brightgreen.svg)](CONTRIBUTING.md)

> A modern, open-source Personal Knowledge Management (PKM) desktop application built entirely in Rust.
> Inspired by Obsidian, designed for performance, privacy, and extensibility -- born from the practical need to scale beyond a 17 000-file vault.

---

## Why Forgexalith? (origin story)

My day-to-day knowledge base lives in an Obsidian vault that has grown to 700+ markdown notes across 17 000+ files, spread across infrastructure projects (Docker Swarm on a VPS, Raspberry Pi homelab), MCP servers, Solana trading research, technical references, and a decade of personal admin and notes.

At that scale, the existing tooling started to creak. The graph view became sluggish past a few thousand nodes. Search missed notes whose title did not lexically match my query. The Electron runtime added a heavy memory footprint for what is, at its core, a markdown index. I had no Rust-side API to bulk-script operations on my vault. The on-disk index had no integrity signature, so I could not tell if it had been silently corrupted.

So I started rebuilding the PKM engine from scratch in Rust -- not as an Obsidian fork, not as a plugin, but as a parallel engine that consumes the same vault format (plain markdown + wikilinks + frontmatter YAML) and gives me full control over the indexing, the graph layout, the storage backend, and the security model.

That engine is **Forgexalith**. It exists today as an early-alpha desktop app: Tauri 2 backend, Leptos 0.7 CSR frontend (zero-JS, compiled to WebAssembly), Tantivy full-text search, force-directed graph rendered on wgpu, HMAC-signed manifest, append-only audit log, 12 hardening fixes already applied, and 239 tests green at baseline Phase 19.

The bet of opening the source is simple : if my problems with vault-scale PKM tools are common enough that other people have them too, the modular crate structure (8 crates, MIT-licensed) makes it easy for them to take what works and contribute back what improves it.

## The two-repo story

Forgexalith (this repo) is the **product** -- a Rust PKM you can clone, build, and use today.

[claude-root-orchestrator](https://github.com/btaoldai/claude-root-orchestrator) is the **factory** -- the open methodology of AI-assisted Rust development that built it: orchestration system, skills, routing rules, CLAUDE.md hierarchy, multi-agent coordination patterns.

If you are wondering how a single developer can maintain an 8-crate Rust workspace with 239 tests, ADRs, C4 diagrams, dependency audits, SBOMs, and a 700-note dogfood vault all at once -- the answer lives in the orchestrator repo. Reading both gives you the full picture: not just what was built, but how.

You don't have to use the orchestrator to contribute to Forgexalith. But if you want to apply the same pattern to your own projects, that's where you start.

## Features

**Vault & Storage**
- Open any folder as a vault (700+ markdown notes tested, 17 000+ files supported)
- File watcher with incremental re-indexing (notify v7)
- HMAC-SHA256 signed manifest (tampering triggers full re-index)
- Append-only audit log (`.forge-index/audit.jsonl`)

**Editor**
- Live Markdown preview / Edit mode toggle
- Auto-save with 500 ms debounce
- Wikilink navigation (`[[target]]` -- click to jump, fuzzy resolution)
- Frontmatter YAML parsing + tag indexing
- Multi-tab editing (Ctrl+W / Ctrl+Tab / Ctrl+Shift+Tab)

**Graph**
- Organic force-directed layout (Fruchterman-Reingold)
- Auto-fit camera, zoom, pan, touch + pinch-to-zoom
- Node click navigation (UUID -> file path)
- Interactive node drag (single node + connected nodes follow softly)
- wgpu-accelerated renderer (work in progress)

**Search**
- Tantivy full-text index with title boost (x3)
- Ranked results
- Fuzzy search via command palette (Ctrl+P)

**UI**
- Zero-JavaScript frontend (Leptos 0.7 CSR -> WebAssembly)
- File tree with drag-and-drop, lazy rendering
- Folder hierarchy with create / move / delete (path-traversal hardened)
- Sidebar tabs: Files | Tags
- Command palette (Ctrl+P)
- Canvas spatial outliner with drawing layer
- Dark theme (Obsidian-inspired, CSS @layer architecture)

## Quick start

### Prerequisites

- **Rust 1.88+** with the `wasm32-unknown-unknown` target (`rustup target add wasm32-unknown-unknown`)
- **Tauri CLI v2** (`cargo install tauri-cli --version "^2.0" --locked`)
- **Trunk** (`cargo install trunk --locked`)
- **Linux desktop only** : WebKitGTK 4.1 + libsoup-3.0 (Ubuntu/Debian: `apt install libwebkit2gtk-4.1-dev libsoup-3.0-dev`)

### Build & run

```bash
git clone https://github.com/btaoldai/forgexalith.git
cd forgexalith

# Run in dev mode (hot reload)
cargo tauri dev

# Or build a release binary
cargo tauri build
```

> **First-time vault open** : the Tantivy index is built from scratch on first open. Expect ~20-30 seconds per 1000 notes (smoke-tested at ~3 minutes for an 8 968-file vault on Windows 11 / Ryzen 7 PRO). The window is responsive during indexing -- you'll see progress events. Subsequent opens use the on-disk HMAC-signed manifest cache and are near-instant (incremental diff only).

For the full manual end-to-end test workflow, see [docs/development/testing-workflow.md](docs/development/testing-workflow.md).

## Architecture (TL;DR)

Monorepo workspace, 8 Rust crates, single Tauri 2 desktop binary :

```
forge-core      Shared types, traits, primitives
forge-vault     VaultStore, Tantivy index, file watcher, StorageBackend trait
forge-editor    Wikilink + markdown link extraction, helpers
forge-graph     NoteGraph, GraphSnapshot, force-directed simulation
forge-canvas    Canvas / whiteboard view + drawing layer
forge-renderer  wgpu graph renderer (in progress)
forge-ui        Leptos 0.7 CSR frontend (built via Trunk to WASM)
src-tauri       Tauri 2 backend, IPC commands, CSP injection
```

For the full architecture overview, see [ARCHITECTURE.md](ARCHITECTURE.md) and [docs/architecture/](docs/architecture/) (ADRs + C4 diagrams).

## Project status

**In Build / Early Alpha** -- functional, not yet released. Expect breaking changes.

| Phase | Status |
|---|---|
| 1-15 -- Scaffold + Core + UI Foundation + Graph Rewrite | Complete |
| 16-18 -- Tabs, file watcher, wikilink navigation | Complete |
| 19 -- Frontmatter YAML + tag index + TagsPanel | Complete |
| 20 -- Command palette (Ctrl+P, fuzzy search) | Complete |
| 22 + 22b -- Canvas + drawing layer | Complete |
| Audit licence (cargo audit + cargo deny + SBOM) | Complete (0 CVE) |
| Public release preparation | In progress |
| 21 -- Theme system (dark / light / custom) | Planned |
| 23 -- Plugin system (WASM sandbox) | Planned |
| 24 -- Calendar (.ics, daily notes, RRULE) | Planned |
| Linux compatibility | Planned |

Current baseline: **239 tests green**, `cargo check` ~2.5 s, `cargo build` ~57 s (cold), WASM build ~28 s, 0 CVE in dependencies. Vault smoke-tested up to **8 968 notes / 517 wikilinks / 585 tags** (~3 min initial Tantivy index on Windows, < 20 ms incremental opens afterward).

See [docs/audits/LICENCE-AUDIT-REPORT.md](docs/audits/LICENCE-AUDIT-REPORT.md) for the full licence and dependency audit.

## Documentation

This README is the entrance hall. Everything else lives in dedicated documents :

| Topic | Document |
|---|---|
| Architecture overview (crates, data flow) | [ARCHITECTURE.md](ARCHITECTURE.md) |
| Architecture decision records (ADRs) | [docs/architecture/adr/](docs/architecture/adr/) |
| C4 diagrams (system context + containers) | [docs/architecture/c4/](docs/architecture/c4/) |
| Threat model + security mitigations (F1-F12) | [docs/security/threat-model.md](docs/security/threat-model.md) |
| Licence + dependency audit (0 CVE, SBOM) | [docs/audits/LICENCE-AUDIT-REPORT.md](docs/audits/LICENCE-AUDIT-REPORT.md) |
| Software Bill of Materials (SPDX 2.3) | [SBOM-native](docs/audits/SBOM-native.spdx.json) + [SBOM-wasm](docs/audits/SBOM-wasm.spdx.json) |
| Manual end-to-end test workflow | [docs/development/testing-workflow.md](docs/development/testing-workflow.md) |
| AI-assisted development methodology | [docs/development/methodology.md](docs/development/methodology.md) |
| How to contribute | [CONTRIBUTING.md](CONTRIBUTING.md) |
| Code of Conduct | [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) |
| Security policy + vulnerability reporting | [SECURITY.md](SECURITY.md) |
| AI contributors + training data caveat | [AI-CONTRIBUTORS.md](AI-CONTRIBUTORS.md) |
| Third-party licences (notable deps) | [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md) |
| Changelog (Keep a Changelog format) | [CHANGELOG.md](CHANGELOG.md) |

## Disclaimer (AS-IS clause)

This software is provided **AS IS**, without warranty of any kind, express or implied, including but not limited to the warranties of merchantability, fitness for a particular purpose, and non-infringement. The author makes no guarantees regarding stability or fitness for production use. Use at your own risk.

This is an early-stage / alpha project. Expect bugs, breaking changes, and incomplete features. Do not store irreplaceable data in test vaults until a stable release is tagged.

See [LICENSE](LICENSE) for the full legal text.

## Contributing

Contributions are welcome. This project is in early stages so there are many opportunities to help :

- Bug reports -- file an issue describing the problem and how to reproduce it
- Feature requests -- open a discussion or issue with your idea
- Pull requests -- fork, create a feature branch, submit a PR (squash merge)
- Documentation -- improve README, add examples, write guides

Please read [CONTRIBUTING.md](CONTRIBUTING.md) before submitting a PR. By contributing you agree that your contributions are licensed under MIT (see LICENSE).

For the security policy, see [SECURITY.md](SECURITY.md). For the code of conduct, see [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## AI Contributors & methodology

Forgexalith is built with AI-assisted development as a core, transparent part of the workflow. Claude Code (Anthropic) is the lead orchestrator for code generation, refactoring, and architecture. Perplexity contributes to technical research. Gemini provides independent code review and audit cross-reference.

For the full attribution, methodology, and human-in-the-loop principles, see [AI-CONTRIBUTORS.md](AI-CONTRIBUTORS.md) and the companion repo [claude-root-orchestrator](https://github.com/btaoldai/claude-root-orchestrator).

## License

MIT (c) 2026 Baptiste Ochlafen. See [LICENSE](LICENSE).

Third-party dependencies and their licenses are listed in [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md). Software Bill of Materials (SBOM) for the native binary and the WASM frontend are available in [docs/audits/](docs/audits/).

## Author

**Baptiste Ochlafen** ([@btaoldai](https://github.com/btaoldai))
Independent developer. Also runs [TheRustLab](https://baptiste-ochlafen.fr).

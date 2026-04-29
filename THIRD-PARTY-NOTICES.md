# Third-Party Notices -- Forgedsidian

This file documents third-party assets and notable dependencies used in
Forgedsidian, along with their respective licences.

Last updated: 2026-04-17

## Fonts

| Font | Source | Licence | Usage |
|------|--------|---------|-------|
| Roboto | Google Fonts (CDN) | Apache License 2.0 | Body text |
| Roboto Mono | Google Fonts (CDN) | Apache License 2.0 | Code / monospace accents |

Full licence text: https://fonts.google.com/attribution

## Icons

Application icons in `src-tauri/icons/` are original assets generated via
`cargo tauri icon` from a custom source image. They are distributed under the
same MIT licence as the project.

## Notable Rust Dependencies

The full list of dependencies and their licences is available in the SBOM files:
- `SBOM-native.spdx.json` (621 packages, native workspace)
- `SBOM-wasm.spdx.json` (309 packages, WASM / forge-ui)

### Frameworks

| Crate | Version | Licence | Role |
|-------|---------|---------|------|
| tauri | 2.10.3 | MIT OR Apache-2.0 | Desktop application shell |
| leptos | 0.7.x | MIT | Reactive frontend (CSR/WASM) |
| wgpu | 24.x | MIT OR Apache-2.0 | GPU rendering (forge-renderer) |
| tantivy | 0.22.x | MIT | Full-text search (forge-vault) |
| tokio | 1.x | MIT | Async runtime |

### Cryptography

| Crate | Version | Licence | Role |
|-------|---------|---------|------|
| hmac | 0.12 | MIT OR Apache-2.0 | Vault manifest signing |
| sha2 | 0.10 | MIT OR Apache-2.0 | Hashing |
| rand | 0.8 | MIT OR Apache-2.0 | Random generation |

### Parsing / Data

| Crate | Version | Licence | Role |
|-------|---------|---------|------|
| pulldown-cmark | 0.12 | MIT | Markdown parsing (forge-editor) |
| serde / serde_json | 1.x | MIT OR Apache-2.0 | Serialization |
| petgraph | 0.6 | MIT OR Apache-2.0 | Graph data structures (forge-graph) |

## Licence Distribution Summary

The overwhelming majority of dependencies use MIT, Apache-2.0, or dual
MIT/Apache-2.0 licencing. Other licences present in the dependency tree:
Unicode-3.0, MPL-2.0, ISC, BSD-3-Clause, Zlib, CC0-1.0, Unlicense, BSL-1.0.
All are compatible with the project's MIT licence.

No copyleft-only (GPL, AGPL, LGPL-only) dependencies were detected.

## CDN / External Resources

| Resource | URL | Licence |
|----------|-----|---------|
| Google Fonts API | fonts.googleapis.com | Google Fonts TOS / Apache 2.0 for font files |

No other external CDN scripts or stylesheets are loaded at runtime.

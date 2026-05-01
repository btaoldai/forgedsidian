# Licence Audit Report -- Forgexalith

Date: 2026-04-17
Tools: cargo-deny 0.19.4 + cargo-audit 0.22.1 + cargo-sbom 0.10.0

## Summary

- Audited crates: 621 (native workspace) + 309 (WASM / forge-ui).
- Detected licences: MIT OR Apache-2.0 (majority), MIT, Apache-2.0, Unicode-3.0, MPL-2.0, ISC, BSD-3-Clause, Zlib, CC0-1.0, Unlicense, BSL-1.0, LGPL-2.1-or-later (only in `OR` clauses).
- Licence violations: none (`cargo deny check licenses` = OK).
- CVEs: none (0 vulnerabilities, 23 warnings for unmaintained or unsound crates).
- Third-party assets: Google Fonts (Roboto, Roboto Mono) -- Apache 2.0 / OFL.
- `unwrap()` in production code: 0 (all `unwrap` calls are inside `#[cfg(test)]` blocks).
- IPC: 1 command fixed (`save_note` -- `canonicalize` + `starts_with` added).

## cargo deny details

### Licences
PASS -- all licences are in the allow-list defined in `deny.toml`.
2 warnings for licences listed but never encountered (OpenSSL, Unicode-DFS-2016).

### Advisories (20 errors, all `unmaintained` or `unsound`)

| Crate | Type | Advisory | Source |
|-------|------|----------|--------|
| atk, atk-sys, gdk, gdk-sys, gdkwayland-sys, gdkx11, gdkx11-sys, gtk, gtk-sys, gtk3-macros | unmaintained | RUSTSEC-2024-0411..0420 | Transitive dependency of Tauri v2 (GTK3 Linux bindings). No remediation available until Tauri migrates to GTK4. |
| fxhash | unmaintained | RUSTSEC-2025-0057 | Transitive dependency (selectors -> tauri-utils). Alternative: rustc-hash. |
| instant | unmaintained | RUSTSEC-2024-0384 | Transitive dependency (notify v7). Alternative: web-time. |
| paste | unmaintained | RUSTSEC-2024-0436 | Transitive dependency (wgpu / metal). Alternative: pastey. |
| proc-macro-error | unmaintained | RUSTSEC-2024-0370 | Transitive dependency. |
| unic-char-property, unic-char-range, unic-common, unic-ucd-ident, unic-ucd-version | unmaintained | RUSTSEC-2025-0075..0100 | Transitive dependency (urlpattern -> tauri-utils). |
| rand 0.7.3, rand 0.8.5 | unsound | RUSTSEC-2026-0097 | Unsound when a custom logger uses `rand::rng()`. Low risk (no custom logger in Forgexalith). |
| glib 0.18.5 | unsound | RUSTSEC-2024-0429 | `VariantStrIter` unsoundness. Transitive GTK3 dependency. |
| lru 0.12.5 | unsound | RUSTSEC-2026-0002 | `IterMut` Stacked Borrows violation. Transitive dependency (tantivy cache). |

Justification: all of the above are transitive dependencies of Tauri v2, wgpu, or tantivy. No direct remediation is possible without a major upgrade of these frameworks. Each advisory is explicitly listed with rationale in [`deny.toml`](../../deny.toml).

### Bans (duplicates)
42 warnings for crates appearing in multiple versions (base64, bitflags, hashbrown, syn, thiserror, windows-sys, etc.). This is inherent to the Tauri v2 + wgpu ecosystem. No blocking violation.

### Sources
No errors.

## cargo audit details

- Vulnerabilities: 0
- Warnings (unmaintained / unsound): 23 -- the same crates as cargo deny above.
- No known CVE affects the project.

## SBOM

| File | Packages | Validation |
|------|----------|------------|
| SBOM-native.spdx.json | 621 | WARN -- 632 spdx-tools validation errors (`download_location` uses `registry+https://` instead of a pure URL -- known cargo-sbom limitation, not a real issue) |
| SBOM-wasm.spdx.json | 309 | WARN -- same kind of validation errors |

Both files are generated and usable. The validation errors are cosmetic (a cargo-sbom-specific format quirk for `download_location`).

### Licence distribution (native)

| Licence | Count |
|---------|-------|
| MIT OR Apache-2.0 | 310 |
| MIT | 166 |
| Apache-2.0 OR MIT | 46 |
| Unicode-3.0 | 18 |
| Apache-2.0 OR Apache-2.0 OR MIT | 16 |
| Zlib OR Apache-2.0 OR MIT | 11 |
| Unlicense OR MIT | 9 |
| MPL-2.0 | 7 |
| Apache-2.0 | 6 |
| ISC | 4 |
| BSD-3-Clause | 4 |
| Zlib | 3 |
| Other (CC0, BSL-1.0, LGPL-2.1-or-later in OR clauses) | < 10 |

## Assets

| Asset | Source | Licence | Notes |
|-------|--------|---------|-------|
| Roboto (font) | Google Fonts CDN | Apache License 2.0 | Loaded via `<link>` in `index.html`. |
| Roboto Mono (font) | Google Fonts CDN | Apache License 2.0 | Loaded via `<link>` in `index.html`. |
| Icons (`src-tauri/icons/`) | Generated locally | Original (MIT, with the project) | 9 PNG files + 1 ICO. Standard Tauri sizes (16x16 to 256x256 + `icon.png`). Generated via `scripts/prepare_icons.py`. |

### External JS / CSS
No third-party CDN script or stylesheet detected in `index.html` (apart from Google Fonts, documented above). All CSS is inlined in `<style>`. The frontend is compiled from Rust / Leptos via Trunk (`<link data-trunk rel="rust">`).

## IPC validation

| Command | Takes a path? | Validation | Method |
|---------|---------------|------------|--------|
| `create_note` | Yes (folder, name) | OK | `reject_traversal` on the assembled relative path |
| `create_folder` | Yes (parent, name) | OK | `reject_traversal` on the assembled relative path |
| `delete_folder` | Yes (path) | OK | `reject_traversal` + `canonicalize` + `starts_with(vault)` + vault-root deletion blocked |
| `move_file` | Yes (from, to) | OK | Parent-dir check + `canonicalize` + `starts_with(vault)` |
| `move_folder` | Yes (from, to) | OK | Same + self-containment check + vault-root move blocked |
| `read_file` | Yes (path) | OK | `reject_traversal` + symlink rejection + `canonicalize` + `starts_with(vault)` + 5 MB cap |
| `save_note` | Yes (note_id) | OK (fixed 2026-04-17) | `reject_traversal` + `canonicalize(parent)` + `starts_with(vault)` + final path reassembly |
| `get_note` | Yes (path) | OK | `reject_traversal` + symlink rejection + `canonicalize` + `starts_with(vault)` |
| `open_in_default_app` | Yes (path) | OK | Scheme blocklist + `reject_traversal` + symlink rejection + `canonicalize` + `starts_with(vault)` |
| `open_vault` | Yes (path) | OK | `validate_vault_path` (absolute + parent-dir rejection + `canonicalize` + `is_dir`) |
| `pick_and_open_vault` | OS dialog | OK | Path provided by the system dialog, no user-supplied string |
| `list_all_files` | No | N/A | Scan from `vault_path` state |
| `list_folders` | No | N/A | Same, symlinks skipped |
| `list_tags` | No | N/A | Query the index |
| `notes_by_tag` | No (tag string) | N/A | `strip_prefix` on returned paths |
| `list_notes` | No | N/A | Query the index |
| `search_notes` | No (query string) | N/A | Query the index |
| `get_graph_snapshot` | No | N/A | -- |
| `resolve_wikilink` | No (wikilink string) | N/A | -- |
| `save_canvas_drawings` | No | N/A | Fixed path `.forgexalith/canvas-drawings.json` |
| `load_canvas_drawings` | No | N/A | Same |
| `get_canvas` | No | N/A | -- |

### Fix applied (2026-04-17)
`save_note` was hardened by adding `canonicalize(parent)` + `starts_with(vault_canonical)` + final path reassembly from the canonical parent and the filename. The pattern is now aligned with `read_file` and `get_note`. Modified file: `src-tauri/src/commands/file_ops/read.rs`, lines 89-105.

## Build tooling

| Tool | Version |
|------|---------|
| rustc | 1.88.0 (pinned in `rust-toolchain.toml`) |
| cargo | 1.88.0 |
| trunk | (local dev tool, not installed in the audit environment) |
| tauri-cli | (local dev tool, not installed in the audit environment) |
| wasm-bindgen-cli | (local dev tool, not installed in the audit environment) |
| cargo-deny | 0.19.4 |
| cargo-audit | 0.22.1 |
| cargo-sbom | 0.10.0 |

Note: trunk, tauri-cli, and wasm-bindgen-cli are local build tools that are not required for the audit. Their versions should be verified on the developer machine.

## Verdict

**WARN** -- the project is broadly compliant for an MIT open-source release, with the following caveats:

1. **IPC `save_note`**: FIXED on 2026-04-17 -- `canonicalize` + `starts_with` added post-join (same pattern as `read_file` / `get_note`).
2. **Unmaintained crates**: 20 advisories for transitive dependencies (GTK3, fxhash, instant, paste, proc-macro-error, unic-*). No direct remediation is possible. Accepted risk -- imposed by Tauri v2, wgpu, and tantivy. See [`deny.toml`](../../deny.toml) for the explicit ignore list with rationale.
3. **SBOM validation**: cosmetic errors on the `download_location` format. The SBOMs are usable as-is.
4. **No known CVE**.
5. **Zero `unwrap()` in production code**.
6. **Licences**: all compatible with MIT.

Pre-PASS checklist:
- [x] Fix `save_note` with `canonicalize` + `starts_with` (done 2026-04-17).
- [ ] Verify the versions of trunk, tauri-cli, and wasm-bindgen-cli on the build machine.
- [x] Create `THIRD-PARTY-NOTICES.md` to document fonts and notable dependencies (done 2026-04-17).
- [x] Run `cargo test --workspace` after the fix to validate non-regression (done -- 239 tests green).

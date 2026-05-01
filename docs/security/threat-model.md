# Threat model -- Forgexalith

> Synthesis of the internal security review applied to Forgexalith during phases 14-18 (April 2026). Focus: local-first PKM application, Tauri 2 + Leptos 0.7 CSR (compiled to WebAssembly).

## Scope

Forgexalith is a local-first desktop application:

- Runs as a single user on their machine.
- No telemetry, no outbound network calls by the application itself.
- Reads and writes a vault directory chosen by the user.
- Maintains a local index in `.forge-index/` inside the vault.
- Embeds a WebView (Tauri 2 -> WebKitGTK on Linux, WKWebView on macOS, WebView2 on Windows) to render the Leptos UI.

This document focuses on:

- **Local filesystem integrity** of the vault and the index.
- **WebView attack surface** (CSP, markdown rendering, JavaScript injection paths).
- **IPC boundary** between the WebView (untrusted-ish, in the sense that it processes user-authored content) and the native backend.

Out of scope:

- Network-based attacks (Forgexalith is not a server).
- Multi-user / shared-machine scenarios (the application is intended for a single user; the documentation states this explicitly).
- OS-level side-channel attacks.
- Supply-chain compromise of dependencies (covered separately by `cargo-audit`, `cargo-deny`, and the SBOM in `docs/audits/`).

## Methodology

The review used **STRIDE** as a structuring framework:

- **S** -- Spoofing of identity
- **T** -- Tampering with data
- **R** -- Repudiation
- **I** -- Information disclosure
- **D** -- Denial of service
- **E** -- Elevation of privilege

For each STRIDE category, the review listed concrete attack scenarios applicable to Forgexalith and the mitigations in place.

## STRIDE summary

### Spoofing

- No authentication mechanism in the application (intentional: local-only assumption).
- IPC commands are not signed. Mitigation: the Tauri IPC layer rejects calls from origins other than the embedded WebView.

### Tampering

- The on-disk manifest is signed with HMAC-SHA256. Tampering triggers a full re-index on the next vault open. The HMAC key is generated at vault creation and stored at `.forge-index/.hmac-key` with restricted permissions.
- Notes themselves are not encrypted on disk (acceptable trade-off for a local PKM that needs to interoperate with other tools that read plain markdown).

### Repudiation

- Append-only audit log at `.forge-index/audit.jsonl` records save / move / delete operations, with timestamps and paths.

### Information disclosure

- IPC error messages are sanitized -- no full filesystem paths leak to the WebView.
- The WebView CSP locks down `default-src` to `'self'`. External links open in the user's default browser.
- Markdown HTML output is sanitized via the `ammonia` crate (mitigates XSS through user-authored content).
- Local storage values are bounds-checked on deserialization (mitigates corruption-induced crashes).

### Denial of service

- No rate limiting on IPC commands (acceptable for a local-only application; an attacker who can call the IPC already has user-level filesystem access).
- Large vaults (700+ notes tested, 17 000+ files supported) may cause memory pressure during full re-index. Mitigation: lazy rendering of the file tree, incremental re-indexing on file watcher events.

### Elevation of privilege

- Tauri runs as the user. There is no privilege boundary inside the application beyond what the OS provides.

## Mitigations applied (F1 to F12)

The following 12 hardening fixes were applied during the internal review (April 2026):

| ID | Title | Component |
|---|---|---|
| F1 | XSS sanitization on rendered markdown | `forge-ui/src/components/editor.rs` |
| F2 | Path traversal rejection on `read_file` IPC | `src-tauri/src/commands/file_ops.rs` |
| F3 | Error messages sanitized (no path disclosure) | `src-tauri/src/commands/file_ops.rs` |
| F4 | Platform-agnostic path joining | `src-tauri/src/commands/file_ops.rs` |
| F5 | Defense-in-depth path validation in frontend IPC | `forge-ui/src/ipc.rs` |
| F6 | Local storage bounds clamping | `forge-ui/src/settings.rs` |
| F7 | Removed `unwrap()` panic in editor | `forge-ui/src/components/editor.rs` |
| F8 | Removed redundant CSP injection in JS | `src-tauri/src/lib.rs` |
| F9 | Symlink rejection in scan + read | `src-tauri/src/commands/scan.rs`, `file_ops.rs` |
| F10 | Vault index permission hardening | `src-tauri/src/commands/vault_ops.rs` |
| F11 | HMAC-SHA256 manifest signing | `forge-vault/src/manifest.rs` |
| F12 | Append-only audit log framework | `forge-vault/src/audit.rs` |

## Open findings (not a blocker for early-alpha)

- **CSP could be stronger**: a future iteration will tighten the CSP header (current `default-src 'self'` is acceptable but minimal).
- **Notes plaintext on disk**: optional vault encryption (AES-256-GCM at rest) is on the roadmap but not in v0.1.
- **Manifest reveals vault topology**: the manifest itself is not encrypted; an attacker with read access to `.forge-index/` learns note paths.

## Continuous review

- Weekly automated audit via GitHub Actions: `cargo audit` + `cargo deny check` (see [`.github/workflows/audit.yml`](../../.github/workflows/audit.yml)).
- Dependabot updates Cargo dependencies and GitHub Actions on a recurring schedule (see [`.github/dependabot.yml`](../../.github/dependabot.yml)).
- A licence + dependency audit was performed on 2026-04-17 (see [`docs/audits/LICENCE-AUDIT-REPORT.md`](../audits/LICENCE-AUDIT-REPORT.md)). Result: 0 CVE, 0 licence incompatibility, SBOM (SPDX 2.3) generated for both native and WASM artifacts.

## Reporting an issue

For security-sensitive reports, please use the private channel described in [SECURITY.md](../../SECURITY.md). Do not open a public issue.

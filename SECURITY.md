# Security Policy

Thank you for taking the time to help keep Forgexalith and its users safe.

## Reporting an issue

**Please do not open a public issue for security-sensitive reports.** Public disclosure before a fix is available can put users at risk.

Please use the **GitHub private advisory** channel:

https://github.com/btaoldai/forgexalith/security/advisories/new

This is the only supported reporting channel for security-sensitive issues. It guarantees confidentiality, is integrated with GitHub's CVE assignment workflow, and gives both reporter and maintainer an audit trail.

If you cannot access GitHub for any reason, open a public **non-security** issue asking for a contact channel, without disclosing the vulnerability, and the maintainer will provide one.

When reporting, please include:

- A short description of the issue and its potential impact.
- Steps to reproduce.
- The version (commit hash) and platform (OS, Rust version).
- Any proof-of-concept material, if available.
- Whether you wish to be credited in the disclosure (and how).

## Supported versions

Forgexalith is in early-alpha. Only the latest commit on `main` is actively maintained. Pre-tag development snapshots are not supported.

| Version | Supported |
|---|---|
| `main` (latest) | Yes |
| pre-`v0.1.0-alpha` snapshots | No |

Once `v0.1.0-alpha` is tagged, this policy will be updated to specify which tags are supported (typically the latest release plus the previous one).

## Disclosure timeline

The maintainer aims for the following timeline, in good faith and best-effort:

- **Within 48 hours**: acknowledgment of receipt.
- **Within 7 days**: initial triage and severity assessment.
- **Within 30 days**: a fix or a documented mitigation in `main`.
- **Within 90 days** (or coordinated with reporter): public disclosure, with CVE if applicable.

These timelines may slip for complex issues; if so, the reporter will be kept informed.

## Threat model

Forgexalith is a local-first desktop application. It does not include a server, no telemetry, and the application itself does not make outbound network calls (dependencies may fetch updates -- see SBOM in `docs/audits/`).

The threat model focuses on:

- **Local filesystem integrity**: vault data and the on-disk index must not be tampered with by other processes without detection.
- **WebView attack surface**: the embedded WebView (Tauri 2) must be locked down via CSP. Markdown rendering must be sanitized.
- **IPC boundary**: paths passed from the WebView to the native backend must be validated (canonicalize + traversal rejection + symlink rejection).

Out of scope (for now, unless explicitly added later):

- Network-based attacks (Forgexalith is not a server).
- Multi-user / shared-machine scenarios (the documentation states the application is intended for single-user local use).
- Side-channel attacks on the underlying OS.

For the full threat model and the list of mitigations applied (12 fixes shipped, with the IDs F1-F12), see [docs/security/threat-model.md](docs/security/threat-model.md).

## Past audits

| Date | Type | Outcome | Document |
|---|---|---|---|
| 2026-04-17 | Dependency + license audit (cargo-audit, cargo-deny, cargo-about) | 0 CVE, 0 license incompatibility, SBOM generated for native + WASM artifacts | [docs/audits/LICENCE-AUDIT-REPORT.md](docs/audits/LICENCE-AUDIT-REPORT.md) |
| 2026-04-12 | Internal application security review | 12 fixes applied (F1-F12) | [docs/security/threat-model.md](docs/security/threat-model.md) |

## Acknowledgments

A list of researchers and contributors who have responsibly reported issues will be maintained here as the project matures.

## Notes on the AS-IS clause

Forgexalith is provided AS IS, under the MIT License (see [LICENSE](LICENSE) and the Disclaimer in [README.md](README.md)). The maintainer makes no warranty of fitness for any particular purpose, including security-critical use. Reporting issues helps improve the project for everyone but does not create any obligation of support or fitness.

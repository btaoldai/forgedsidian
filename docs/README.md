# Forgexalith -- Documentation

This directory hosts the project's developer-facing documentation.

## Layout

| Section | Purpose |
|---|---|
| [architecture/](architecture/) | Architecture Decision Records (ADRs) and C4 diagrams |
| [audits/](audits/) | Licence audit, dependency audit, SBOM (SPDX 2.3) |
| [development/](development/) | Build, test, methodology, debugging |
| [security/](security/) | Threat model, hardening notes |
| [contributing/](contributing/) | First-time contributor guides (in development) |

## Quick links

- **Build & run** -- see [development/](development/) (in progress) and the project [README](../README.md).
- **Architecture overview** -- [../ARCHITECTURE.md](../ARCHITECTURE.md).
- **ADRs** -- [architecture/adr/](architecture/adr/).
- **C4 diagrams** -- [architecture/c4/](architecture/c4/).
- **Licence audit (0 CVE)** -- [audits/LICENCE-AUDIT-REPORT.md](audits/LICENCE-AUDIT-REPORT.md).
- **SBOM** -- [audits/SBOM-native.spdx.json](audits/SBOM-native.spdx.json) and [audits/SBOM-wasm.spdx.json](audits/SBOM-wasm.spdx.json).
- **Threat model** -- [security/threat-model.md](security/threat-model.md).
- **Methodology (AI-assisted dev)** -- [development/methodology.md](development/methodology.md).

## Contributing to docs

Documentation contributions are very welcome. See [CONTRIBUTING.md](../CONTRIBUTING.md) for the general contribution process. For documentation specifically:

- Prefer prose to bullet-point lists when explaining concepts.
- Keep code examples minimal, runnable, and pinned to a Cargo / Rust version.
- For ADRs, follow the existing template (Status, Context, Decision, Consequences).
- Cross-link generously between docs (relative links, not absolute URLs).
- For diagrams, prefer formats that render natively on GitHub (Mermaid, ASCII, embedded images via PNG/SVG).

## In progress

Sections currently under construction:

- [ ] `development/build.md` -- platform-by-platform build instructions
- [ ] `development/debugging.md` -- DevTools (WebView), tracing layers, panic patterns
- [ ] `development/coding-conventions.md` -- style guide (Rust + Tauri + Leptos)
- [ ] `contributing/first-issue.md` -- onboarding for newcomers
- [ ] `contributing/pr-flow.md` -- from fork to merge

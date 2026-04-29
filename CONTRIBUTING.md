# Contributing to Forgedsidian

Thank you for considering a contribution. This project is in early stages, so there are many opportunities to help -- from typo fixes to new features.

## Code of Conduct

This project follows the [Contributor Covenant 2.1](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

## How we work

Forgedsidian is built using an open AI-assisted methodology. You don't have to use it to contribute, but reading the companion repo [claude-root-orchestrator](https://github.com/btaoldai/claude-root-orchestrator) helps you decode our commit messages, PR descriptions, and ADRs. See also [AI-CONTRIBUTORS.md](AI-CONTRIBUTORS.md) for how AI assistants are used.

## How to ask questions

For open-ended questions, please use [Discussions](https://github.com/btaoldai/forgedsidian/discussions) (categories: Q&A, Ideas, Show & Tell). Reserve issues for actionable items (bugs, feature requests, documentation gaps).

## How to report bugs

1. Search [existing issues](https://github.com/btaoldai/forgedsidian/issues) first.
2. If your bug is new, open an issue using the **Bug report** template.
3. Provide:
   - Forgedsidian version (commit hash if from source)
   - Operating system and version (Windows / macOS / Linux)
   - Rust version (`rustc --version`)
   - Steps to reproduce
   - Expected vs actual behavior
   - Logs or stack trace if available

## How to propose features

Open an issue using the **Feature request** template. Include:

- The problem you're trying to solve (not just the solution).
- Use cases (daily-driver, power-user, research, education).
- Alternatives you considered.
- Whether you'd be willing to contribute a PR.

For larger features, please open a Discussion first to align on direction before investing in a PR.

## Development setup

### Prerequisites

- [Rust](https://rustup.rs/) 1.88+
- `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- [Tauri 2 CLI](https://v2.tauri.app/start/prerequisites/) -- platform-specific dependencies (WebKit on Linux, etc.)
- [Trunk](https://trunkrs.dev/): `cargo install trunk --locked`

### Build & run

```bash
# Clone
git clone https://github.com/btaoldai/forgedsidian.git
cd forgedsidian

# Compile check (fast)
cargo check --workspace

# Run all tests (239+ at baseline)
cargo test --workspace

# Run in dev mode (Tauri + Trunk hot-reload)
cargo tauri dev
```

For more detail on the build process, see [docs/development/](docs/development/).

## Coding conventions

### Rust style

- Run `cargo fmt --all` before committing. CI rejects unformatted code.
- Run `cargo clippy --workspace --all-targets -- -D warnings`. CI rejects new warnings.
- Avoid `unwrap()` in production code without a justifying comment.
- Prefer `Result<T, E>` with `thiserror` for library errors, `anyhow::Result` for binaries.
- Document public APIs with doc comments (`///`). For complex behavior, include an example.

### Architectural conventions

- Cross-platform by default. Code that works only on one OS requires explicit justification (issue or ADR).
- Modular by default. New features go in the most specific crate; if a feature crosses crates, consider whether the boundary is right.
- Security by default. New IPC commands must validate paths (`reject_traversal` + canonicalize + `starts_with(vault_root)`). New WebView features must respect the CSP.

### Commit conventions

This project uses [Conventional Commits](https://www.conventionalcommits.org/):

- `feat(scope):` -- new feature
- `fix(scope):` -- bug fix
- `docs(scope):` -- documentation only
- `refactor(scope):` -- behavior-preserving change
- `test(scope):` -- tests only
- `chore(scope):` -- build, CI, dep updates
- `perf(scope):` -- performance improvement
- `security(scope):` -- security fix or hardening

Scope examples: `forge-vault`, `forge-ui`, `src-tauri`, `docs`, `ci`, `branding`.

### AI-assisted contributions

If a substantial portion of your PR was generated, refactored, or co-authored with an AI assistant (Claude, Copilot, Cursor, etc.), please:

1. Add a `Co-authored-by:` trailer to the relevant commit message:
   ```
   Co-authored-by: Claude (Anthropic) <noreply@anthropic.com>
   ```
2. Mention it in your PR description (just so reviewers can apply appropriate scrutiny).
3. Make sure CI is green -- AI-assisted code is held to exactly the same quality bar as human-written code.

This is encouraged, not stigmatized. We use AI assistants too. See [AI-CONTRIBUTORS.md](AI-CONTRIBUTORS.md) for our methodology.

## Pull request process

1. **Fork** the repository.
2. Create a feature branch: `git checkout -b feat/your-feature` or `fix/your-bug`.
3. Make your changes. Add or update tests.
4. Run locally: `cargo fmt && cargo clippy -- -D warnings && cargo test`.
5. Commit using Conventional Commits.
6. Push and open a PR against `main`. Use the PR template.
7. CI will run: `check`, `cross-platform / ubuntu-latest`, `cross-platform / windows-latest`, `cross-platform / macos-latest`, `audit` (weekly).
8. Address review feedback.
9. PR will be **squash-merged** when approved and CI is green.
10. The branch will be auto-deleted after merge.

## First-time contributors

Look for issues labeled [`good first issue`](https://github.com/btaoldai/forgedsidian/labels/good%20first%20issue). These are scoped to be approachable :

- Documentation typo fixes
- Simple feature additions in a single crate
- Test coverage for under-tested areas
- Translation contributions (when we add i18n)

If you want to take an issue, comment on it first so we don't duplicate effort.

## License agreement & disclaimer

By contributing to this repository, you agree that your contributions are licensed under the same terms as the project (see [LICENSE](LICENSE) -- MIT).

You acknowledge that this project is provided **AS IS**, without warranty. Maintainers and contributors accept no liability for any damages, data loss, or issues arising from the use of this software.

## Questions

For questions about the contribution process itself, open a [Discussion](https://github.com/btaoldai/forgedsidian/discussions) in the Q&A category.

For security vulnerabilities, see [SECURITY.md](SECURITY.md) -- do **not** open a public issue.

Thank you.

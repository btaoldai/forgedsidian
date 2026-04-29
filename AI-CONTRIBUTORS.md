# AI Contributors -- Forgedsidian

> Forgedsidian is built with AI-assisted development as a **core, transparent part** of the workflow.
> This document explains the role of each assistant, what they did, and how their contributions are tracked.
>
> **The methodology behind this is itself open-source** in a companion repo:
> [claude-root-orchestrator](https://github.com/btaoldai/claude-root-orchestrator) -- the orchestration system, skills, routing rules, and CLAUDE.md hierarchy that powers this dev workflow.

## Why this document exists

Two reasons.

**First: transparency.** A non-trivial fraction of the code in this repository was generated, reviewed, or refactored with the help of AI assistants. Pretending otherwise would be misleading to contributors and to anyone auditing the codebase for security, licensing, or quality. Open-source norms are still emerging on this front; this document is one attempt at clarity.

**Second: reproducibility.** If the methodology I use works, others should be able to apply it. Listing the assistants and their roles is a small contribution to a body of practice on AI-assisted Rust development that the community is figuring out together.

This document is **not** an attempt to make any AI a co-author in the legal sense -- the LICENSE attribution is human. It is a description of process.

## The two-repo story

| | Forgedsidian (this repo) | claude-root-orchestrator |
|---|---|---|
| **What** | The product | The factory |
| **Output** | A Rust PKM you can clone, build, use today | The methodology, skills, routing rules, CLAUDE.md hierarchy |
| **Audience** | PKM users, Rust devs, contributors | Devs who want to apply the same AI-assisted pattern |
| **License** | MIT | (see that repo) |

Reading both gives you the full picture: not just *what* was built, but *how* one developer with TDAH/TSA cognition can manage an 8-crate Rust workspace, 239 tests, multi-agent coordination, ADRs, security audits, and a 700-note dogfood vault without burning out.

Contributors don't need to use the orchestrator to contribute. But understanding it helps decode the structure of commit messages, PR descriptions, and references to internal artifacts (`00-control-center/`, `.claude/logs/`) that may appear in some docs.

## The crew

### Claude Code (Anthropic) -- Lead orchestrator

- **Role**: main coding agent, multi-agent orchestration, code review, architecture
- **Models in rotation**:
  - `claude-opus-4-6` -- critical tasks (audit initial, refactor cross-crate, security architecture, lock-file design)
  - `claude-sonnet-4-6` -- default for medium tasks (documentation, scaffolding, light refactor)
  - `claude-haiku-4-5` -- light tasks (read-only navigation, grep, simple agent delegation)
- **Contributions**: ~90% of the Rust code generation, all ADRs, all security fixes, test scaffolding, refactorings (e.g. dependency inversion `forge-vault` <- `forge-editor`), documentation drafts, this README and AI-CONTRIBUTORS.md
- **Workflow integration**: Cargo workspace, multi-crate, doc-comments-first, Conventional Commits

### Claude (via Cowork mode, Anthropic) -- Project coordination

- **Role**: multi-project orchestration, cross-vault navigation, planning, documentation, publication preparation
- **Models in rotation**: same as Claude Code
- **Contributions**: roadmap, backlog management, session logs, dashboard updates, PUBLICATION-PLAN.md (this open-source release), templates and templates' rationale, cross-project pattern propagation

### Perplexity -- Knowledge base curation

- **Role**: real-time technical research, library comparison, vendor evaluation
- **Contributions**: architecture decision inputs (Tauri vs egui, Tantivy vs alternatives, Leptos CSR vs SSR), real-time RFC reading, cross-checking benchmarks, sourcing recent third-party security advisories

### Gemini (Google) -- Audit second opinion

- **Role**: independent code review, security audit cross-checking, alternative perspective when an architectural choice felt risky
- **Contributions**: threat model peer review, supply-chain audit cross-reference, prose proofreading on FR/EN docs

## How AI contributions are tracked

### In-session (private to the maintainer)

- Claude Code logs each agent session in `.claude/logs/` (orchestrator + agents). These logs are kept private in the maintainer's vault; they are not committed to this repo.
- Backlogs are maintained in dual format (claude-friendly machine-readable + human-friendly markdown).

### In commits (public)

- The maintainer (Baptiste) authors and validates every commit. The author and committer trailers always reflect the human.
- When AI contribution to a specific commit is substantial (a refactor, a new feature, a non-trivial block of code), a **`Co-authored-by:` trailer** is added to the commit message:

```
feat(forge-vault): add HMAC-SHA256 manifest signing

Implements append-only audit log + manifest signature verification.
Tampering triggers full re-index. HMAC key generated at vault creation,
stored in .forge-index/.hmac-key with restricted permissions.

Closes #XX

Co-authored-by: Claude (Anthropic) <noreply@anthropic.com>
```

This is the convention adopted by GitHub, Linux kernel, and many large open-source projects when there is a non-author contributor whose work is reflected in the diff. We extend it to AI-assisted contribution as a transparency mechanism.

### In ADRs

When an Architecture Decision Record is shaped by AI-assisted reasoning, the ADR cites the dialogue that produced the decision. Example: ADR-0002 (Tauri + Leptos vs alternatives) was developed with Claude Opus exploring the trade-offs, Perplexity sourcing benchmarks, and the human making the final call. The ADR documents that.

## The human-in-the-loop principle

| Action | Who decides | Who executes |
|---|---|---|
| Create a feature | Human | AI proposes, human reviews, human commits |
| Architecture change | Human | AI explores trade-offs, human chooses, ADR records |
| Security fix | Human | AI proposes patch, human verifies + tests, human commits |
| Refactor | Human | AI does the bulk, human reviews diff, human merges |
| Push to remote | **Human only** | Human only |
| Publish a release | **Human only** | Human only |
| License compliance | **Human only** | Human only |

AI assistants in this project **do not have repository write access**. They cannot push, commit autonomously, tag releases, or publish anything. They can read the repo, propose changes through the conversation, and ask the human to apply them. The human reviews and decides every time.

## Limitations and disclaimers

- **No warranty for AI-generated code.** Like the rest of the project, anything in this codebase is AS IS (see LICENSE and the Disclaimer in README). The fact that a chunk was AI-generated does not change that.
- **License compliance is human responsibility.** The maintainer reviews dependency licenses (`cargo deny check`, `cargo audit`, `cargo about`) before each major change. AI assistants do not autonomously add dependencies.
- **Security responsibility is human.** Threat model, fix prioritization, and CVE handling are owned by the maintainer. AI assists with research and proposal but does not validate.
- **Bias and hallucination apply.** AI assistants can confidently produce wrong code. The CI pipeline (`cargo test`, `cargo clippy -D warnings`, `cargo fmt --check`, `cargo audit`) is the ground truth. Code that doesn't pass CI doesn't merge, regardless of who wrote it.

## Training data caveat (honest disclosure)

Large language models like Claude are trained on a broad corpus that includes public source code (open-source repositories, books, documentation, websites). Anthropic applies filtering and refusal mechanisms to limit verbatim reproduction of copyrighted code, but **no filter is perfect**.

For Forgedsidian, this means:

- **Best-effort, not guaranteed**: the maintainer has reviewed the codebase for explicit markers (third-party copyrights, attribution comments, references to specific reference projects). The pre-publication audit found no such markers (see commit history). However, **we cannot guarantee with 100% certainty** that no AI-generated block of code resembles, in some part, code seen during training.
- **Idiomatic vs verbatim**: most of the code is idiomatic Rust using public APIs (Tantivy, Tauri, Leptos, petgraph, wgpu) and well-known algorithms (Fruchterman-Reingold, HMAC-SHA256, Tantivy querying). Idiomatic patterns are not, in themselves, copyright violations.
- **Inspiration vs copy**: where the design is inspired by other projects (e.g. Obsidian's organic graph layout), this is acknowledged in the relevant source comment. Inspiration of behaviour or aesthetics is not the same as code reproduction.
- **Fix-on-notice policy**: if you recognize a substantial portion of your own copyrighted code in this repository, please [open a security advisory](https://github.com/btaoldai/forgedsidian/security/advisories/new) or email the maintainer (see [SECURITY.md](SECURITY.md)). The maintainer will investigate and, if confirmed, attribute, rewrite, or remove the affected code.

This disclosure follows the practice adopted by many AI-assisted open-source projects (GitHub Copilot users, Cursor users, etc.). It is not legal advice; it is an honest description of the process.

## Methodology, in detail

For the full picture of how this works -- routing rules, skills, multi-agent waves, sub-agent briefing doctrine, lessons learned -- see the companion repo: [claude-root-orchestrator](https://github.com/btaoldai/claude-root-orchestrator).

If you want to apply the same pattern to your own project, that's where to start.

## Questions, feedback, criticism

This is a relatively new way of working in the open. If you have questions about the methodology, criticism of the approach, or suggestions for how to make the AI contribution more transparent or auditable, please open an issue or a discussion. We are figuring this out collectively.

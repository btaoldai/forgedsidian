# Development methodology -- AI-assisted Rust workflow

> If you have already read [AI-CONTRIBUTORS.md](../../AI-CONTRIBUTORS.md), this page is the more detailed companion. If you have not, start there for the rationale.

## Why this page exists

Forgedsidian is built by a single maintainer using an AI-assisted workflow as a core part of the process. This page documents that workflow at a level of detail useful to two audiences:

1. **Contributors to Forgedsidian** -- so you can decode what you see in commit messages, PR descriptions, and references to internal artifacts (`00-control-center/`, `.claude/logs/`, "Phase 22b", etc.).
2. **Developers who want to apply the same pattern** to their own projects.

The full methodology, the orchestration system, the skills, and the routing rules are open-sourced separately in:

**[github.com/btaoldai/claude-root-orchestrator](https://github.com/btaoldai/claude-root-orchestrator)**

This page is a digest. The companion repo is the authoritative source.

## High-level workflow

```
            +----------------------+
            |   Human (Baptiste)   |
            |   - intent           |
            |   - validation       |
            |   - commit / push    |
            +-----------+----------+
                        |
                        v
            +----------------------+        skills loaded based on intent
            |   skill-root         | <----- (rust-expert, baptiste-code-style,
            |   (orchestrator      |         skill-publication, ...)
            |    selector)         |
            +-----------+----------+
                        |
                        v
            +----------------------+
            |   Claude (Opus /     |
            |   Sonnet / Haiku)    |        models picked by task complexity
            |   - propose          |
            |   - draft code       |
            |   - run tests        |
            |   (in sandbox)       |
            +-----------+----------+
                        |
                        v
            +----------------------+
            |   Diff / proposal    |
            |   ready for human    |
            +-----------+----------+
                        |
                        v
            +----------------------+
            |   Human reviews,     |
            |   tests locally,     |
            |   commits if green   |
            +----------------------+
```

The human is **on the critical path for every commit**. AI assistants do not have repository write access. They produce proposals; the human verifies, runs CI locally, and commits.

## Three layers of guardrails

### 1. Workspace-level rules (`CLAUDE.md` ROOT)

A single root file in the maintainer's workspace defines:

- Identity and conventions (language, tone, formatting).
- Tech stack (Rust + Tauri + Leptos by default).
- Hard-blocking rules (no auto-commit, no emoji in produced files, no unwrap in production code, cross-platform principle).
- Routing rules to specialized skills.

This file is loaded automatically by every Claude session, so the conventions never have to be repeated in chat.

### 2. Project-level rules (`forge-pkm/CLAUDE.md` in the maintainer's vault)

Per-project overrides and additions. For Forgedsidian:

- Locked sections (LOCK) where AI cannot change anything without explicit human approval.
- Sub-agent briefing doctrine -- lessons learned across multiple "waves" of refactoring (e.g. always provide exact API signatures when delegating test-writing to a sub-agent, or it will invent plausible-but-wrong types).
- Mini-swarm interface for parent orchestrators (so cross-project orchestration sees Forgedsidian's status from a known set of files).

### 3. Skill-level rules (loaded from `~/.claude/skills/`)

Specialized "skills" loaded based on the task at hand. The most relevant ones for Forgedsidian:

- `baptiste-code-style` -- doctrine for all Rust / Python code (documentation-first, zero-trust, modular, resilient).
- `rust-expert` -- advanced Rust patterns (async/tokio, error handling, lifetimes, workspaces, macros, performance).
- `skill-publication` -- systematic guard-rail for any public release: AS-IS clause check, no-go on push without explicit approval, audit secrets, audit licences.
- `cybermentor` -- explanatory / tutorial-style content (when documentation needs a structured walk-through tone).

Skills are open-source where applicable -- see the orchestrator companion repo.

## Multi-agent waves

For larger refactors, Claude operates in "waves":

1. **Architect wave** (1 Opus agent) -- proposes the plan, writes ADR if needed.
2. **Executor wave** (parallel Sonnet agents) -- each one tackles a specific module or test file. They cannot push, cannot run cargo (sandbox limitation).
3. **Validator wave** (1 Sonnet agent + human) -- reviews the diff, the human runs cargo locally, CI runs in GitHub Actions.

The lessons learned across these waves are encoded as the **sub-agent briefing doctrine** in `forge-pkm/CLAUDE.md` (e.g. always provide concrete API signatures, always disclose sandbox limitations, never let a wrapper re-introduce a removed dependency).

## Commit attribution convention

Every commit is authored and committed by the human. When a substantial portion of a commit is AI-generated, a `Co-authored-by:` trailer is added:

```
feat(forge-vault): add HMAC-SHA256 manifest signing

Implements append-only audit log + manifest signature verification.
Tampering triggers full re-index. HMAC key generated at vault creation,
stored in .forge-index/.hmac-key with restricted permissions.

Closes #XX

Co-authored-by: Claude (Anthropic) <noreply@anthropic.com>
```

This is a transparency mechanism, not a legal claim. The license (MIT) is attributed to the human author.

## Recommended reading order

If you want to understand or apply this pattern :

1. Start with [AI-CONTRIBUTORS.md](../../AI-CONTRIBUTORS.md) for the why.
2. Then this page for the high-level workflow.
3. Then the companion repo [claude-root-orchestrator](https://github.com/btaoldai/claude-root-orchestrator) for the full mechanics, skills, and templates.

## Feedback

This is a relatively new way of working in the open. If you have observations, criticisms, or improvements, please open a [Discussion](https://github.com/btaoldai/forgedsidian/discussions) -- the methodology evolves through public iteration.

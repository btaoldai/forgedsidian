# ADR 0004 — Calendar as a first-class PKM entity

**Status**: Accepted
**Date**: 2026-04-14
**Phase**: 24 (planned, post Phase 19 tags)

## Context

Users of Forgedsidian need time-based organization alongside the note/graph
model. Calendar events from external systems (Google, Outlook, Nextcloud)
distributed as `.ics` files are the lingua franca of temporal data. A tight
coupling between events and notes enables journaling workflows, meeting-note
capture, and temporal navigation of the knowledge graph.

Obsidian offers calendar features only via third-party plugins (Calendar,
Day Planner, Full Calendar). Forgedsidian integrates the concept natively to
avoid plugin fragmentation and to leverage the Rust + Tauri stack for
reliability (timezone handling, RRULE expansion, file I/O).

## Decision

Introduce a dedicated crate `forge-calendar` for parsing, storing, and
editing calendar events. Events are **first-class entities** in the vault,
stored at `.forge-calendar/events.json` and signed with HMAC-SHA256 (same
mechanism as the manifest — see Phase 16). UI exposes a month-grid view with
click-to-daily-note integration.

### Dependencies

- `icalendar` — `.ics` parsing and writing (RFC 5545 compliant)
- `rrule` — recurrence expansion (Daily/Weekly/Monthly/Yearly)
- `chrono` + `chrono-tz` — datetime and timezone handling
- `uuid` — stable event identifiers (already in workspace)

### Linking model

Events and notes are linked via **two distinct wikilink forms**:

- `[[event:<uuid>]]` — references a specific event by its stable UUID
  (rename-safe, survives event edits)
- `[[YYYY-MM-DD]]` — references the daily note for a given date (Obsidian
  convention, human-friendly)

A single event may carry an optional `related_note: Option<PathBuf>` field
pointing to a Markdown note. Daily notes automatically aggregate their
day's events in a rendered section.

### Scope V1 (Phase 24)

- Import `.ics` via IPC command `import_ics(path)`
- Month view (Leptos grid, day cell, event pill)
- Daily-note auto-linking on click
- CRUD events via forms + IPC
- Export vault events to `.ics` (round-trip)
- RRULE Daily / Weekly / Monthly
- Category / tag filtering (depends on Phase 19 tags)
- Tauri notifications for event reminders

### Out of scope V1

- CalDAV sync (deferred to V2, tracked as Phase 25 candidate)
- OAuth with Google / Microsoft (deferred)
- Advanced iCalendar features (`VALARM` complex actions, `VFREEBUSY`,
  `VJOURNAL`, `VTODO` — the last one may converge later with Canvas ABCDE)

## Consequences

### Positive

- Temporal navigation of the PKM (events + notes co-indexed)
- Obsidian-like daily note workflow natively supported (no plugin needed)
- Interop standard (`.ics`) avoids vendor lock-in
- Independent crate keeps `forge-core` light; parser isolated behind
  `CalendarError` enum (follows the Zero Trust + Resilience doctrines)
- UUID-based linking is rename-safe (solves a common Obsidian pain point)

### Negative / Trade-offs

- New subsystem to maintain (parser, store, UI) — estimated 2 weeks of
  focused work for scope B
- Time zones are historically a source of bugs (mitigated by `chrono-tz` +
  extensive test coverage, per the test-agile doctrine)
- RRULE edge cases (DST transitions, leap years, BYDAY with BYMONTHDAY
  combinations) require thorough testing
- Dual linking form (UUID + daily-date) adds cognitive load — documented
  clearly in user docs

### Open questions

- Should events appear as nodes in the graph view? **Pending user feedback
  in V1 beta** — if yes, forge-graph gains an event-node type.
- Should daily notes be auto-generated on first click, or require explicit
  creation? **Default to auto-create with a confirmation toast** (same
  pattern as Obsidian Daily Notes plugin).
- Notification granularity: per-event explicit reminders only, or also a
  configurable default (e.g. "15 min before")? **Per-event in V1, global
  default deferred to V2.**

## Related

- ADR 0001 — Rust workspace layout (forge-calendar joins the DAG)
- Phase 19 — Frontmatter YAML + tags (prerequisite for category filtering)
- Phase 22 — Canvas view (potential future convergence with VTODO)

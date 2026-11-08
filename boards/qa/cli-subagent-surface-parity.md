---
id: "CLI-010"
title: "CLI subagent surface parity"
priority: "P2"
type: "feature"
area: "CLI"
spec: ""
status: "qa"
predecessors: "CLI-001, CLI-006, GATE-004"
created: "2026-09-21"
updated: "2026-09-23"
---

# CLI subagent surface parity

## Summary

Child of `GATE-004` (parity gap 6). Surface subagent/child sessions in the Rust CLI where a run
surface exists, and stop silently hiding child sessions from session listings.

## Parent

`GATE-004` subagent feature parity (`boards/todo/gate-subagent-feature-parity.md`), gap 6.

## Problem

- The Rust CLI filters out child sessions and never shows them:
  `crates/opencode-cli/src/main.rs:1095,1330,3525,3545` filter `parent_id.is_none()`.
- There is no CLI subagent surface, footer, or child-session view.
- The reference CLI run footer has subagent tabs and data
  (`packages/opencode/src/cli/cmd/run/footer.subagent.tsx`, `subagent-data.ts`).

## Vanilla reference

Reference `f54ce313b99a`:

- `packages/opencode/src/cli/cmd/run/footer.subagent.tsx` and `subagent-data.ts` render subagent
  tabs/details, bootstrapping child session messages and permissions.
- `FooterSubagentState` / `FooterSubagentTab` are part of the run footer types
  (`packages/opencode/src/cli/cmd/run/types.ts`).

## Scope / deliverables

- Decide the Rust CLI's run surface (if it has an interactive run mode) and, where it exists, show
  child/subagent sessions with their agent label and status like the reference footer.
- Where no interactive run surface exists, record that as a documented partial in this card and in
  `GATE-004`, mirroring the `GATE-001` CLI-footer approach.
- Do not silently drop child sessions from listings a user can inspect; either show them nested under
  their parent or provide an explicit child view, and document the choice.
- Ensure cancelling/aborting from the CLI reaches the active subagent session.

## Acceptance criteria

- Child sessions are visible or explicitly navigable in the CLI, or the absence is recorded as a
  documented partial with a reason and a follow-up trigger.
- Where a run surface exists, subagent sessions show label and status and can be inspected.
- CLI abort/cancel reaches the active subagent session.
- No live code path silently filters child sessions without either displaying or documenting them.
- `cargo test -p opencode-cli` (or the CLI's test target) passes.

## Verification

- Manual: run a task from the CLI and confirm the child/subagent session is observable per the chosen
  surface.
- Compare footer behavior against `footer.subagent.tsx` where applicable.
- `cargo check -p opencode-cli`.

## Documented Partial (CLI-010 resolution)

Per GATE-004 Shared Decision 9 and the `GATE-001` precedent, `CLI-010` closes as an explicit
documented partial:

- **No interactive CLI run footer.** The reference subagent footer (`footer.subagent.tsx`,
  `subagent-data.ts`) has no Rust host because the CLI has no interactive run surface: `CLI-002`
  (route `opencode run` through the canonical session runtime) is still blocked by `GATE-002`, and
  `CLI-001`/`CLI-006` only added task-send and status/list surfaces. Follow-up trigger: when
  `CLI-002` lands, add the subagent tabs/details to the run footer.
- **Child sessions are no longer silently dropped.** `session list` and `session find` now include
  child (subagent) sessions with an explicit `parentId` (JSON) and a `Parent (subagent)` column
  (table); `session show` prints the parent and lists child sessions. The only remaining
  `parent_id.is_none()` reads (`resolve_requested_session`, `resolve_base_session`) select the most
  recent *root* for `--continue`; that is root selection, not child filtering.
- **Abort.** The server abort endpoint (`POST /session/{id}/abort`) already cancels a session's
  active background subagents (FEAT-048, via the parent turn's abort token). The CLI has no
  interactive interrupt command yet; wiring a CLI abort to that endpoint is deferred with `CLI-002`
  as part of the interactive run surface.

Status: accepted documented partial for `GATE-004` gap 6.

## Dev Notes

- `crates/opencode-cli/src/main.rs`: `session list` (JSON + table) and `session find` include child
  sessions; JSON rows carry `parentId`; tables show a `Parent (subagent)` column; `session show`
  prints `Parent (subagent)` and lists `Children`.
- Added helpers `session_json_row`, `session_table_header`, `session_table_row` with tests
  `session_json_row_includes_parent_id` and `session_table_row_surfaces_child_parentage`.
- Verification: `cargo fmt --all`; `cargo check -p opencode-cli`; `cargo test -p opencode-cli`
  (7 passed). Manual `session list`/`show` smoke against a workspace with a subagent child is
  available via `cargo run -p opencode-cli -- session list`.

### PR Link

- PR #101 (https://github.com/cchris-p/opencode-modded-rust/pull/101) — `feature/CLI-010-cli-subagent-surface` → `development`.
- Program: GATE-004 (H-009), PR 7/7 (documented partial). Awaiting human test/merge.

### 2026-09-23 - Merged into `development`

- Merged via PR #101 (merge commit `3233ccc`) on explicit user approval; the documented-partial
  resolution was accepted.
- Branch cleanup complete: remote and local `feature/CLI-010-cli-subagent-surface` deleted.
- Card remains in `qa` pending a recorded QA report.

## Related Items

- `GATE-004` subagent feature parity.
- `CLI-002` CLI/`AgentExecutor` tool-loop parity - overlapping CLI dispatch (blocked by `GATE-002`).
- `GATE-001` session prompt queue - precedent for recording a CLI-footer scope as a documented partial.

## Blocked By - 2026-09-22

- `CLI-001` Copy Cline-style CLI task send conventions (prerequisite gate: defines the CLI run surface).
- `CLI-006` Add CLI status visibility for tasks and background sessions (prerequisite gate).
- `GATE-004` subagent feature parity is open; this card is its gap 6 and cannot complete before the gate.
- No handoff covers this card. Its CLI dependency overlaps `CLI-002` (routing, blocked by `GATE-002`), so
  there is no CLI run surface to host a subagent view yet. Do not start until `CLI-001`, `CLI-006`, and
  `GATE-004` land.
- Reference pin is `f54ce313b99a` (current). Re-verify all vanilla line references at that pin before use.
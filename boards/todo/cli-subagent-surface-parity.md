---
id: "FEAT-050"
title: "CLI subagent surface parity"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "todo"
predecessors: ""
created: "2026-09-21"
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

## Related Items

- `GATE-004` subagent feature parity.
- `FEAT-012` CLI/`AgentExecutor` tool-loop parity - overlapping CLI dispatch (blocked by `GATE-002`).
- `GATE-001` session prompt queue - precedent for recording a CLI-footer scope as a documented partial.

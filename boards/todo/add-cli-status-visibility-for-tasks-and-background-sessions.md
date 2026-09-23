---
id: "CLI-006"
title: "Add CLI status visibility for tasks and background sessions"
priority: "P3"
type: "feature"
area: "CLI"
spec: ""
status: "todo"
created: "2026-09-16"
updated: "2026-09-22"
---

# Add CLI status visibility for tasks and background sessions

## Status - 2026-09-22

Reactivated at user request as a co-prerequisite gate for the remaining `CLI-*` stories. Cline-like
functionality (`CLI-001`) plus CLI status visibility must exist before other CLI stories are refined
further.

Prerequisite already in place: `GATE-001` (done) exposes `status` (`idle|busy|queued|retry|...`) plus
`position`/`depth` from `GET /session/status`, which is this card's data source. Do not synthesize
`queued` client-side.

Canonical behavior reference: `wiki/cli-surface.md`.

Handoff: `handoffs/2026-09-22-cli-task-surface-and-status-handoff.md` (`H-006`).

## Summary

Add a CLI-visible status surface for tasks/sessions so command-line workflows can see what is running, waiting, completed, errored, or otherwise actionable without opening the TUI.

## Why this exists

`CLI-001` focuses on Cline-style task send conventions: starting work, sending follow-up messages, and attaching files from the shell. Status visibility is related but separable. It should not bloat the send-task implementation, and it should respect the attach/detach lifecycle decisions tracked elsewhere.

## Product Decisions

- Visible states: `idle`, `busy`, `queued` (with depth/position), `retry`, and `error`; completed is
  derived from assistant message completion. `queued` must come from the `GATE-001` run status, not be
  synthesized client-side.
- Output formats: human-readable plain text by default, plus `--json` for scripts. Rich/TUI styling is
  out of scope.
- Status is read from the existing session/status model (`GET /session/status` and session list data);
  no separate status database.

## Scope

- Define which task/session states are visible from the CLI, such as running, idle, awaiting approval, complete, failed, or detached where supported.
- Add a command or subcommand that lists recent or active tasks/sessions with compact status information.
- Add a command or subcommand that views one task/session status in more detail.
- Expose plain-text output by default and `--json` for scripts; do not add rich/TUI formatting.
- Reuse the existing session/task state model rather than inventing a separate status database.
- Make status useful for sessions that are not currently visible in the TUI when the runtime can observe them.

## Non-goals

- Sending prompts or follow-up chat messages; that is `CLI-001`.
- Defining detach behavior for TUI-launched servers; that is `CLI-004`.
- Deciding same-workspace automatic attach/reuse; that is `CLI-005`.
- Building a full process supervisor or background job manager.
- Cross-machine orchestration.

## Done when

- A user can list relevant tasks/sessions from the CLI with their current status.
- A user can inspect one task/session from the CLI without opening the TUI.
- Status output clearly distinguishes running work from completed or blocked work where the runtime exposes that state.
- The command behavior is documented separately from task send/follow-up behavior.

## Recommended verification

- Start or identify at least one active session and confirm the CLI status command shows it as active/running or equivalent.
- Confirm completed sessions do not look active.
- Confirm the output remains useful in plain terminal usage and, if implemented, JSON output parses cleanly.

## Related Items

- `CLI-001` Copy Cline-style CLI task send conventions
- `FEAT-007` Add advanced coding-session polling
- `CLI-003` Remove local TUI server reuse so every ort run starts a fresh server for the activated workspace
- `CLI-004` Plan explicit detach command behavior for TUI-launched servers
- `CLI-005` Decide whether same-workspace server attach or reuse should exist
- `CLI-007` Add default task target selection for CLI sends
- `CLI-008` Queue CLI task sends while TUI session is open

## Notes

- Split out from the original broad `CLI-001` Cline-workflow placeholder on 2026-09-16.

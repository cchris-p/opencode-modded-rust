---
id: "FEAT-019"
title: "Add CLI status visibility for tasks and background sessions"
priority: "P3"
type: "feature"
area: "FEAT"
spec: ""
status: "todo"
created: "2026-09-16"
---

# Add CLI status visibility for tasks and background sessions

## Summary

Add a CLI-visible status surface for tasks/sessions so command-line workflows can see what is running, waiting, completed, errored, or otherwise actionable without opening the TUI.

## Why this exists

`FEAT-005` focuses on Cline-style task send conventions: starting work, sending follow-up messages, and attaching files from the shell. Status visibility is related but separable. It should not bloat the send-task implementation, and it should respect the attach/detach lifecycle decisions tracked elsewhere.

## Scope

- Define which task/session states are visible from the CLI, such as running, idle, awaiting approval, complete, failed, or detached where supported.
- Add a command or subcommand that lists recent or active tasks/sessions with compact status information.
- Add a command or subcommand that views one task/session status in more detail.
- Decide whether status output supports rich, plain, and JSON formats in the first implementation.
- Reuse the existing session/task state model rather than inventing a separate status database.
- Make status useful for sessions that are not currently visible in the TUI when the runtime can observe them.

## Non-goals

- Sending prompts or follow-up chat messages; that is `FEAT-005`.
- Defining detach behavior for TUI-launched servers; that is `FEAT-017`.
- Deciding same-workspace automatic attach/reuse; that is `FEAT-018`.
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

- `FEAT-005` Copy Cline-style CLI task send conventions
- `FEAT-007` Add advanced coding-session polling
- `FEAT-016` Remove local TUI server reuse so every ort run starts a fresh server for the activated workspace
- `FEAT-017` Plan explicit detach command behavior for TUI-launched servers
- `FEAT-018` Decide whether same-workspace server attach or reuse should exist
- `FEAT-020` Add default task target selection for CLI sends
- `FEAT-021` Queue CLI task sends while TUI session is open

## Notes

- Split out from the original broad `FEAT-005` Cline-workflow placeholder on 2026-09-16.

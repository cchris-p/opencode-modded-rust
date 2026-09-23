---
id: "FEAT-060"
title: "Add a runtime poll registry and TUI surface for outstanding polls"
priority: "P3"
type: "feature"
area: "FEAT"
spec: "wiki/advanced-coding-session-polling.md"
status: "todo"
created: "2026-09-23"
---

# Add a runtime poll registry and TUI surface for outstanding polls

## Summary

Add durable runtime tracking of outstanding polling waits and a TUI surface that shows them, so waits are visible and survive beyond a single tool call.

This is a follow-up card split from `FEAT-007`. It is the deferred runtime/TUI surface for polling and does not change the core polling concept.

## Why this exists

The first slice keeps the wait inside the agent tool. As observables and background-session targets grow, users need to see pending waits and decide when another session's result is adopted.

## Scope

- Define a durable runtime poll object with lifecycle states and cancellation.
- Track outstanding polls at the runtime level, not only inside a tool call.
- Add a TUI surface showing outstanding polls, their observed sources, and their status.
- Decide and document whether the TUI shows all outstanding polls globally or only for the active session.
- Preserve user control over adopting another session's result; do not auto-adopt.

## Non-goals

- Auto-waking paused sessions without explicit user control.
- Cross-repository polling.
- Adding new observable sources (owned by `FEAT-058` and `FEAT-059`).

## Done when

- Outstanding polls are runtime-owned and visible in the TUI.
- Poll lifecycle and cancellation are documented and tested.
- Adopting a polled result remains an explicit user/agent action.
- `wiki/advanced-coding-session-polling.md` reflects the shipped surface.

## Recommended verification

- Run relevant `cargo test` targets for the runtime and TUI crates.
- Run `ort-build`, then `ort`, and confirm an outstanding poll is visible and cancellable.

## Related Items

- `FEAT-007` Add advanced coding-session polling (design parent)
- `FEAT-057` Implement the git ref/branch polling agent tool
- `FEAT-059` Add board lane and background-session observables to polling

## Notes

- This card depends on the tool contract stabilizing in `FEAT-057` before the runtime object is designed.

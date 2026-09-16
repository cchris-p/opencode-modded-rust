---
id: "FEAT-017"
title: "Plan explicit detach command behavior for TUI-launched servers"
priority: "P2"
type: "feature"
area: "FEAT"
status: "todo"
created: "2026-09-16"
---

# Plan explicit detach command behavior for TUI-launched servers

## Summary

Explore and design an explicit detach command/action for `ort`/TUI-launched local servers.

## Context

`FEAT-016` keeps the immediate launcher contract strict: every normal `ort` launch starts a fresh
server, no persisted server record is read or written, and exiting the TUI terminates the server
started for that TUI launch. A separate detach command would intentionally leave the launched
server alive after the TUI exits.

## Scope

- Define what an explicit detach command means in the TUI.
- Decide whether `Ctrl-D` exits and terminates the launch server, while a separate detach action
  leaves the server alive.
- Define how the user discovers and invokes detach.
- Define what URL/session information is shown after detach so the user can intentionally reattach.

## Non-goals

- Reintroducing automatic reuse before the behavior is explicitly approved.
- Changing `FEAT-016`'s no-persisted-record cleanup path.
- Same-workspace auto-attach or automatic server reuse; that is tracked separately in `FEAT-018`.

## Done when

- The desired detach and normal exit semantics are documented clearly enough to implement safely.
- Any required detach-time output or state is explicitly scoped and constrained.
- The relationship to `opencode attach <url>` is defined.

## Related Items

- `FEAT-016` Remove local TUI server reuse so every ort run starts a fresh server for the activated workspace
- `FEAT-018` Decide whether same-workspace server attach or reuse should exist

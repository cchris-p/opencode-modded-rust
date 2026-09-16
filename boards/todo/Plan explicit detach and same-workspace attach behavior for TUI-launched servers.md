---
id: "FEAT-017"
title: "Plan explicit detach and same-workspace attach behavior for TUI-launched servers"
priority: "P2"
type: "feature"
area: "FEAT"
status: "todo"
created: "2026-09-16"
---

# Plan explicit detach and same-workspace attach behavior for TUI-launched servers

## Summary

Explore and design explicit detach behavior for `ort`/TUI-launched local servers, plus whether a
future launch should offer or perform same-workspace attach when a matching server is still alive.

## Context

`FEAT-016` keeps the immediate launcher contract strict: every normal `ort` launch starts a fresh
server, no persisted server record is read or written, and exiting the TUI terminates the server
started for that TUI launch. During FEAT-016 refinement, the user noted that same-workspace attach
could be useful but may also be annoying and should only be implemented after explicit confirmation.

## Scope

- Define what an explicit detach command means in the TUI.
- Decide whether `Ctrl-D` exits and terminates the launch server, while a separate detach action
  leaves the server alive.
- Decide whether future `ort` launches should detect live same-workspace servers and prompt, attach,
  or continue starting fresh servers.
- If detection is desired, define whether a cleanup-only/process record is acceptable and what it is
  forbidden to do.

## Non-goals

- Reintroducing automatic reuse before the behavior is explicitly approved.
- Changing `FEAT-016`'s no-persisted-record cleanup path.

## Done when

- The desired detach, exit, and same-workspace attach semantics are documented clearly enough to
  implement safely.
- Any required records or discovery mechanisms are explicitly scoped and constrained.
- The relationship to `opencode attach <url>` is defined.

## Related Items

- `FEAT-016` Remove local TUI server reuse so every ort run starts a fresh server for the activated workspace

---
id: "FEAT-018"
title: "Decide whether same-workspace server attach or reuse should exist"
priority: "P2"
type: "feature"
area: "FEAT"
status: "todo"
created: "2026-09-16"
---

# Decide whether same-workspace server attach or reuse should exist

## Summary

Decide whether future `ort` launches should detect an already-running server for the same workspace
and offer, prompt for, or automatically perform attach/reuse.

## Context

During FEAT-016 refinement, the user said this would be a cool feature but may be annoying in some
situations, and should be documented as a follow-up item planned for implementation only once there
is "110% confirmation".

This is intentionally separate from `FEAT-017`, which only covers an explicit detach command/action.

## Scope

- Decide whether same-workspace server detection should exist at all.
- Decide whether detection should prompt, attach automatically, or only print a suggestion.
- Define how a matching workspace server would be discovered without reintroducing unsafe stale
  server reuse.
- If a process/server record is proposed, define exactly what it is allowed to do and what it is
  forbidden to do.
- Decide whether detach should write any persisted display/discovery record; this was explicitly
  deferred from `FEAT-017`.
- Define how this differs from explicit `opencode attach <url>`.

## Non-goals

- Implementing same-workspace attach/reuse before explicit 110% confirmation.
- Changing FEAT-016's default behavior: normal `ort` starts fresh and normal TUI exit terminates the
  server it launched.
- Defining the explicit detach command itself; that is tracked in `FEAT-017`.
- Adding a persisted detach/server record without the same explicit 110% confirmation gate.

## Done when

- The project has an explicit go/no-go decision for same-workspace attach/reuse.
- If approved, the desired user interaction and safety constraints are clear enough to implement.
- If rejected, the no-reuse invariant remains documented and this item records why.

## Related Items

- `FEAT-016` Remove local TUI server reuse so every ort run starts a fresh server for the activated workspace
- `FEAT-017` Plan explicit detach command behavior for TUI-launched servers

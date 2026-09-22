---
id: "CLI-005"
title: "Decide whether same-workspace server attach or reuse should exist"
priority: "P2"
type: "feature"
area: "CLI"
status: "hold"
predecessors: "CLI-001, CLI-006"
created: "2026-09-16"
updated: "2026-09-22"
attention: "Human decision gate: implementation requires explicit 110% confirmation of the desired attach/reuse behavior"
---

# Decide whether same-workspace server attach or reuse should exist

## Blocked By - 2026-09-22

- `CLI-001` Copy Cline-style CLI task send conventions (prerequisite gate).
- `CLI-006` Add CLI status visibility for tasks and background sessions (prerequisite gate).
- This is a human decision gate; do not start or resolve it until `CLI-001`/`CLI-006` land and the user
  gives explicit 110% confirmation.

## Summary

Decide whether future `ort` launches should detect an already-running server for the same workspace
and offer, prompt for, or automatically perform attach/reuse.

## Context

During CLI-003 refinement, the user said this would be a cool feature but may be annoying in some
situations, and should be documented as a follow-up item planned for implementation only once there
is "110% confirmation".

This is intentionally separate from `CLI-004`, which only covers an explicit detach command/action.

## Scope

- Decide whether same-workspace server detection should exist at all.
- Decide whether detection should prompt, attach automatically, or only print a suggestion.
- Define how a matching workspace server would be discovered without reintroducing unsafe stale
  server reuse.
- If a process/server record is proposed, define exactly what it is allowed to do and what it is
  forbidden to do.
- Decide whether detach should write any persisted display/discovery record; this was explicitly
  deferred from `CLI-004`.
- Define how this differs from explicit `opencode attach <url>`.

## Non-goals

- Implementing same-workspace attach/reuse before explicit 110% confirmation.
- Changing CLI-003's default behavior: normal `ort` starts fresh and normal TUI exit terminates the
  server it launched.
- Defining the explicit detach command itself; that is tracked in `CLI-004`.
- Adding a persisted detach/server record without the same explicit 110% confirmation gate.

## Done when

- The project has an explicit go/no-go decision for same-workspace attach/reuse.
- If approved, the desired user interaction and safety constraints are clear enough to implement.
- If rejected, the no-reuse invariant remains documented and this item records why.

## Related Items

- `CLI-003` Remove local TUI server reuse so every ort run starts a fresh server for the activated workspace
- `CLI-004` Plan explicit detach command behavior for TUI-launched servers

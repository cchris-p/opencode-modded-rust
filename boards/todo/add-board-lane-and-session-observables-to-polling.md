---
id: "FEAT-059"
title: "Add board lane and background-session observables to polling"
priority: "P3"
type: "feature"
area: "FEAT"
spec: "wiki/advanced-coding-session-polling.md"
status: "todo"
created: "2026-09-23"
---

# Add board lane and background-session observables to polling

## Summary

Extend advanced polling so a session can wait until a board item reaches a lane or a background coding session publishes a result or reaches a runtime state.

This is a follow-up card split from `FEAT-007`. It delivers the background-session coverage that the polling design requires as a first-class target.

## Why this exists

The polling design must support waiting on board movement and on another session becoming ready or publishing a result, including sessions that are not the currently visible TUI session. `FEAT-057` only covers git.

## Scope

- Add a board source that can wait on a card reaching a target lane.
- Add a session source that can wait on a background session reaching a runtime state or publishing a result.
- Define stable, safe handles for identifying a background session as a polling target.
- Keep results compact; return the session result as evidence, not the full transcript, unless a separate explicit request asks for it.

## Non-goals

- A durable runtime poll registry or TUI surface.
- Auto-waking paused sessions.
- Cross-repository polling.

## Done when

- Board lane and background-session observables are available and documented.
- Background sessions are covered as explicit targets, including sessions not visible in the TUI.
- Results remain compact and evidence-backed.
- Tests cover met and timeout paths for both observables.

## Recommended verification

- Run `cargo test -p opencode-tool`.
- Add a background-session test that waits on a session state transition without importing its transcript.

## Related Items

- `FEAT-007` Add advanced coding-session polling (design parent)
- `FEAT-002` Keep sessions running after TUI exit
- `FEAT-057` Implement the git ref/branch polling agent tool

## Notes

- The open question "what stable handles should identify a background session as a polling target?" must be resolved in the spec as part of this card.

---
id: "FEAT-022"
title: "Persist session workspace identity"
priority: "P1"
type: "feature"
area: "FEAT"
spec: ""
status: "todo"
created: "2026-09-18"
---

# Persist session workspace identity

## Summary

Record the workspace directory that owns each session so later list, view, load, and task-target flows can reliably distinguish sessions created from different project directories.

## Why this exists

`ort` now starts a fresh local TUI server for the directory it was launched from, but persisted sessions can still be discovered globally. If `project_dir_a` is the primary workspace because `ort` was run from `project_dir_a`, the user should not see sessions created from unrelated directories when viewing or loading sessions for that workspace.

This card establishes the durable session metadata needed before UI and command surfaces can filter sessions safely.

## Scope

- Persist a canonical workspace identity for every newly created session, derived from the server/workspace that created it.
- Use a stable representation that survives relative path differences, symlinks, and process cwd differences where practical.
- Ensure resumed or loaded sessions retain their original workspace identity instead of being silently rebound to the current process cwd.
- Define how legacy sessions with no workspace identity are represented and surfaced to callers.
- Expose the session workspace identity through the internal session API used by TUI and CLI list/load surfaces.
- Add focused tests or fixtures that create sessions from two different directories and verify their stored workspace identities differ.

## Non-goals

- Implementing every TUI filtering behavior; that is `FEAT-023`.
- Reintroducing automatic server reuse or attach behavior.
- Building remote or multi-machine workspace identity.
- Migrating or guessing workspace identity for all historical sessions beyond a clear legacy/unknown state.

## Done when

- New sessions persist the canonical workspace directory they belong to.
- Session reads expose workspace identity to callers that list, view, load, or target sessions.
- A session created from `project_dir_a` remains associated with `project_dir_a` even if later inspected from `project_dir_b`.
- Legacy sessions without workspace identity have explicit unknown/legacy handling and do not masquerade as belonging to the current workspace.
- Tests or focused verification cover sessions created from at least two different directories.

## Recommended verification

- Run or simulate `ort` from `project_dir_a`, create a session, and inspect persisted session metadata.
- Run or simulate `ort` from `project_dir_b`, create a session, and confirm the metadata differs from the first session.
- Load each session by ID and confirm its original workspace identity is preserved.
- Confirm sessions with missing workspace metadata are surfaced as legacy/unknown rather than current-workspace sessions.

## Related Items

- `FEAT-016` Remove local TUI server reuse so every ort run starts a fresh server for the activated workspace
- `FEAT-020` Add default task target selection for CLI sends
- `FEAT-023` Filter session list and load by workspace
- `BUG-011` ort targets the rust repo workspace and inherits the vanilla openrouter default

## Notes

- Created from the user requirement: when `project_dir_a` is the primary workspace because `ort` was run there, only sessions for that project should be visible/loadable, not sessions created by `ort` in other directories.

---
id: "START-016"
title: "Define structured task state for V1"
priority: "P1"
type: "research"
area: "START"
spec: "wiki/v1.md"
status: "doing"
created: "2026-08-29"
---

# Define structured task state for V1

## Summary

Specify the authoritative task-state model the Rust runtime should use for V1 so bounded work does not depend on transcript history or ad hoc todo state.

## Why this exists

`START-004` found that the current repo persists sessions and lightweight todos, but does not yet define a durable task object with explicit objective, criteria, stage, and completion state.

## Must define

- the minimum persistent task record for V1
- which fields are authoritative for objective, stage, and completion criteria
- how task state relates to sessions, transcript history, and todo items
- which transitions the runtime may perform automatically versus explicitly
- the minimum storage changes required to persist task state safely

## Done when

- A concrete V1 task-state model is documented.
- The model is consistent with `invariants/task-state.md`.
- Follow-up implementation work can be broken down without ambiguity.

## Notes

- Keep this focused on the model and persistence boundary, not full runtime orchestration.
- Coordinate with `START-005` so runtime stages and task state do not drift apart.

## Dev Notes

- Landed the concrete model in `wiki/v1-task-state.md`.
- Expanded `invariants/task-state.md` to capture the new authority and persistence rules.
- Aligned `wiki/v1-runtime-loop.md` to reference the durable task-state document as the schema authority.
- Chose a dedicated `session_tasks` record keyed by `session_id` instead of overloading `SessionStatus`, transcript history, or todo rows.
- Verification: `git diff --check`

## Related Items

- `START-004` Assess current Rust state
- `START-005` Define V1 runtime loop

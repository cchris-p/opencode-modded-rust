---
id: "START-005"
title: "Define V1 runtime loop"
priority: "P1"
type: "feature"
area: "START"
spec: "wiki/v1.md"
status: "done"
created: "2026-08-28"
---

# Define V1 runtime loop

## Summary

Specify the minimal V1 runtime loop that satisfies the core architecture for a serious personal daily-driver on a narrow workflow.

## Why this exists

V1 needs a precise execution model so implementation work does not drift into broad feature development or implicit parity work.

## Must define

- task stages
- task state model
- context construction inputs
- implementation stage boundaries
- review and verification boundaries
- completion criteria for a single bounded task

## Done when

- A concrete V1 runtime loop is documented.
- The loop is consistent with the current invariants.
- Implementation can be broken into smaller engineering tasks without ambiguity.

## Outcome

- Runtime-loop document: `wiki/v1-runtime-loop.md`
- Defined explicit runtime-owned stages: `selected`, `context_prepared`, `implementing`, `verifying`, `reviewing`, `repairing`, and `completed`.
- Established that structured task state, not transcript history, is authoritative for objective, stage, verification, review, and completion.
- Defined stage boundaries for context construction, implementation, verification, review, and reopen behavior.
- Mapped the main downstream implementation cards that should realize this contract.

## Notes

- TUI support is part of V1, but this item is about the runtime loop rather than interface polish.
- The implementer must not be the sole authority on task success.

## Related Items

- `START-016` Define structured task state for V1
- `START-017` Enforce verification and review stages in runtime
- `START-018` Complete TUI approval and question handling
- `START-019` Define local-model-first provider path
- `START-021` Define V1 evaluation task pack
- `START-022` Define review quality rubric and fixtures

## Dev Notes

- Added `wiki/v1-runtime-loop.md` as the concrete V1 runtime contract derived from `wiki/v1.md` and the current invariants.
- Defined the minimum runtime stage model, allowed transitions, task-state authority rules, and completion gate.
- Kept the doc at the architecture-contract level so downstream implementation items can stay focused on persistence, enforcement, TUI handling, and evaluation.

## Verification

- Reviewed `wiki/v1.md`, `wiki/v2.md`, `wiki/v3.md`, `wiki/current-rust-state.md`, `invariants/runtime-lifecycle.md`, `invariants/task-state.md`, `invariants/context.md`, and `invariants/verification.md` for alignment.

## Branch

- `feature/START-005-v1-runtime-loop`

## PR

- https://github.com/cchris-p/opencode-modded-rust/pull/13

## Completion

- Merged into `development` on 2026-08-30.

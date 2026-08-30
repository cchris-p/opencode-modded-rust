---
id: "START-007"
title: "Plan ScopeMux integration"
priority: "P2"
type: "research"
area: "START"
spec: "wiki/scopemux-integration-plan.md"
status: "done"
created: "2026-08-28"
---

# Plan ScopeMux integration

## Summary

Define how `ScopeMux` should integrate with the Rust runtime as an early but non-blocking architecture target.

## Why this exists

`ScopeMux` is strategically important for structural retrieval and context quality, but it is still early and should not destabilize V1 scope.

## Questions to answer

- What abstraction boundary should the runtime preserve so `ScopeMux` can be integrated cleanly later?
- Which retrieval responsibilities stay generic in V1?
- Which capabilities become `ScopeMux` responsibilities once it is mature enough?
- What confidence model is needed for heuristic structural relationships?

## Scope

- Define the retrieval and context-assembly boundary the runtime should preserve in V1.
- Identify what the current Rust runtime should continue doing directly before any `ScopeMux` adoption.
- Define the minimum deferred contract for future `ScopeMux` support.
- Create explicit follow-up work if preserving that boundary requires new runtime abstraction work.

## Implementation Direction

- Ground the plan in the current Rust runtime rather than speculative parity with upstream.
- Treat `ScopeMux` as a future retrieval/input system for task context, not as a replacement for runtime-owned task state, lifecycle, or verification.
- Keep V1 generic enough to work without `ScopeMux`, while preserving a clean injection point for richer structural retrieval later.
- Prefer one clear retrieval abstraction boundary over spreading `ScopeMux` assumptions through prompt construction, tool logic, and TUI flows.

## Deliverables

- A repo-local plan document at `wiki/scopemux-integration-plan.md`.
- A clear statement of what remains generic in V1 versus what future `ScopeMux` support may own.
- Explicit follow-up board items for any required runtime abstraction work.

## Done when

- The integration boundary is documented.
- The minimum deferred contract for future `ScopeMux` support is defined.
- Follow-up tasks exist for any required runtime abstractions.

## Outcome

- Added `wiki/scopemux-integration-plan.md` as the repo-local plan for preserving a future `ScopeMux` retrieval boundary without expanding V1 scope.
- Defined what remains generic in V1 versus what future `ScopeMux` support may own.
- Created `START-025` to add the retrieval-provider boundary needed for later integration.

## Verification

- The plan references current runtime architecture language from `wiki/v1-runtime-loop.md`, `wiki/v1.md`, and `wiki/v2.md`.
- The plan is specific enough to guide later runtime work without reopening basic boundary questions.
- Any new abstraction work is represented by explicit board items rather than implied future intent.

## Dev Notes

- Resumed a stale handoff where `START-006` had already been completed separately.
- Completed the remaining `START-007` planning work in PR #14 and kept the follow-up explicit rather than implied.

## PR Link

- https://github.com/cchris-p/opencode-modded-rust/pull/14

## Completion

- Merged into `development` on 2026-08-30.

## Notes

- This is a planning item, not a requirement to ship `ScopeMux` in V1.
- The goal is to avoid painting the runtime into a corner before `ScopeMux` is ready.

## Related Items

- `START-004` Assess current Rust state
- `START-005` Define V1 runtime loop
- `START-019` Define local-model-first provider path
- `START-025` Add retrieval-provider boundary for task context assembly

---
id: "START-017"
title: "Enforce verification and review stages in runtime"
priority: "P1"
type: "feature"
area: "START"
spec: "wiki/v1.md"
status: "doing"
created: "2026-08-29"
---

# Enforce verification and review stages in runtime

## Summary

Implement runtime-owned review and verification stages so a task cannot reach completion solely because code was produced.

## Why this exists

`START-004` found that the current repo has review and test commands, but completion is still not guarded by explicit verification stages that satisfy the V1 and invariant requirements.

## Scope

- Introduce explicit review and verification states or equivalent runtime-owned gates.
- Prevent direct completion paths that bypass those gates.
- Keep the work focused on stage enforcement rather than broad UI polish.

## Implementation Direction

- Build on `START-016` task state rather than overloading the existing `SessionStatus` values.
- Enforce task-level transitions so implementation cannot move directly from active coding to completed without recorded verification and review outcomes.
- Preserve the distinction between session lifecycle (`active`, `completed`, `archived`, `compacting`) and task runtime stages (`implementing`, `verifying`, `reviewing`, `repairing`, `completed`).
- Reopen failed or incomplete work through the runtime stage model instead of leaving completion as an ad hoc caller decision.
- Keep TUI and API changes limited to what is needed to surface enforced stage progress and blocked completion state.

## Acceptance Detail

- A task cannot be marked complete unless verification has run and review has produced a non-blocking outcome.
- Missing required verification is treated as an incomplete or failed gate, not as success.
- Blocking review findings reopen the task into `repairing`.
- Allowed and rejected transitions are encoded in one authoritative runtime path rather than scattered caller checks.

## Likely Touchpoints

- `crates/opencode-session` runtime and session orchestration
- `crates/opencode-types` task-stage or gate types
- `crates/opencode-storage` if stage results must persist across reloads
- `crates/opencode-server` and `crates/opencode-tui` only where current status reporting must expose enforced stage state

## Verification

- Tests cover the allowed transitions from `implementing` to `verifying` to `reviewing` to `completed`.
- Tests prove direct completion is rejected when verification or review state is missing.
- Tests cover reopen behavior from failed verification and blocking review.

## Done when

- Completion depends on explicit verification.
- Review and verification are modeled as distinct runtime stages or gates.
- The implementation aligns with `invariants/runtime-lifecycle.md` and `invariants/verification.md`.
- Runtime completion rules are encoded in one place that future task execution paths must use.

## Notes

- Use `START-005` and `START-016` as upstream design inputs.
- The implementer must not become the final authority on correctness.
- Current code still exposes a direct `session.complete()` path, so this card should remove or gate that shortcut for V1 task completion semantics.

## Dev Notes

- Added a durable `SessionTask` model and stage/result enums in `crates/opencode-types`.
- Added one authoritative task transition path in `crates/opencode-session` and gated `session.complete()` behind passed verification plus approved review.
- Persisted session task state in a new `session_tasks` storage table and exposed the task record on the server session payload.
- Verification:
  - `cargo test -p opencode-session test_task_transitions_require_verification_and_review`
  - `cargo test -p opencode-session test_failed_verification_reopens_into_repairing`
  - `cargo test -p opencode-session test_blocking_review_reopens_into_repairing`
  - `cargo test -p opencode-server storage_roundtrip_restores_sessions_and_messages`
  - `cargo test -p opencode-session` still has two unrelated pre-existing failures in `instruction::tests::test_find_up_walks_parents` and `instruction::tests::test_find_up_stops_at_stop_dir`

## Related Items

- `START-004` Assess current Rust state
- `START-005` Define V1 runtime loop
- `START-016` Define structured task state for V1

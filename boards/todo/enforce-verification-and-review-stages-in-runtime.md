---
id: "START-017"
title: "Enforce verification and review stages in runtime"
priority: "P1"
type: "feature"
area: "START"
spec: "wiki/v1.md"
status: "todo"
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

## Done when

- Completion depends on explicit verification.
- Review and verification are modeled as distinct runtime stages or gates.
- The implementation aligns with `invariants/runtime-lifecycle.md` and `invariants/verification.md`.

## Notes

- Use `START-005` and `START-016` as upstream design inputs.
- The implementer must not become the final authority on correctness.

## Related Items

- `START-004` Assess current Rust state
- `START-005` Define V1 runtime loop
- `START-016` Define structured task state for V1

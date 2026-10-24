---
id: "FUTURE-PROOFING-002"
title: "Define stable-core and volatile-policy boundaries"
priority: "P1"
type: "feature"
area: "FUTURE-PROOFING"
spec: "wiki/harness-substrate.md"
status: "todo"
created: "2026-10-24"
---

# Define stable-core and volatile-policy boundaries

## Summary

Make the two-layer substrate explicit in code and invariants: a small stable core and a
volatile policy layer, with a one-way dependency rule.

This is the foundational card for `FUTURE-PROOFING-001`. Without it, every later
capability (Jev, Laya, shims, events, cloud) will be woven into the loop and will
require a core change to add or remove.

## Why this exists

The current runtime is still chat-loop-shaped (`wiki/current-rust-state.md`). The V1
task-stage runtime is ahead. If Jev or any other capability lands before the boundary is
explicit, it will be integrated twice.

Change management is expensive because core changes ripple. The fix is not to freeze
the core; it is to make the core small and make volatility live behind it.

## Scope

- Define the stable-core module boundary:
  - session/task state,
  - lifecycle and transitions,
  - tool execution,
  - permission authority,
  - storage,
  - provider abstraction,
  - event bus.
- Define the volatile-policy boundary:
  - prompts and skills,
  - shims and guardrails,
  - decision registry and thresholds,
  - capability profiles,
  - model routing,
  - evaluation packs.
- Define a one-way dependency rule: core may call policy interfaces; policy may not
  be imported by core internals.
- Add an architecture test or lint that fails when a core crate imports a
  policy/provider module.
- Update `wiki/harness-substrate.md` and `invariants/future-proofing.md` with the
  final boundary map.
- Record the decision-boundary stub in the core as the only policy entry point.

## Acceptance criteria

- A table lists every crate/module as core, boundary, or policy.
- The dependency rule is enforced by a test or CI check, not by convention.
- A deterministic stub implements the decision boundary and runs without Jev/Laya.
- Adding a fake decision class requires no core change.
- Removing the stub requires no core change.
- The V1 task-stage runtime can be built on the same boundary.

## Non-goals

- Moving all existing code in this card.
- Deleting the plugin bridge or any current capability.
- Designing the task-stage runtime itself.
- Introducing a new service or process.

## Related

- `FUTURE-PROOFING-001`
- `FUTURE-PROOFING-003`
- `FUTURE-PROOFING-005`
- `DECISION-003`
- `wiki/harness-substrate.md`
- `invariants/future-proofing.md`

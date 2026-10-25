---
id: "FUTURE-PROOFING-002"
title: "Define stable-core and volatile-policy boundaries"
priority: "P1"
type: "feature"
area: "FUTURE-PROOFING"
spec: "wiki/harness-substrate.md"
status: "qa"
created: "2026-10-24"
updated: "2026-10-25"
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

## Implementation - 2026-10-25

The boundary is now explicit in code, docs, and an enforcing test.

- **Decision boundary in the core** (`crates/opencode-core/src/decision.rs`, exported as
  `opencode_core::decision`): typed `DecisionRequest` (class, serializable state, options,
  required confidence, budget, sensitivity, authority tier, policy version) and `DecisionResult`
  (choice, confidence, engine, rationale, cost, policy version). `DecisionProvider` is the
  engine-neutral trait; `DecisionRule` + `DeterministicProvider` are the deterministic rules rung of
  the degradation ladder. Classes are data-driven strings registered by policy code, so adding a
  class is `with_rule(...)` and removing one is `without_class(...)`; an out-of-band choice is
  rejected. No engine, model, or prompt is named in the core.
- **Enforced dependency rule** (`crates/opencode-core/tests/architecture_layers.rs`): classifies
  every workspace crate as core, boundary, policy, or interface, and fails when
  - a core crate depends on a boundary/policy/interface crate,
  - a policy crate depends on anything but core,
  - a boundary crate depends on a process interface,
  - a crate is missing from the map or the map names a missing crate,
  - the core depends on an engine/policy crate (`jev`, `laya`, `scopemux`).
- **Docs**: `wiki/harness-substrate.md` records the crate map, the rationale
  (`opencode-retrieval` is core; `opencode-scopemux` is policy; `opencode-plugin` is core;
  `opencode-session` is boundary because it assembles prompts and legitimately imports policy
  retrieval), the three enforced directions, and the stub semantics. `invariants/future-proofing.md`
  now states the enforced-rule and single-entry-point requirements.
- **Acceptance mapping**:
  - table of every crate/module -> `wiki/harness-substrate.md` "Crate boundary map" +
    `every_workspace_crate_is_categorized`;
  - rule enforced by test -> the four architecture tests;
  - deterministic stub without Jev/Laya -> `DeterministicProvider` + its tests;
  - add a fake class without a core change -> `deterministic_stub_answers_a_fake_class_without_a_core_change`;
  - remove the stub without a loop change -> callers depend on `&dyn DecisionProvider`
    (`a_caller_swaps_providers_without_changing_its_code`); the stub itself is confined to the core
    crate and no loop or adapter names it;
  - V1 task-stage runtime on the same boundary -> documented in `wiki/harness-substrate.md`
    ("Decision boundary stub"), since the contract is serializable and runtime-owned.

Verification (2026-10-25):

- `cargo test -p opencode-core`: 11 unit + 5 architecture tests pass; `cargo fmt --all` clean.
- `cargo test -p opencode-session -p opencode-provider` still green after the core export change.

Deferred to `DECISION-003`: the data-driven registry with owner/threshold/fallback metadata, and the
Jev/Laya engine clients (`DECISION-004`). No runtime loop consumes the boundary yet, by design.

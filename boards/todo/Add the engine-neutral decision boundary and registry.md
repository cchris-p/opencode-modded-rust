---
id: "DECISION-003"
title: "Add the engine-neutral decision boundary and registry"
priority: "P1"
type: "feature"
area: "DECISION"
spec: "wiki/decision-layer.md"
status: "todo"
created: "2026-10-24"
---

# Add the engine-neutral decision boundary and registry

## Summary

Add the stable interface between the runtime and any decision provider, plus a
data-driven registry of decision classes.

The core must be able to ask a typed question and receive a typed answer without knowing
whether the provider is Jev, Laya, a deterministic rule, or a test stub.

## Why this exists

This is the card that prevents Jev from being woven into the loop. If the boundary is not
engine-neutral, the swap test fails and the substrate is not future-proof.

## Scope

- Define `DecisionRequest`:
  - class id,
  - state (minimized, typed),
  - options,
  - confidence floor,
  - cost/latency budget,
  - sensitivity tier,
  - capability profile and policy version.
- Define `DecisionResult`:
  - choice,
  - confidence,
  - engine and engine kind,
  - cost and latency,
  - rationale code,
  - escalated flag.
- Define the `DecisionProvider` trait/interface.
- Add a deterministic stub provider for tests.
- Add a decision registry with class metadata: owner, schemas, authority tier,
  threshold, fallback, budget, evaluation card, retirement criterion.
- Enforce the core-import rule: no engine-specific module in the core.
- Add schema versioning and validation.
- Add tests for unknown class, invalid option, budget exceeded, and missing provider.

## Acceptance criteria

- The runtime can run a decision with only the deterministic stub.
- A new class is added by registry data, not by a core change.
- The core compiles with no Jev/Laya dependency.
- The registry rejects duplicate or unknown classes.
- Authority tier is mandatory and defaults to `advisory`.
- The swap test passes with a second stub implementation.

## Non-goals

- Implementing the Jev/Laya clients (that is `DECISION-004`).
- Writing the evaluation harness (that is `DECISION-006`).
- Adding real decision classes beyond smoke fixtures.
- Defining the task-stage runtime.

## Related

- `DECISION-001`
- `DECISION-002`
- `DECISION-004`
- `FUTURE-PROOFING-002`
- `FUTURE-PROOFING-005`
- `wiki/decision-layer.md`
- `invariants/decision-layer.md`

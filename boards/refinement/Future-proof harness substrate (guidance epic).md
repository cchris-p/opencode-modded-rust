---
id: "FUTURE-PROOFING-001"
title: "Future-proof harness substrate (guidance epic)"
priority: "P1"
type: "epic"
area: "FUTURE-PROOFING"
spec: "wiki/harness-substrate.md"
status: "refinement"
created: "2026-10-24"
---

# Future-proof harness substrate (guidance epic)

## Summary

This is a **living guidance epic**, not a delivery epic. It defines and enforces the
absorption-resistant substrate for `opencode-modded-rust`: the small stable core, the
volatile policy layer, the engine-neutral decision boundary, capability profiles, shim
retirement, owned data, and first-class efficiency and trust.

It stays open. Concrete work spawns from it as `FUTURE-PROOFING-*` cards or into the
`DECISION-*` epic. It is never promoted to `done`; it is reviewed and revised.

## Why this exists

Agent harnesses are exposed to a **data flywheel**: a harness makes harder tasks usable,
more users attempt them, more data reaches the model labs, models absorb the capability,
and the harness's model-compensating scaffolding becomes redundant. The harness that
wins is not the one with the most scaffolding; it is the one that can add, swap, and
remove scaffolding cheaply as models change.

That is a change-management problem, not a feature problem. It needs an explicit
substrate, not good intentions.

## What "future-proof" means here

Future-proof means **low adaptation cost to an unknown future change**, not accurate
prediction. The operative test is the eight-test suite:

1. swap the decision engine without touching the loop;
2. delete any shim or decision class without breaking the runtime;
3. add a decision class without a core change;
4. adopt a model by adding a profile and evaluation, not a rewrite;
5. complete a real task with no hosted decision engine;
6. replay a decision log and reproduce the behavior;
7. hit a spend cap and degrade gracefully;
8. explain every decision from the log.

A change that fails a test is a core change and needs explicit justification.

## Guidance (normative)

- Keep the core small: session/task state, lifecycle, tools, permissions, storage,
  provider abstraction, event bus.
- Put volatility in policy: prompts, skills, shims, decision registry, capability
  profiles, thresholds.
- Separate decisions from generation. System 1 selects; the model generates.
- Classifiers propose; deterministic rules, invariants, and humans dispose.
- Make every shim deletion-ready: owner, firing metric, kill switch, retirement path.
- Own the data: decision log, outcomes, provenance, sensitivity tiers, egress policy.
- Measure efficiency and trust as first-class axes, not tie-breakers.
- Expose fewer internals; version policy independently; canary and roll back.
- Degrade without a hosted dependency.

## How this epic is used

- Stays in `refinement` as a living reference.
- New structural work is assessed against the eight tests before implementation.
- New action items are spawned as `FUTURE-PROOFING-*` cards, or into `DECISION-*` when
  they are specific to the System-1 layer.
- Quarterly, or on a major model release, run the absorption review and retire shims.
- If an invariant changes, update `invariants/future-proofing.md` and this epic together.

## Action items

### Substrate

- `FUTURE-PROOFING-002` Define stable-core and volatile-policy boundaries
- `FUTURE-PROOFING-003` Add capability profiles and a shim registry
- `FUTURE-PROOFING-004` Version and canary the policy layer independently
- `FUTURE-PROOFING-005` Build the eight-test future-proofing conformance suite
- `FUTURE-PROOFING-006` Establish the absorption-review and shim-retirement cadence

### Measurement and data

- `FUTURE-PROOFING-007` Build the harness efficiency scoreboard against vanilla
- `FUTURE-PROOFING-008` Define decision-log egress retention and privacy policy
- `FUTURE-PROOFING-009` Define the owned data-flywheel contract

### Next frontier and trust

- `FUTURE-PROOFING-010` Promote event-wake and software-factory primitives
- `FUTURE-PROOFING-011` Future-proof the durable and cloud runtime
- `FUTURE-PROOFING-012` Future-proof trust and spend controls

## Execution order

1. `FUTURE-PROOFING-002` — boundaries (foundation).
2. `FUTURE-PROOFING-005` — eight-test suite (after 002; can run in parallel with 003).
3. `FUTURE-PROOFING-003` — capability profiles and shim registry.
4. `FUTURE-PROOFING-004` — policy versioning (after 003).
5. `FUTURE-PROOFING-007` — efficiency scoreboard (parallel; establishes the baseline).
6. `FUTURE-PROOFING-006` — absorption cadence (after 003 and 007).
7. `FUTURE-PROOFING-008` — decision-log egress policy (prerequisite for `DECISION-005`).
8. `FUTURE-PROOFING-009` — data-flywheel contract (after 008).
9. `FUTURE-PROOFING-012` — trust and spend (parallel, after 002).
10. `FUTURE-PROOFING-010` — event/wake primitives (P2, parallel).
11. `FUTURE-PROOFING-011` — durable/cloud runtime (after 002 and 010).

The `DECISION-*` epic depends on 002, 003, and 008; the rest can proceed in parallel.

## Non-goals

- Predicting the next model or vendor.
- Freezing the core so nothing can change.
- Chasing parity with OpenCode for its own sake.
- Treating benchmark scores as proof of future-proofness.
- Exposing harness internals as user contracts.
- Giving classifiers authority over permissions or dispositions.

## Done when

This epic has no `done` state. It is reviewed when:

- a new model class is adopted,
- a new decision class is added,
- a major structural change is proposed,
- the absorption review runs,
- an invariant in `invariants/future-proofing.md` changes.

## Related

- `wiki/harness-substrate.md`
- `invariants/future-proofing.md`
- `wiki/decision-layer.md`
- `DECISION-001` System-1 decision layer (Jev-Laya) epic
- `wiki/decision-log-egress.md`
- `wiki/post-jev-roadmap.md`
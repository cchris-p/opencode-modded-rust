---
id: "DECISION-015"
title: "Post-Jev absorption audit and demotion plan"
priority: "P2"
type: "research"
area: "DECISION"
spec: "wiki/post-jev-roadmap.md"
status: "todo"
created: "2026-10-24"
---

# Post-Jev absorption audit and demotion plan

## Summary

After the first Jev classes are in production, audit every decision class against the
current models and decide:

- keep as System 1,
- demote to deterministic rules,
- replace with native model behavior,
- remove entirely,
- expand to a new frontier.

This is the absorption review applied specifically to the decision layer.

## Why this exists

The flywheel will absorb some decisions. A class that was valuable when a model could
not self-check becomes token overhead when it can. The audit makes that visible and
forces a decision.

## Scope

- Re-run the harness-vs-model comparison on current models.
- For each decision class, measure:
  - firing rate,
  - accuracy and calibration,
  - cost and latency,
  - incremental value over the current model behavior,
  - whether a deterministic rule now covers it,
  - whether a newer model handles it natively.
- Classify each class: keep, demote, remove, replace, expand.
- Update capability profiles and the decision registry.
- Record **absorption lag** for each demoted/removed class.
- Decide the next frontier: events/coordination, cloud/durable runtime, or trust/spend.
- Open follow-up cards for removals and expansions.

## Acceptance criteria

- Every active class has an audit result and a decision.
- At least one class is demoted or removed, or a documented reason says why not.
- Capability profiles are updated.
- Absorption lag is recorded.
- The next-frontier decision is recorded with a rationale.
- Removals follow the normal board/PR flow and pass the delete test.

## Non-goals

- Removing a class that still carries measurable value.
- A full benchmark platform.
- Changing the default model based on one audit.
- Replacing the periodic absorption review (`FUTURE-PROOFING-006`).

## Related

- `DECISION-001`
- `DECISION-006`
- `DECISION-014`
- `FUTURE-PROOFING-006`
- `wiki/post-jev-roadmap.md`
- `wiki/harness-substrate.md`

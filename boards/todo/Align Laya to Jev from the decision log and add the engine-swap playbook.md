---
id: "DECISION-014"
title: "Align Laya to Jev from the decision log and add the engine-swap playbook"
priority: "P1"
type: "feature"
area: "DECISION"
spec: "wiki/post-jev-roadmap.md"
status: "todo"
created: "2026-10-24"
---

# Align Laya to Jev from the decision log and add the engine-swap playbook

## Summary

Turn the decision log into a fine-tuning dataset, align a Laya checkpoint to Jev, and
write the playbook for swapping engines per class.

The goal is to remove per-request cost and data egress where a local model is competitive,
without losing Jev's accuracy on the classes where it still wins.

## Why this exists

Jev is hosted, paid, and sees state. The durable position is not "we depend on Jev"; it
is "Jev is the preferred engine until a local model is good enough, and we can prove it."

`STD-089` already defines the umbrella and the benchmark. This card is the product-side
execution.

## Scope

- Export training data from the decision log using `wiki/decision-log-egress.md` rules.
- Split by task and time, not randomly, to avoid leakage.
- Fine-tune a Laya checkpoint (English/multilingual/typed-decisions) on the training
  split.
- Evaluate on held-out and out-of-distribution sets using `DECISION-006`.
- Compare per class: accuracy, calibration, cost, latency, escalation.
- Define the swap threshold per class: when local is good enough.
- Write the engine-swap playbook:
  - preconditions,
  - policy/profile change,
  - shadow/canary,
  - rollback,
  - cost and egress verification.
- Swap at least one class in a controlled test and record the result.
- Keep Jev as the preferred engine for classes where Laya is not competitive.

## Acceptance criteria

- A fine-tune export exists with lineage and retention.
- A Laya checkpoint is evaluated against Jev on held-out and OOD data.
- At least one class passes the swap test and the offline test.
- The cost saving is measured, not assumed.
- Egress reduction is measured for at least one sensitive class.
- The playbook is documented and repeatable.
- Rollback to Jev is one policy change.

## Non-goals

- Replacing Jev everywhere.
- Training a foundation model.
- Publishing model weights.
- Removing the hosted engine path.
- Using private data without the egress policy.

## Related

- `DECISION-001`
- `DECISION-004`
- `DECISION-005`
- `DECISION-006`
- `DECISION-015`
- `STD-089` Jev/Laya umbrella
- `TP-026` / `TP-029` (benchmark and economics reference)
- `wiki/post-jev-roadmap.md`
- `wiki/decision-log-egress.md`

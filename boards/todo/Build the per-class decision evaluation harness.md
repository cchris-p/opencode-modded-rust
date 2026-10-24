---
id: "DECISION-006"
title: "Build the per-class decision evaluation harness"
priority: "P1"
type: "feature"
area: "DECISION"
spec: "wiki/decision-layer.md"
status: "todo"
created: "2026-10-24"
---

# Build the per-class decision evaluation harness

## Summary

Build the evaluation harness that decides whether a decision class is allowed to act.

It measures accuracy, precision/recall, calibration, cost, latency, escalation, and
generalization for each class, on held-out and out-of-distribution data.

## Why this exists

The `transcript-pulls` benchmark proved the economics and calibration of Jev on one
classification task. It did not prove Jev is accurate on permissions, tool repair, or
review triage. Every class needs its own evidence.

## Scope

- Define an evaluation dataset format per class: state, question, options, ground truth,
  source, sensitivity, split.
- Ground truth may be human-labelled or human-reviewed; machine-drafted labels must be
  marked.
- Implement metrics:
  - accuracy, per-class precision/recall/F1,
  - calibration (ECE or reliability curve),
  - cost per decision,
  - p50/p95 latency,
  - escalation rate,
  - agreement with the current path.
- Implement splits: train/validation/held-out/test, by task and time where possible.
- Add out-of-distribution sets: different repo, model, language, or failure shape.
- Add a report format that is easy to review and commit.
- Add a replay mode that runs a policy/engine version against a frozen set.
- Add a promotion gate: a class cannot leave shadow without a passing report.
- Reuse the `transcript-pulls` benchmark shape where it fits; do not rebuild that
  pipeline here.

## Acceptance criteria

- At least one class has a real evaluation set and a report.
- The report includes calibration and cost, not just accuracy.
- Held-out and OOD results are reported separately.
- The harness runs without live credentials in CI using recorded fixtures.
- A failed or null result blocks promotion and is recorded.
- The report names the exact engine, model, and policy version.

## Non-goals

- A public leaderboard.
- Labelling every class in this card.
- Replacing human review.
- Optimizing for in-distribution gains.
- Training a model.

## Related

- `DECISION-001`
- `DECISION-005`
- `DECISION-007`
- `DECISION-008`
- `DECISION-009`
- `DECISION-014`
- `wiki/decision-layer.md`
- `wiki/agent-evaluation-strategy.md`

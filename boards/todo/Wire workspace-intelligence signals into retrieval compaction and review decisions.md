---
id: "DECISION-016"
title: "Wire workspace-intelligence signals into retrieval compaction and review decisions"
priority: "P2"
type: "feature"
area: "DECISION"
spec: "wiki/decision-layer.md"
status: "todo"
created: "2026-10-24"
---

# Wire workspace-intelligence signals into retrieval compaction and review decisions

## Summary

Make the Workspace Intelligence Protocol a first-class input to three decision classes:

- retrieval sufficiency,
- compaction trigger,
- review triage.

The map supplies evidence; the decision layer decides; the runtime enforces. The map never
gains authority.

## Why this exists

The decision layer can only be as good as its evidence. A bag of generic retrieval
results is weaker than a map that knows anchors, deltas, observability, duplicates, and
token estimates. Conversely, the map should not decide how much context to include; that
is a runtime/decision responsibility.

## Scope

- Retrieval sufficiency:
  - input: map candidates, anchors, coverage, confidence, token estimate;
  - output: sufficient, read more, read a different source, ask.
- Compaction trigger:
  - input: delta size, token estimates, spans that must survive;
  - output: compact now, defer, retention set.
- Review triage:
  - input: delta, observability, duplicate/refactor signals, changed symbols;
  - output: blocking, non-blocking, noise, approve.
- Define the request/response fields at the decision boundary.
- Preserve the one-way rule: map -> decision -> runtime.
- Keep the generic-provider path working when WIP is absent.
- Record the map version, representation, and confidence in the decision log.
- Evaluate each decision class with and without map input.

## Acceptance criteria

- Each decision class can consume map evidence without importing provider internals.
- A missing map or unsupported language falls back to generic behavior.
- The map cannot advance a stage, verify, review, complete, or change a disposition.
- The decision log records map version/provenance.
- At least one class shows a measurable improvement or a documented null result.
- The runtime retains inclusion thresholds and budgets.

## Non-goals

- Replacing the decision boundary.
- Giving the map authority.
- Building new map capabilities (those are `SCOPE-*` cards).
- Promoting a class without evaluation.

## Related

- `DECISION-001`
- `DECISION-006`
- `DECISION-009`
- `FUTURE-PROOFING-013`
- `FUTURE-PROOFING-014`
- `wiki/decision-layer.md`
- `wiki/workspace-intelligence.md`
- `invariants/workspace-intelligence.md`

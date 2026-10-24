---
id: "DECISION-002"
title: "Inventory and baseline harness decision points"
priority: "P1"
type: "research"
area: "DECISION"
spec: "wiki/decision-layer.md"
status: "todo"
created: "2026-10-24"
---

# Inventory and baseline harness decision points

## Summary

Produce a complete inventory of every decision point in the harness, with its current
implementation, cost, latency, accuracy or error rate, sensitivity, authority tier, and
candidate System-1 class.

This is the baseline that justifies each decision class and measures whether the decision
layer actually improves anything.

## Why this exists

Without an inventory, the decision layer becomes a solution looking for problems. Without
a baseline, "Jev is cheaper" is not measurable per class.

## Scope

- Inventory decisions across:
  - permissions and tool access,
  - agent/tool selection,
  - model/provider/tier selection,
  - loop continuation, retry, and escalation,
  - compaction and context assembly,
  - retrieval sufficiency,
  - failure classification,
  - review severity,
  - board/task classification,
  - memory write/recall,
  - safety screening.
- Record the exact code location for each.
- Record current behavior, cost, latency, and error rate.
- Record the sensitivity tier of the state involved.
- Record the proposed authority tier (advisory/gated/delegated/forbidden).
- Rank by expected value: cost saved, reliability gained, risk.
- Write the inventory to `docs/decision-inventory.md` (or a wiki section if it remains
  planning-only).

## Acceptance criteria

- Every current decision point has a row with code location and owner.
- Each row states whether it is deterministic, generative, or absent.
- Each row has a baseline measurement or an explicit "unmeasured" flag with a plan.
- Each row has a sensitivity tier and a proposed authority tier.
- The inventory is linked from `wiki/decision-layer.md`.
- At least the Wave 1 classes are fully specified.

## Non-goals

- Implementing any decision class.
- Benchmarking Jev on every class.
- Changing current behavior.
- Inventing decisions the harness does not make.

## Related

- `DECISION-001`
- `DECISION-003`
- `DECISION-007`
- `DECISION-008`
- `DECISION-009`
- `wiki/decision-layer.md`

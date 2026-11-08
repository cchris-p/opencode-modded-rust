---
id: "FUTURE-PROOFING-006"
title: "Establish the absorption-review and shim-retirement cadence"
priority: "P1"
type: "chore"
area: "FUTURE-PROOFING"
spec: "wiki/harness-substrate.md"
status: "todo"
created: "2026-10-24"
---

# Establish the absorption-review and shim-retirement cadence

## Summary

Create a recurring review that answers two questions on every model release and at a
regular interval:

1. Which harness shims and decision classes has the model absorbed?
2. Which can be deleted, demoted, or moved from policy to deterministic rules?

This is the operational form of "always ahead of the flywheel."

## Why this exists

Absorption is invisible without a cadence. A shim that was necessary six months ago can
become pure token cost today. Without a review, the harness accumulates scaffolding,
change-management cost, and latency.

## Scope

- Define the review trigger: every model release, every major provider change, and a
  quarterly baseline.
- Re-run the harness-vs-model evidence on current models and a small real task pack.
- Compare against the last review: shim firing rates, decision accuracy, cost, latency.
- Classify each shim/class as keep, demote, delete, or replace with deterministic rule.
- Open removal cards automatically for zero-firing or absorbed items.
- Publish a short review note: what changed, what was removed, what was added.
- Track **absorption lag**: model release date -> profile update; obsolete shim date ->
  removal.

## Acceptance criteria

- The cadence, owner, and trigger are documented.
- A review note exists for the first run.
- At least one shim or class is classified for removal/demotion.
- Absorption lag is recorded.
- Removal follows the normal board/PR flow.
- No shim is kept without a stated reason.

## Non-goals

- Retiring shims that still carry measurable value.
- Rebuilding the full evaluation platform.
- Benchmarking every model.
- Changing the model default based on one review.

## Related

- `FUTURE-PROOFING-001`
- `FUTURE-PROOFING-003`
- `FUTURE-PROOFING-007`
- `DECISION-015`
- `wiki/harness-substrate.md`
- `wiki/post-jev-roadmap.md`

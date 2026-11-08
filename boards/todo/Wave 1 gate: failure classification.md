---
id: "DECISION-007"
title: "Wave 1 gate: failure classification"
priority: "P1"
type: "feature"
area: "DECISION"
spec: "wiki/decision-layer.md"
status: "todo"
created: "2026-10-24"
---

# Wave 1 gate: failure classification

## Summary

Classify a failed or stalled turn into a cause the runtime can act on:

- transport/stream failure,
- provider/model failure,
- tool-call or protocol failure,
- harness bug,
- environment/auth failure,
- user interruption.

Use the classification to choose retry, fallback, diagnostics, escalation, or a visible
error.

## Why this exists

The `EPIC-001` history shows recurring DeepSeek failures that were fixed one symptom at
a time, with manual triage each time. A cheap classifier at the failure boundary can
route to the right remedy and produce structured evidence instead of a vague stall.

## Why Wave 1

The failure boundary is stable and survives the V1 task-stage rewrite. It does not depend
on compaction or retrieval design.

## Scope

- Define the input: terminal state, error text, stream events, provider, model, tool
  call, elapsed time, budget.
- Define the classes and the action mapping.
- Define confidence and escalation: low confidence falls back to current behavior and is
  logged.
- Integrate at the turn terminal/error boundary.
- Record the classification and the chosen action in the decision log.
- Feed structured diagnostics into the session log.
- Evaluate against historical failure fixtures and live shadow runs.
- Authority: `advisory` first; `gated` only for retry/fallback choices that are already
  safe and reversible.

## Acceptance criteria

- Historical failure fixtures are classified with a reviewed accuracy baseline.
- A low-confidence decision does not change behavior.
- A high-confidence transport failure takes the transport remedy, not a generic retry.
- The classification is visible in diagnostics.
- No failure is hidden or converted into a success.
- Cost per decision is negligible relative to a model turn.

## Non-goals

- Fixing the underlying provider defects.
- Replacing `EPIC-001` or `QA-002`.
- Classifying user intent.
- Auto-retrying unboundedly.

## Related

- `DECISION-001`
- `DECISION-006`
- `DECISION-008`
- `EPIC-001` DeepSeek default-model reliability hardening
- `QA-002` deterministic DeepSeek stream/loop fixture harness
- `wiki/decision-layer.md`

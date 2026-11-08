---
id: "DECISION-005"
title: "Add shadow mode the decision log and replay"
priority: "P1"
type: "feature"
area: "DECISION"
spec: "wiki/decision-log-egress.md"
status: "todo"
created: "2026-10-24"
---

# Add shadow mode the decision log and replay

## Summary

Add shadow mode for decision classes, an append-only decision log with provenance, and a
replay path that reproduces decisions under the recorded policy version.

Shadow mode runs System 1 beside the current behavior without granting authority.

## Why this exists

Promotion without shadow evidence is guesswork. Replay without a log is impossible. The
log is also the training set for Laya and the audit trail for trust.

## Scope

- Add a per-class mode: off, shadow, promoted.
- In shadow: compute the decision, log it, compare with the current outcome, never act.
- Log schema per `wiki/decision-log-egress.md`:
  - class, state digest/summary, question, options,
  - result, confidence, engine, policy version, capability profile,
  - cost, latency, sensitivity,
  - downstream outcome and human correction.
- Append-only writes; corrections are new records.
- Add a replay command that re-runs a log against a candidate policy/engine.
- Add redaction and retention hooks.
- Add a report of agreement, disagreement, cost, and latency per class.
- Add tests for deterministic replay and for shadow decisions never affecting runtime.

## Acceptance criteria

- A shadow class produces log records and no behavior change.
- A promoted class acts and records the same fields.
- Replay reproduces the recorded decisions deterministically.
- The log cannot be rewritten by the runtime.
- Retention and deletion commands work.
- No secret is logged.
- Agreement/disagreement is reported per class.

## Non-goals

- Promoting any class.
- Building a UI for replay.
- Training a model.
- Logging raw transcripts or full prompts.

## Related

- `DECISION-001`
- `DECISION-003`
- `DECISION-004`
- `DECISION-006`
- `DECISION-014`
- `FUTURE-PROOFING-008`
- `FUTURE-PROOFING-009`
- `wiki/decision-log-egress.md`

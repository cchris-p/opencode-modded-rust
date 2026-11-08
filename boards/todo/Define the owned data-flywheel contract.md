---
id: "FUTURE-PROOFING-009"
title: "Define the owned data-flywheel contract"
priority: "P1"
type: "feature"
area: "FUTURE-PROOFING"
spec: "wiki/harness-substrate.md"
status: "todo"
created: "2026-10-24"
---

# Define the owned data-flywheel contract

## Summary

Define the provenance-bearing data contract that combines decision logs, task/session
outcomes, review verdicts, board transitions, transcript/classification labels, and
human corrections into one owned, replayable asset.

This is the advantage a generic harness cannot copy: private outcomes the labs do not
train on.

## Why this exists

The decision log alone is not a flywheel. A flywheel needs outcomes: did the task
succeed, did the review catch a real issue, did the operator correct the decision, did
the board item return to `doing`? Without outcomes, decisions are unlabelled.

## Scope

- Define a common record identity that links decision -> task -> session -> outcome.
- Define provenance fields: engine, model, policy version, capability profile, commit.
- Define outcome sources: verification, review, human correction, board transition,
  retry/reopen.
- Define a joinable local store or index that does not duplicate raw transcripts.
- Define access, retention, and egress boundaries by referencing
  `wiki/decision-log-egress.md`.
- Define the training-export query: allowed classes, splits, and lineage.
- Document how the contract supports evaluation, replay, and Laya fine-tuning.

## Acceptance criteria

- A decision can be joined to its downstream outcome.
- A task can be reconstructed from ids and summaries without raw private content.
- Human corrections are representable as first-class records.
- The contract works for Jev and Laya.
- The contract does not require a remote service.
- The retention/egress policy applies to every field.

## Non-goals

- Building a data warehouse.
- Exporting the dataset to a third party.
- Replacing the central session archive.
- Storing raw transcripts in the decision log.
- Training a model in this card.

## Related

- `FUTURE-PROOFING-001`
- `FUTURE-PROOFING-008`
- `DECISION-005`
- `DECISION-014`
- `wiki/harness-substrate.md`
- `wiki/decision-log-egress.md`

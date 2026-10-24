---
id: "SCOPE-008"
title: "Feed retrieval provenance and map outcomes into the data flywheel"
priority: "P2"
type: "feature"
area: "SCOPE"
spec: "wiki/workspace-intelligence.md"
status: "todo"
created: "2026-10-24"
---

# Feed retrieval provenance and map outcomes into the data flywheel

## Summary

Record retrieval provenance and map outcomes as part of the owned data contract: provider,
WIP version, capability, map schema, candidates, confidence, plan signals, fallback reason,
and downstream task/review outcome.

This turns the map from a stateless helper into part of the compounding data asset.

## Why this exists

The map makes many choices: which nodes, which slice, which confidence, which delta. Today
those choices are not joined to outcomes, so the product cannot learn which retrieval was
useful. The decision log covers decisions; retrieval needs the same treatment.

## Scope

- Define a retrieval record joined to the decision log:
  - session/task/subagent id,
  - provider, WIP version, map schema,
  - capability and representation,
  - candidates, confidence, provenance,
  - plan signals and lifecycle,
  - fallback reason,
  - token/cost/latency,
  - downstream outcome (task success, review, human correction).
- Apply `wiki/decision-log-egress.md` sensitivity tiers and local-first rules.
- Do not log raw private code by default; use digests/spans/symbols.
- Provide a query for evaluation and a governed training export.
- Add retention/deletion for retrieval records and dependent exports.
- Link retrieval records to the task/review outcome.
- Add tests for join, egress denial, retention, and deletion.

## Acceptance criteria

- A retrieval can be joined to its downstream outcome.
- The record includes provider/version/map schema and fallback.
- Sensitive/secret content is not logged or sent to a hosted service.
- A governed export exists for evaluation/fine-tuning.
- Retention and deletion apply to retrieval records and exports.
- The data is usable to answer "did the map help this task?"

## Non-goals

- A data warehouse.
- Logging full source files.
- Training a model in this card.
- Sharing retrieval data across machines by default.

## Related

- `FUTURE-PROOFING-009` owned data-flywheel contract
- `FUTURE-PROOFING-013`
- `FUTURE-PROOFING-017`
- `DECISION-005` decision log
- `DECISION-016`
- `wiki/decision-log-egress.md`
- `wiki/workspace-intelligence.md`
- `invariants/workspace-intelligence.md`

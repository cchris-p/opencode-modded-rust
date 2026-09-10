---
id: "PHASE-004"
title: "Evaluation harness"
priority: "P3"
type: "epic"
area: "PHASE"
spec: "wiki/agent-evaluation-strategy.md"
status: "todo"
created: "2026-09-10"
---

# Evaluation harness

## Summary

Stand up a lightweight, repeatable way to judge the Rust runtime across V1-V3: a defined task pack, a review-quality rubric with fixtures, persisted structured results, and an easy rerun workflow.

## Why this exists

The runtime is now capable enough to evaluate. Per `wiki/agent-evaluation-strategy.md`, the product needs an evidence-backed way to compare changes and track reliability across versions rather than relying on ad hoc manual sessions. The evaluation cards currently sit in `hold`; this phase groups them as the next planning phase.

## Scope

- `START-021` Define V1 evaluation task pack.
- `START-022` Define review quality rubric and fixtures.
- `START-023` Persist structured evaluation results.
- `START-024` Add lightweight evaluation rerun workflow.
- Reuse the existing regression net (`QA-001`: SSE integrity, multi-turn session-loop tests, live smoke) rather than inventing a parallel one.

## Non-goals

- Heavy benchmarking infrastructure.
- Broad model/provider leaderboards.

## Done when

- A defined task pack, rubric, persisted results, and a rerun workflow exist and are used to judge at least the V1 narrow workflow.

## Related Items

- `START-021` Define V1 evaluation task pack
- `START-022` Define review quality rubric and fixtures
- `START-023` Persist structured evaluation results
- `START-024` Add lightweight evaluation rerun workflow
- `QA-001` Build a repeatable debug/QA verification suite (completed basis)
- `PHASE-001` V1 daily-driver hardening (provides the workflow to evaluate)

## Notes

- Grouping/tracking parent; child cards currently live in `hold` and are not relaned by this card.
- `wiki/agent-evaluation-strategy.md` is the source for the V1-V3 evaluation intent.

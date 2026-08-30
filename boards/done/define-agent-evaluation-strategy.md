---
id: "START-006"
title: "Define agent evaluation strategy"
priority: "P2"
type: "docs"
area: "START"
spec: "wiki/agent-evaluation-strategy.md"
status: "done"
created: "2026-08-28"
---

# Define agent evaluation strategy

## Summary

Document how the Rust product will evaluate agent quality and workflow reliability without relying on TS-vs-Rust benchmark comparisons.

## Why this exists

The project still needs a way to judge whether the runtime is improving, even though formal benchmarking against the TypeScript reference is not a current priority.

## Must define

- what kinds of tasks are used for evaluation
- how task success is judged
- how review quality is judged
- how regressions are detected within the Rust product itself
- which signals matter more than raw speed

## Outcome

- Added `wiki/agent-evaluation-strategy.md` as the repo-local evaluation framework for V1 through V3.
- Anchored evaluation on bounded real-work task packs, explicit verification, and fresh-context review.
- Defined separate judgments for task success, review quality, regression detection, and secondary speed signals.
- Identified concrete follow-up work for task packs, review fixtures, structured result capture, and rerun workflow support.

## Follow-up items created

- `START-016` Define V1 evaluation task pack
- `START-017` Define review quality rubric and fixtures
- `START-018` Persist structured evaluation results
- `START-019` Add lightweight evaluation rerun workflow

## Done when

- A lightweight evaluation approach exists for V1 through V3.
- The approach reflects real personal-use workflows.
- Follow-up work can use the same evaluation language when comparing agent behavior changes.

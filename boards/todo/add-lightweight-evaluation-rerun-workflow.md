---
id: "START-024"
title: "Add lightweight evaluation rerun workflow"
priority: "P2"
type: "feature"
area: "START"
spec: "wiki/agent-evaluation-strategy.md"
status: "todo"
created: "2026-08-30"
---

# Add lightweight evaluation rerun workflow

## Summary

Create a repeatable operator workflow or helper that makes rerunning the evaluation task pack practical after meaningful runtime changes.

## Why this exists

The evaluation strategy only becomes useful if reruns happen often enough to catch regressions before they pile up.

## Scope

- Define the minimum repeated-run workflow.
- Keep the first version lightweight enough for a single-user local workflow.
- Reuse the task-pack, review-rubric, and structured-result work rather than inventing a parallel path.

## Done when

- A repeatable rerun workflow exists.
- The workflow makes before-or-after comparison practical.
- The approach fits V1 and V2 without requiring heavy automation.

## Related items

- `START-006` Define agent evaluation strategy
- `START-021` Define V1 evaluation task pack
- `START-023` Persist structured evaluation results

## Notes

- This can begin as an operator runbook before it grows into productized tooling.

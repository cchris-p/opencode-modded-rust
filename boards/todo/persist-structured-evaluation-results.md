---
id: "START-018"
title: "Persist structured evaluation results"
priority: "P2"
type: "feature"
area: "START"
spec: "wiki/agent-evaluation-strategy.md"
status: "todo"
created: "2026-08-30"
---

# Persist structured evaluation results

## Summary

Store evaluation outcomes in a structured repo-local format so runtime regressions can be compared over time.

## Why this exists

The evaluation strategy needs more than anecdotal notes if the project is going to detect stage-specific regressions and compare runs consistently.

## Scope

- Define the minimum evaluation result schema.
- Capture pass or fail, failure stage, verification outcome, review outcome, and short notes.
- Keep the initial storage model lightweight and local-first.

## Done when

- A concrete structured result format exists.
- Repeated runs can be compared without re-reading freeform notes.
- The design does not depend on a large telemetry system.

## Related items

- `START-006` Define agent evaluation strategy
- `START-016` Define V1 evaluation task pack
- `START-017` Define review quality rubric and fixtures

## Notes

- Start simple. A durable local artifact is more important than ambitious analytics.

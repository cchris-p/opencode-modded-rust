---
id: "START-021"
title: "Define V1 evaluation task pack"
priority: "P1"
type: "docs"
area: "START"
spec: "wiki/agent-evaluation-strategy.md"
status: "hold"
created: "2026-08-30"
---

# Define V1 evaluation task pack

## Summary

Create the first stable set of bounded V1 evaluation tasks so runtime changes can be measured against a repeatable real-work corpus.

## Why this exists

`START-006` defines the evaluation language, but the repo still needs concrete tasks to rerun when runtime behavior changes.

## Scope

- Define a small named V1 task pack grounded in real repository work.
- Give each task an objective, completion criteria, verification commands, and review focus.
- Keep the pack small enough to rerun regularly.

## Done when

- A repo-local evaluation task pack exists for V1.
- Each task is specific enough to run without reinterpretation.
- The pack covers at least bug-fix, focused feature, and docs-plus-code alignment work.

## Related items

- `START-006` Define agent evaluation strategy
- `START-005` Define V1 runtime loop

## Notes

- Prefer realistic tasks over synthetic benchmark prompts.
- Blocked until more real product usage exists across current functionality stories.
- Use all generated transcripts as raw evidence when this resumes.
- Include the collected OpenCode OG session corpus alongside this repo's transcripts so evaluator candidates come from real workflows instead of invented scenarios.
- Focus the future task pack on real functionality work, not synthetic evaluation-only exercises.

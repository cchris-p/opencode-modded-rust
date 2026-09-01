---
id: "START-022"
title: "Define review quality rubric and fixtures"
priority: "P1"
type: "docs"
area: "START"
spec: "wiki/agent-evaluation-strategy.md"
status: "hold"
created: "2026-08-30"
---

# Define review quality rubric and fixtures

## Summary

Turn fresh-context review quality into a repeatable judged surface instead of an informal impression.

## Why this exists

`START-006` makes review quality a first-class evaluation dimension, but the repo does not yet define how to test reviewer usefulness or false positives consistently.

## Scope

- Define a review rubric based on real issue-catching outcomes.
- Create a small set of review fixtures or scenarios with known good and bad changes.
- Distinguish blocking findings, non-blocking findings, misses, and noisy false alarms.

## Done when

- A repo-local review-quality rubric exists.
- Review fixtures or scenarios exist that can be rerun.
- The rubric is usable for both manual and future automated evaluation passes.

## Related items

- `START-006` Define agent evaluation strategy
- `START-005` Define V1 runtime loop

## Notes

- Judge review by outcome quality, not by verbosity or writing style.
- Blocked until the repo has a larger body of real functionality transcripts and collected OpenCode OG sessions to analyze.
- When resumed, derive fixtures from actual work traces and real review situations rather than fabricated examples.
- Use all available generated transcripts to identify which reviewer behaviors are worth scoring.

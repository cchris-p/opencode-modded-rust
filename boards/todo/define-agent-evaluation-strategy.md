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

## Done when

- A lightweight evaluation approach exists for V1 through V3.
- The approach reflects real personal-use workflows.
- Follow-up work can use the same evaluation language when comparing agent behavior changes.

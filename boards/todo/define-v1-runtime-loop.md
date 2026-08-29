---
id: "START-005"
title: "Define V1 runtime loop"
priority: "P1"
type: "feature"
area: "START"
spec: "wiki/v1.md"
status: "todo"
created: "2026-08-28"
---

# Define V1 runtime loop

## Summary

Specify the minimal V1 runtime loop that satisfies the core architecture for a serious personal daily-driver on a narrow workflow.

## Why this exists

V1 needs a precise execution model so implementation work does not drift into broad feature development or implicit parity work.

## Must define

- task stages
- task state model
- context construction inputs
- implementation stage boundaries
- review and verification boundaries
- completion criteria for a single bounded task

## Done when

- A concrete V1 runtime loop is documented.
- The loop is consistent with the current invariants.
- Implementation can be broken into smaller engineering tasks without ambiguity.

## Notes

- TUI support is part of V1, but this item is about the runtime loop rather than interface polish.
- The implementer must not be the sole authority on task success.

---
id: "START-025"
title: "Add retrieval-provider boundary for task context assembly"
priority: "P2"
type: "feature"
area: "START"
spec: "wiki/scopemux-integration-plan.md"
status: "todo"
created: "2026-08-30"
---

# Add retrieval-provider boundary for task context assembly

## Summary

Introduce a narrow runtime retrieval interface so task context assembly can stay generic in V1 while leaving a clean integration point for future `ScopeMux` support.

## Why this exists

`START-007` defines `ScopeMux` as a future retrieval-quality layer rather than a V1 dependency, but the current runtime still builds context directly inside session and prompt code.

## Scope

- Define the minimum retrieval request and response shapes the runtime needs.
- Keep the first provider generic and repository-local.
- Refactor only enough context assembly to consume that boundary cleanly.

## Done when

- The runtime has one explicit retrieval-provider boundary for task context assembly.
- V1 still works without `ScopeMux`.
- Future `ScopeMux` integration can target that boundary instead of scattering logic across the runtime.

## Related Items

- `START-005` Define V1 runtime loop
- `START-007` Plan ScopeMux integration
- `START-016` Define structured task state for V1

## Notes

- Keep this focused on the boundary, not on shipping graph retrieval.

---
id: "START-019"
title: "Define local-model-first provider path"
priority: "P1"
type: "research"
area: "START"
spec: "wiki/v1.md"
status: "todo"
created: "2026-08-29"
---

# Define local-model-first provider path

## Summary

Define what local-model-first means for the Rust product's V1 provider layer and identify the minimum supported execution path needed for serious daily-driver use.

## Why this exists

`START-004` found that the current provider abstraction is reusable, but the visible product shape still reads as multi-provider breadth rather than a clearly local-model-first V1 runtime.

## Must define

- the minimum local-model execution path V1 must support end-to-end
- which provider configuration and transport assumptions are acceptable for V1
- whether OpenAI-compatible local endpoints are sufficient or need explicit first-class treatment
- which current provider surfaces are in-scope, deferred, or reference-only for V1

## Done when

- The V1 local-model-first provider target is documented.
- The minimum implementation path is specific enough to break into engineering work.
- Deferred provider breadth is called out explicitly instead of remaining implied scope.

## Notes

- This is about product direction and minimum runtime expectations, not a broad provider parity effort.
- Coordinate with existing provider settings and catalog work instead of reopening them without need.

## Related Items

- `START-004` Assess current Rust state
- `START-009` Port provider setup from OpenCode
- `START-012` Refresh provider and model catalog
- `START-015` Mirror OpenAI auth configuration in settings

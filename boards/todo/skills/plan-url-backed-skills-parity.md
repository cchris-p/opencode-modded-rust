---
id: "SKILLS-002"
title: "Plan URL-backed skills parity"
priority: "P3"
type: "research"
area: "SKILLS"
spec: "wiki/skills-parity-audit.md"
status: "todo"
created: "2026-08-29"
---

# Plan URL-backed skills parity

## Summary

Decide whether and how the Rust product should support URL-backed skill sources that exist in the reference OpenCode line.

## Why this exists

The current parity audit shows that URL-backed skills are part of the reference behavior, but they are not used in the current narrow workflow and are not required for the first local transferable-skills parity pass.

## Scope

- Evaluate the reference `skills.urls` and related source-list behavior.
- Determine whether URL-backed skills are needed for this product's real workflow.
- If needed, define the minimum correct behavior, caching model, trust model, and verification approach.

## Done when

- A product decision exists for URL-backed skills: implement, defer long-term, or reject.
- If implementation is chosen, follow-up execution items are specific and implementation-ready.
- If deferred or rejected, the decision is recorded in repo-local docs.

## Notes

- This is intentionally separate from `SKILLS-001` so local filesystem skills parity can land first.

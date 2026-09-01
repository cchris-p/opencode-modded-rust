---
id: "SKILL-001"
title: "Add session-summary cascade skill by session name"
priority: "P3"
type: "feature"
area: "SKILL"
spec: ""
status: "todo"
created: "2026-08-31"
---

# Add session-summary cascade skill by session name

## Summary

Define and add a skill-oriented workflow where richer session information can cascade into simpler summaries grouped or selected by session name.

## Why this exists

Session history can carry more detail than a later task needs. A dedicated skill should make it easier to collapse that information into progressively simpler summaries without losing the session-name anchor that makes prior work recognizable and reusable.

## Scope

- Define the intended user flow for selecting or targeting sessions by session name.
- Define how full session information should cascade into shorter summary layers.
- Keep the first pass focused on skill behavior and prompt shaping, not on broad new session-storage architecture.

## Done when

- A concrete skill-level design exists for session-name-based summary cascading.
- The expected summary levels and handoff shape are explicit enough for implementation.
- Any required follow-up product work is split into implementation-ready board items.

## Related Items

- `SKILLS-001` Align skills with current and reference OpenCode behavior
- `SKILLS-002` Plan URL-backed skills parity
- `FEAT-001` Improve historical chat transcripts workflow

## Notes

- Treat this as a skill-first workflow idea unless investigation shows it needs core runtime changes.

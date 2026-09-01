---
id: "START-020"
title: "Constrain primary product surface to the V1 workflow"
priority: "P2"
type: "research"
area: "START"
spec: "wiki/v1.md"
status: "hold"
created: "2026-08-29"
---

# Constrain primary product surface to the V1 workflow

## Summary

Define which user-facing modes and capabilities are primary for V1, and which broad legacy or parity-oriented surfaces should be explicitly de-emphasized, hidden, or treated as reference-only.

## Why this exists

`START-004` found that the current repo exposes a much broader CLI and server surface than the narrow V1 workflow requires, which increases ambiguity about what the product is trying to ship first.

## Questions to answer

- Which entry points are primary for V1?
- Which current commands or surfaces should remain available but non-primary?
- Which surfaces should be treated as explicitly deferred rather than silently carried as V1 expectations?
- What documentation or UX changes are needed so the product reads as a focused daily-driver instead of a parity chase?

## Done when

- The primary V1 surface is documented.
- Out-of-scope or de-emphasized surfaces are called out explicitly.
- Follow-up implementation work exists for any UI, CLI, or docs changes needed to reinforce that boundary.

## Notes

- This is a product-boundary item, not a mandate to delete useful infrastructure.
- Retain compatibility surfaces only where they improve product utility or reduce risk.
- Blocked on completing the remaining current functionality-based stories first.
- Evaluation stories do not count as blockers for this item because they are intentionally parked in the `hold` lane pending more real usage evidence.

## Related Items

- `START-004` Assess current Rust state
- `START-008` Full parity deferred

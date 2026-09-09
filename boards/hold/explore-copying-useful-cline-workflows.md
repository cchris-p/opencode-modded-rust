---
id: "FEAT-005"
title: "Explore copying useful Cline workflows"
priority: "P3"
type: "research"
area: "FEAT"
spec: ""
status: "hold"
created: "2026-09-08"
---

# Explore copying useful Cline workflows

## Summary

Intentionally vague placeholder to look at Cline and copy useful workflow ideas where they fit this product, especially CLI-oriented workflows for managing background sessions.

## Why this exists

Cline may have product patterns worth stealing, adapting, or explicitly rejecting. The near-term example is a CLI workflow for background sessions, but this card should stay broad until the actual Cline behaviors are reviewed against this repo's daily-driver goals.

## Possible Areas

- CLI commands for listing, resuming, attaching to, or managing background sessions.
- Background-session status visibility outside the TUI.
- Lightweight handoff between editor/TUI/CLI surfaces.
- Any Cline feature that reduces friction in long-running coding sessions.

## Non-goals

- Treating Cline as a blanket parity target.
- Expanding V1 beyond the narrow daily-driver workflow without a follow-up decision.
- Implementing copied features directly from this card.

## Done when

- Specific Cline workflow candidates have been reviewed.
- Useful candidates are split into implementation-ready board items.
- Rejected candidates are briefly noted so they do not keep resurfacing as vague parity asks.

## Related Items

- `FEAT-002` Keep sessions running after TUI exit
- `FEAT-004` Add in-session send-to-fork commands
- `START-008` Full parity deferred
- `START-020` Constrain primary product surface to the V1 workflow

## Notes

- This card is intentionally not implementation-ready.
- Keep it as a holding bucket for Cline-inspired workflow review until concrete candidate features are identified.

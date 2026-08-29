---
id: "START-002"
title: "Freeze TS reference line"
priority: "P1"
type: "docs"
area: "START"
spec: ""
status: "done"
created: "2026-08-28"
---

# Freeze TS reference line

## Summary

Record and preserve the initial TypeScript reference line used by `scopemux-code` planning.

## Why this exists

The project intends to draw the line at the current OpenCode implementation for now, so the chosen reference point should be explicit.

## Current reference

- Repository: `$HOME/repos/opencode-modded`
- Commit: `e62912b5d18b73316c7bfd6e894b040698f6c880`

## Done when

- Product planning, feature triage, and later sync review work all reference this freeze explicitly unless a later board item changes it (README.md, AGENTS.md, wiki directory).

## Dev Notes

- Confirmed the frozen TypeScript reference line was already documented in `wiki/reference-strategy.md` and `wiki/product-boundary.md`.
- Added explicit freeze references to `README.md` and `AGENTS.md` so the top-level planning docs point to the same repo and commit.
- Verified the documentation update by reviewing the targeted files and the resulting git diff.

## Completion

- Completed on `development` with the frozen reference line explicitly called out in `README.md`, `AGENTS.md`, and the existing wiki documentation.

---
id: "START-003"
title: "Standardize product name"
priority: "P1"
type: "docs"
area: "START"
spec: ""
status: "done"
created: "2026-08-28"
---

# Standardize product name

## Summary

Adopt `scopemux-code` as the canonical product name across planning and product-facing documentation.

The logo removed in START-013 doesn't need to be considered for this story.

## Why this exists

The repository currently contains multiple names inherited from earlier work. That ambiguity makes planning and product identity less clear.

## Done when

- Planning docs consistently use `scopemux-code` as the product name.
- Any remaining alternative names are either removed or explicitly marked as legacy identifiers.

## Notes

- Repository path names and executable compatibility names may remain different when useful.

## Dev Notes

- Clarified in `README.md`, `USER_GUIDE.md`, `docs/README.md`, `wiki/product-boundary.md`, and `invariants/reference-boundary.md` that `scopemux-code` is the canonical product name.
- Marked retained `opencode`, `opencode-*`, and `opencode-modded-rust` identifiers as compatibility, implementation, or repository names rather than product branding.
- Verified the documentation changes by reviewing the targeted docs and checking the updated wording in git diff.

### PR Link

- https://github.com/cchris-p/opencode-modded-rust/pull/4

### Completion

- Fixed and merged into `development` on 2026-08-29.

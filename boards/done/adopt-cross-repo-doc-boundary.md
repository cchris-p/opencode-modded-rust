---
id: "START-001"
title: "Adopt cross-repo documentation boundary"
priority: "P1"
type: "docs"
area: "START"
spec: ""
status: "done"
created: "2026-08-28"
---

# Adopt cross-repo documentation boundary

## Summary

Record the cross-repo documentation ownership model as an explicit project decision for `scopemux-code`.

## Why this exists

The convention is documented in `AGENTS.md`, `wiki/`, and `invariants/`, but it should also exist as a tracked project decision.

## Decision

- `scopemux-code` owns its own `docs/`, `wiki/`, `invariants/`, and `boards/`.
- `$HOME/repos/opencode-modded` may be referenced by explicit path.
- Reference by path, not by inheritance.
- Rust-product policy is binding only when adopted in this repo.

## Done when

- The convention is treated as a standing project rule in future planning and execution work.

## Dev Notes

- Added `invariants/documentation-boundary.md` as the canonical repository-owned statement of the cross-repo documentation boundary.
- Indexed the new invariant in `invariants/README.md` so later planning work has a clear authoritative source.
- Linked `wiki/product-boundary.md` back to the invariant so the planning doc and invariant agree on the canonical rule location.

## Verification

- `git diff --check`
- Reviewed the updated invariant and boundary docs to confirm the documentation-ownership rule is explicit and repo-local.

## PR

- https://github.com/cchris-p/opencode-modded-rust/pull/5

## Completion

- Merged into `development` on 2026-08-29 after local QA approval.

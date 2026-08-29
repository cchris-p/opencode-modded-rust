---
id: "START-001"
title: "Adopt cross-repo documentation boundary"
priority: "P1"
type: "docs"
area: "START"
spec: ""
status: "todo"
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

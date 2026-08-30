---
id: "SKILLS-001"
title: "Align skills with current and reference OpenCode behavior"
priority: "P1"
type: "feature"
area: "SKILLS"
spec: "invariants/skills/README.md"
status: "done"
created: "2026-08-29"
---

# Align skills with current and reference OpenCode behavior

## Summary

Bring the Rust `skills` feature into explicit alignment with both the currently implemented local functionality in this repo and the original OpenCode skills behavior so the same skills are transferable and operate the same way.

## Why this exists

The repo now documents the current skills invariants, but the product surface is still uneven: local skill discovery and loading exist, while some user-facing integration points are incomplete.

The first skills task should lock down parity expectations before feature expansion so future work builds on a stable contract rather than drifting behavior.

## Scope

- Treat this repository as the product codebase of record.
- Use `$HOME/repos/opencode-modded` at the frozen reference line for behavioral comparison.
- Align skill behavior, discovery, transferability, and user-facing operation rather than inventing a new skills model.
- Use `wiki/skills-parity-audit.md` as the evidence-backed audit for this item.
- Limit this item to transferable local filesystem-backed skills and the user-facing surfaces that expose them.

## Must align

- skill file format expectations, including `SKILL.md` frontmatter requirements
- skill discovery roots and configured path behavior
- duplicate-name resolution and listing behavior
- runtime loading behavior and permission gating
- relative-file semantics for resources referenced from a skill directory
- user-facing listing surfaces so available skills are visible wherever the product claims they are
- compatibility of existing skills from the original OpenCode project with this Rust implementation

## Explicitly out of scope

- remote URL-backed skill discovery and sync behavior
- broader post-parity feature expansion beyond local transferable skills

## Deliverables

- A documented parity/alignment assessment for the skills feature.
- Implementation changes required to remove observed behavior gaps.
- Updated docs or invariants when the current written contract is incomplete or imprecise.
- Follow-up board items for intentionally deferred skills work beyond parity.

## Implementation Notes

- Local skill discovery now walks upward from the active directory to the git worktree root for `.opencode`, `.claude`, and `.agents` skill roots.
- Duplicate resolution remains later-source-wins, which gives nearer project roots precedence over ancestor roots.
- The server `/skill` route now returns discovered skill names plus descriptions and the TUI skills dialog consumes the same data.
- Prompt skill suggestions now refresh from the same server-backed discovered skill list.
- The `skill` tool now accepts the reference-compatible `name` key as an alias for `skill_name`.
- Repo-local invariants and the skills parity audit were added to capture the delivered local-filesystem skills contract.

## Verification

- `cargo test -p opencode-tool skill`
- `cargo test -p opencode-tui skill_list`
- `cargo check -p opencode-cli -p opencode-server -p opencode-tool -p opencode-tui`

## Related Items

- `SKILLS-002` tracks deferred URL-backed skills parity planning.

## PR Link

- https://github.com/cchris-p/opencode-modded-rust/pull/9

## Completion

- Fixed, PR'd, merged into `development`, and branch-cleaned on 2026-08-29.

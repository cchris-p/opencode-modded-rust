# Invariants Index

The `invariants/` directory contains absolute truths for the final desired `scopemux-code` system.

## What belongs here

- rules that should remain true across versions
- system constraints that shape implementation decisions
- repository-level policy that must be treated as binding in this repo

## What does not belong here

- temporary implementation notes
- brainstorming
- speculative ideas without commitment
- version-specific convenience decisions that may change freely

## Relationship to other project docs

- `wiki/` explains the roadmap and architecture direction.
- `docs/` explains the current implementation surface.
- `boards/` track the work required to move implementation toward the invariants.

## Current invariant set

- `documentation-boundary.md` defines the cross-repo documentation ownership rule.
- `reference-boundary.md` defines how the TypeScript/OpenCode reference line constrains adoption.
- `context.md` defines context-construction requirements.
- `task-state.md` defines authoritative task-state rules.
- `runtime-lifecycle.md` defines task lifecycle expectations.
- `retrieval.md` defines retrieval expectations.
- `verification.md` defines verification requirements.

## Cross-repo boundary

- Reference by path, not by inheritance.
- Files in `$HOME/repos/opencode-modded` may be used as context or historical reference.
- A rule is binding for `scopemux-code` only when documented in this repo.

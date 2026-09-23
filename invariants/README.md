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
- `wiki/cli-surface.md` is the canonical current/target behavior reference for the CLI task surface and TUI launch/detach/attach lifecycle.

## Current invariant set

- `documentation-boundary.md` defines the cross-repo documentation ownership rule.
- `reference-boundary.md` defines how the TypeScript/OpenCode reference line constrains adoption.
- `context.md` defines context-construction requirements.
- `task-state.md` defines authoritative task-state rules.
- `runtime-lifecycle.md` defines task lifecycle expectations.
- `session-durability.md` defines durable session/message ownership, additive persistence, explicit deletion, and resume integrity.
- `retrieval.md` defines retrieval expectations.
- `coding-session-behavior.md` defines agentic coding-session request requirements (agent identity, system prompt, environment context, tool attachment).
- `cli-task-targeting.md` defines CLI task target selection and queued cross-client send requirements.
- `message-queuing.md` defines per-session prompt ordering, queue-aware status, abort/cancel, and durable ordering intent.
- `coding-session-polling.md` defines low-context polling requirements for coding-session waits.
- `verification.md` defines verification requirements.
- `option-selection.md` defines the two sanctioned option-selection methods (mnemonic and focus) and when each may be used.
- Background session continuation is constrained by `runtime-lifecycle.md` and `task-state.md`.

## Cross-repo boundary

- Reference by path, not by inheritance.
- Files in `$HOME/repos/opencode-modded` may be used as context or historical reference.
- A rule is binding for `scopemux-code` only when documented in this repo.

---
id: "FEAT-049"
title: "Agent-role and @agent-name mention parity"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "todo"
predecessors: ""
created: "2026-09-21"
---

# Agent-role and @agent-name mention parity

## Summary

Child of `GATE-004` (parity gap 5). Align agent roles with the reference: `general` and `explore` are
subagents, primaries are filtered from the subagent catalog (and vice versa), and `@agent-name`
mentions in a prompt invoke the matching subagent.

## Parent

`GATE-004` subagent feature parity (`boards/todo/gate-subagent-feature-parity.md`), gap 5.

## Problem

- The Rust product models `general` as `AgentMode::Primary`
  (`crates/opencode-agent/src/agent.rs:239-253`) and it is currently disabled; the reference models it
  as a `Subagent`. See `wiki/agent-modes-and-custom-agents.md:100-131` and `FEAT-031`.
- The task tool's catalog does not use the registry and does not distinguish subagent roles
  (`crates/opencode-tool/src/task.rs:171-209`).
- There is no `@agent-name` mention parsing in prompts, so a user cannot invoke a specialized
  subagent the way the reference allows.
- Subagent/primary filtering exists for the picker (`GET /agent` filters `Subagent`), but the
  subagent catalog is a separate hardcoded list.

## Vanilla reference

Reference `f54ce313b99a`:

- `mode: "subagent" | "primary" | "all"` (`packages/opencode/src/agent/agent.ts:38`).
- `general` (`:182-195`) and `explore` (`:196-216`) are subagents; `build`/`plan` are primaries.
- Subagents are filtered out of the primary picker (`packages/tui/src/context/local.tsx:78`).
- `@agent-name` mentions invoke a specialized subagent
  (`packages/tui/src/feature-plugins/home/tips-view.tsx:202`).

## Scope / deliverables

- Re-introduce `general` as a `Subagent`-mode agent matching the reference role (general-purpose
  research/multi-step work), not as a selectable primary.
- Make the task tool's available-subagent set come from the `AgentRegistry` filtered by mode, so
  primaries are not offered as subagents unless their mode allows it.
- Parse `@agent-name` mentions in prompt text and route them to the matching subagent (invoke the
  task path with that agent), including validation and a clear error for unknown names.
- Keep custom-config agents able to declare `mode` and appear in the correct surface.
- Update `wiki/agent-modes-and-custom-agents.md` to reflect the resolved role model.

## Acceptance criteria

- `general` is registered and usable as a subagent, and does not appear as a primary mode.
- The `task` tool offers exactly the registry's subagent-capable agents (mode `subagent`/`all`).
- `@explore` / `@general` (or the configured names) in a prompt invokes that subagent; unknown
  mentions produce a clear error and do not silently fall through.
- Primary-only agents cannot be invoked as subagents, matching the reference.
- `cargo test -p opencode-agent -p opencode-tool` passes with role/mention tests.

## Verification

- Unit tests: registry role filtering; mention parsing and routing; unknown-mention error.
- Manual: type `@explore` in a prompt and confirm the subagent runs and the child session is created.
- Side-by-side role lists against the reference `agent.ts`.

## Related Items

- `GATE-004` subagent feature parity.
- `FEAT-031` investigate/disable `general` - done; this card resolves the parity follow-up.
- `FEAT-046` task tool contract parity - registry-driven lookup is shared.
- `wiki/agent-modes-and-custom-agents.md` - must be updated by this card.

---
id: "FEAT-049"
title: "Agent-role and @agent-name mention parity"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "qa"
predecessors: ""
created: "2026-09-21"
updated: "2026-09-23"
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
  task path with that agent), including validation and a clear error for a registered agent that is
  not subagent-capable.
- Keep custom-config agents able to declare `mode` and appear in the correct surface.
- Update `wiki/agent-modes-and-custom-agents.md` to reflect the resolved role model.

## Acceptance criteria

- `general` is registered and usable as a subagent, and does not appear as a primary mode.
- The `task` tool offers exactly the registry's subagent-capable agents (mode `subagent`/`all`).
- `@explore` / `@general` (or the configured names) in a prompt invokes that subagent. A mention of
  a registered non-subagent agent (e.g. `@build`) fails with a clear error and does not silently fall
  through; a token that is neither a file nor a registered agent stays plain prompt text.
- Primary-only agents cannot be invoked as subagents, matching the reference.
- `cargo test -p opencode-agent -p opencode-tool` passes with role/mention tests.

## Verification

- Unit tests: registry role filtering; mention parsing and routing; non-subagent-mention error;
  unknown-token passthrough.
- Manual: type `@explore` in a prompt and confirm the subagent runs and the child session is created.
- Side-by-side role lists against the reference `agent.ts`.

## Dev Notes

- **`general` is now a subagent.** `BuiltinAgent::General` is back in `BuiltinAgent::all()`
  (`crates/opencode-agent/src/agent.rs`). `AgentInfo::general()` sets `mode: Subagent`, the reference
  description ("General-purpose agent for researching complex questions and executing multi-step
  tasks…"), and no dedicated prompt (model default is used, matching the reference). It is never the
  default (`AgentRegistry::default_agent()` still returns `build`) and never appears in the primary
  picker (`GET /agent` filters `Subagent`).
- **`general` permission.** `build_agent_ruleset("general", &[])`
  (`crates/opencode-permission/src/ruleset.rs`) now merges `default_ruleset()` with an explicit
  `todowrite` deny, matching the reference `todowrite: "deny"`. The existing registry-driven task
  path (FEAT-046) already rejects primaries and uses the subagent's own ruleset, so no task-tool
  change was needed.
- **`@agent-name` mention routing.** `resolve_prompt_parts`
  (`crates/opencode-session/src/prompt.rs`) resolves a token file-first, then falls back to a
  registered subagent, producing a `PartInput::Agent` that injects the reference-style synthetic
  "call the task tool with subagent: X" instruction. The server wires this in `run_prompt_turn`
  (`crates/opencode-server/src/routes.rs`), passing `agent_mention_names` (subagent-capable vs other
  registered agents).
- **Validation / error.** A mention of a registered non-subagent agent (e.g. `@build`) returns
  `PromptError::InvalidAgentMention` and the turn ends with a recorded assistant error instead of
  silently falling through. A token that is neither a file nor a registered agent stays plain text
  (matching the reference file-first fallback; avoids false positives on `@` in prose).
- **Deviation from the reference, documented:** the reference does not restrict mentions to
  subagent-mode agents at the task tool. This product keeps the stricter "primaries are not
  subagents" rule already established by FEAT-046 and surfaces a clear error for primary mentions.
- **Verification:** `cargo fmt --all`; `cargo check --workspace`; `cargo test -p opencode-agent
  -p opencode-permission -p opencode-tool -p opencode-server --lib`; `cargo test -p opencode-session
  --lib resolve_prompt_parts`. Added tests: general subagent role/registry, `general` ruleset
  `todowrite` deny, mention parsing/routing/dedup, primary-mention error, unknown-mention passthrough,
  and server-side `resolve_task_subagent`/`agent_mention_names` coverage.
- **Known limitation / follow-up:** TUI `@` autocomplete still sources agent suggestions from the
  primary-only `GET /agent` list, so `@general`/`@explore` must currently be typed rather than
  selected. Recorded in `wiki/agent-modes-and-custom-agents.md` open questions.

### PR Link

- PR #98 (https://github.com/cchris-p/opencode-modded-rust/pull/98) — `feature/FEAT-049-agent-role-mention-parity` → `development`.
- Program: GATE-004 (H-009), PR 4/7. Awaiting human test/merge on the checked-out branch; PR stays open in `qa`.

## Related Items

- `GATE-004` subagent feature parity.
- `FEAT-031` investigate/disable `general` - done; this card resolves the parity follow-up.
- `FEAT-046` task tool contract parity - registry-driven lookup is shared.
- `wiki/agent-modes-and-custom-agents.md` - must be updated by this card.

### 2026-09-23 - Merged into `development`

- Merged via PR #98 (merge commit `24207e0edaff90b1ade5d9c5f88e459653940245`) on explicit user
  approval.
- Branch cleanup complete: remote and local `feature/FEAT-049-agent-role-mention-parity` deleted.
- Card remains in `qa` pending a recorded QA report (post-merge `@explore`/`@general` smoke on
  `development`). The merge alone does not move it to `done`.

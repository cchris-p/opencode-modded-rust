---
id: "FEAT-046"
title: "Task tool contract parity"
priority: "P1"
type: "feature"
area: "FEAT"
spec: ""
status: "done"
predecessors: ""
created: "2026-09-21"
---

# Task tool contract parity

## Summary

Child of `GATE-004` (parity gap 2). Align the Rust `task` tool with the reference contract: registry
driven agent lookup, depth limit, subagent permission derivation, model/agent resolution, error
behavior, and the `<task ...>` output shape.

## Parent

`GATE-004` subagent feature parity (`boards/todo/gate-subagent-feature-parity.md`), gap 2.

## Problem

- `crates/opencode-tool/src/task.rs:171-209` uses a static `get_available_agents()` catalog
  (`explore`, `plan`, `title`, `summary`, `compaction`, `build`) rather than the `AgentRegistry`
  (`crates/opencode-agent/src/agent.rs`). An unknown `subagent_type` still creates a subsession.
- `run_in_background` is part of the input schema (`task.rs:28-29`) but is never read.
- No `subagent_depth` limit exists anywhere.
- Disabled tools are derived from the static catalog (`task.rs:211-223`) with no permission
  derivation from the parent session.
- Output is `"task_id: {id} (for resuming ...)\n\n<task_result>{text}</task_result>"`
  (`task.rs:135-139`), not the reference `<task id state>` wrapper.
- The model fallback uses `do_get_last_model` rather than the resolved subagent/agent model.

## Vanilla reference

Reference `f54ce313b99a`:

- Parameters: `description`, `prompt`, `subagent_type`, optional `task_id`, optional `command`,
  `background` (`packages/opencode/src/tool/task.ts:43-62`).
- Depth limit via walking `parentID`, `cfg.subagent_depth ?? 1` (`:104-117`).
- Permission ask for `task` with `subagent_type` pattern unless `bypassAgentCheck` (`:119-129`).
- Registry lookup; unknown agent errors (`:131-134`).
- Agent model wins over the parent message model (`:181-184`).
- Subagent session permission derivation
  (`packages/opencode/src/agent/subagent-permissions.ts:14-26`).
- Output wrapper `<task id="..." state="running|completed|error">` with optional `<summary>` and
  `<task_result>` (`task.ts:64-79`); `task_error` on failure (`:70`).

## Scope / deliverables

- Look up the subagent through the `AgentRegistry`; error on an unknown or non-subagent type where the
  reference errors.
- Add `subagent_depth` handling (config key + walk of `parent_id`) with the reference default.
- Derive the child session permission ruleset from the parent session plus default `todowrite`/`task`
  denies.
- Resolve the subagent model/agent from the registry, falling back to the parent model.
- Convert the tool output to the reference `<task ...>` shape, including running/completed/error
  states and `task_error`.
- Keep `run_in_background` wired to `FEAT-048`; if the background feature is absent, the flag must
  error the same way the reference does when the experimental gate is off.
- Update `parameters()`/JSON schema to match the reference field set.

## Acceptance criteria

- `task` with a valid subagent uses the registry and the subagent's model when set.
- `task` with a depth beyond `subagent_depth` fails with the reference-style message.
- Unknown `subagent_type` fails and does not create a session.
- Subagent sessions receive parent-derived permissions with `todowrite`/`task` denied unless the
  subagent permits them.
- Tool output matches the reference `<task ...>` wrapper for completed and error states.
- `cargo test -p opencode-tool -p opencode-agent` passes with updated task tests.

## Verification

- Unit tests for registry lookup, unknown agent, depth limit, permission derivation, and output
  formatting.
- Side-by-side output comparison against `packages/opencode/src/tool/task.ts` fixtures.
- `cargo check -p opencode-tool -p opencode-agent`.

## Related Items

- `GATE-004` subagent feature parity.
- `FEAT-045` child session persistence - supplies the real session the tool writes to.
- `FEAT-048` background subagents - owns `background`/`run_in_background`.
- `CLI-002` Route `opencode run` through the canonical session runtime - overlapping dispatch path (gated by `CLI-001`/`CLI-006`).

## Dev Notes

Program: `GATE-004` (H-009), PR 2/7. Branch `feature/FEAT-046-task-tool-contract-parity`.
PR: https://github.com/cchris-p/opencode-modded-rust/pull/96

### What changed

- **Registry-driven subagent resolution.** New `ToolContext::resolve_subagent` capability
  (`crates/opencode-tool/src/tool.rs`), wired server-side (`resolve_task_subagent`,
  `crates/opencode-server/src/routes.rs`) and executor-side (`with_subsession_callbacks`,
  `crates/opencode-agent/src/executor.rs`). `AgentRegistry::resolve_subagent` filters to non-hidden
  `subagent`/`all` agents; unknown or non-subagent names fail with
  `Unknown agent type: <x> is not a valid agent type` and no child session is created.
- **`subagent_depth` config key** (top-level, snake_case, default `1`) added to
  `crates/opencode-config/src/schema.rs`; the depth is computed by walking `parent_id` in both the
  resolver and the server child-creation guard, with the reference error text.
- **Parent-derived child permissions.** New `opencode_agent::derive_subagent_session_permission`
  (Rust equivalent of `packages/opencode/src/agent/subagent-permissions.ts`): parent `deny` +
  `external_directory` rules, plus default `todowrite`/`task` denies unless the subagent's own
  ruleset already permits them. Child sessions record the derived ruleset in
  `metadata.subagent_permission`, add the denies to the session permission overlay, and filter the
  child tool set through it.
- **Output + metadata.** `<task id="..." state="...">` wrapper with optional `<summary>` and
  `<task_result>`/`<task_error>`; metadata now carries `parentSessionId`, `sessionId`, and `model`.
  The subagent's configured model wins over the parent model.
- **Parameters** match the reference field set (`description`, `prompt`, `subagent_type`, optional
  `task_id`, optional `command`, `background`); `load_skills`/`run_in_background` removed.
- **Background gate.** `background: true` requires
  `OPENCODE_EXPERIMENTAL_BACKGROUND_SUBAGENTS=true`, otherwise the reference error is returned.
  Background execution itself stays owned by `FEAT-048` (gate-on currently returns a clear
  not-implemented error).

### Decisions / deviations

- Primaries (`build`, `plan`) are rejected as `subagent_type` per Shared Decision 4 (mode filter).
  Registering `general` as a subagent and `@agent-name` routing remain `FEAT-049`.
- Foreground failures surface as a tool error, matching the reference; the
  `<task state="error">`/`<task_error>` wrapper is implemented and unit-tested for the background
  path.

### Verification

- `cargo fmt --all`
- `cargo check --workspace`
- `cargo test -p opencode-tool -p opencode-agent -p opencode-config -p opencode-server` (green)

## Merge Closeout - 2026-09-23

- Merged into `development` via PR #96 (merge commit `05752d9109ba3c12babd1963118618269aac4919`).
- Feature branch `feature/FEAT-046-task-tool-contract-parity` deleted remotely and locally; local
  checkout is back on `development` and fast-forwarded to the merge.
- Post-merge QA on `development` passed: `cargo fmt --all`, `cargo check --workspace`, and
  `cargo test -p opencode-tool -p opencode-agent -p opencode-config -p opencode-server` (green).
- Promoted `qa -> done` on explicit user direction after the human test/merge gate.
- Program `GATE-004` (H-009) PR 2/7 complete; PR 3 (`FEAT-047`) is next.
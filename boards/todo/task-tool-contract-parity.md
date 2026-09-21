---
id: "FEAT-046"
title: "Task tool contract parity"
priority: "P1"
type: "feature"
area: "FEAT"
spec: ""
status: "todo"
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
- `FEAT-012` CLI/`AgentExecutor` tool-loop parity - overlapping dispatch path (blocked by `GATE-002`).

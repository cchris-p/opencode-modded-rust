---
id: "FEAT-045"
title: "Subagent child sessions are real, persisted, and parent-linked"
priority: "P1"
type: "feature"
area: "FEAT"
spec: ""
status: "doing"
predecessors: ""
created: "2026-09-21"
---

# Subagent child sessions are real, persisted, and parent-linked

## Summary

Child of `GATE-004` (parity gap 1). Make the `task` tool create a real child session that is linked
by `parent_id`, persisted in SQLite, listed as a child, and resumable by `task_id`, instead of the
current in-memory `SubsessionState`/`PersistedSubsession` maps.

## Parent

`GATE-004` subagent feature parity (`boards/todo/gate-subagent-feature-parity.md`), gap 1.

## Problem

- `crates/opencode-tool/src/task.rs:117-132` creates a subsession through the
  `do_create_subsession`/`do_prompt_subsession` callbacks.
- Those callbacks are implemented with in-memory maps: `SubsessionState`
  (`crates/opencode-agent/src/executor.rs:533-607`) and `PersistedSubsession`
  (`crates/opencode-session/src/prompt.rs:1750-1805`).
- The resulting ids are synthetic `task_<agent>_<uuid>` and have no `parent_id`, no persisted
  messages, and no restart resume.
- Real child-session support already exists but is unused by the task tool: `Session::child`
  (`crates/opencode-session/src/session.rs:399-431`), `create_child`
  (`crates/opencode-server/src/routes.rs:494-501`), `sessions.parent_id`
  (`crates/opencode-storage/src/schema.rs:11`), `list_children`
  (`crates/opencode-storage/src/repository.rs:406`).

## Vanilla reference

Reference `f54ce313b99a`:

- `packages/opencode/src/tool/task.ts:156-172` creates the child via
  `sessions.create({ parentID, title: params.description + " (@<agent> subagent)", agent, permission })`.
- `task.ts:136-138` resolves `task_id` to an existing session for resume.
- Child sessions are persisted sessions and appear in `sync.data.session` with a `parentID`
  (`packages/tui/src/routes/session/index.tsx:202-205`).

## Scope / deliverables

- Route `task` execution through real session creation (server-side), setting `parent_id` to the
  calling session and using the vanilla title format.
- Persist the child session and its messages so they survive restart and appear in `list_children`.
- Make `task_id` resolve to an existing child session and prompt it, rather than a map lookup; a
  missing/invalid `task_id` must error clearly, not silently create a new session.
- Remove or demote the in-memory subsession maps once the real path is live; keep a thin fallback
  only if a code path genuinely needs it, documented.
- Ensure the child session's agent and model are recorded on the session (feeds `FEAT-046`/`FEAT-047`).

## Acceptance criteria

- A `task` call produces exactly one child session row with the correct `parent_id` and title.
- The child session and its messages persist across a server restart.
- `list_children` returns the subagent session; a resumed `task_id` continues the same session and
  does not create a second one.
- No synthetic `task_*` session id is exposed to clients as a session identity.
- Existing task-tool unit tests that assert in-memory behavior are updated to the child-session
  contract.

## Verification

- `cargo test -p opencode-tool -p opencode-session -p opencode-server`
- New integration test: spawn `task`, assert one child session with `parent_id`, restart the store,
  assert the child and its messages still exist and resume works.
- Manual: `ort-build`, `ort`, spawn a subagent, confirm the child session is listed and reopens after
  restart.

## Related Items

- `GATE-004` subagent feature parity.
- `FEAT-046` task tool contract parity - shares the task-tool wiring.
- `FEAT-047` TUI subagent navigation - needs real child sessions to navigate to.

## Implementation Notes

### 2026-09-23 - PR 1 of `H-009`

- Branch: `feature/FEAT-045-subagent-child-sessions` (PR link added when opened).
- `SessionPrompt` now accepts optional server-owned `create_subsession`/`prompt_subsession`
  callbacks (`crates/opencode-session/src/prompt.rs`); `execute_tool_calls` only installs the
  in-memory `task_*` fallback when neither is provided, so the server's real path is no longer
  shadowed.
- The server (`crates/opencode-server/src/routes.rs`) now supplies those callbacks from
  `run_prompt_turn`:
  - `create_child_subagent_session` creates a real child via
    `SessionManager::create_child`, sets `parent_id`, the reference title
    `"<description> (@<agent> subagent)"`, records `agent`/`model`/`subagent_disabled_tools`
    metadata, and persists. It enforces the reference default `subagent_depth = 1` as an interim
    guard (config key lands in `FEAT-046`).
  - `prompt_child_subagent` resolves the child agent/model, runs the canonical
    `SessionPrompt` loop on the child, writes it back, and persists, returning the child's final
    assistant text for the `task` tool's output wrapper.
- Unknown/invalid `task_id` now errors (`Unknown subagent session: ...`) without creating a
  session; a missing parent errors.
- In-memory subsession maps remain only as a fallback for callers that do not install server
  callbacks (direct `AgentExecutor` use and unit tests), documented in `prompt.rs`.
- Fixture slice added in `crates/opencode-server/src/routes.rs`:
  `task_child_is_real_parent_linked_and_persisted`, `nested_task_beyond_depth_is_rejected`,
  `unknown_task_id_errors_instead_of_creating_a_session`.
- Verification: `cargo fmt --all`; `cargo check --workspace`; `cargo test -p opencode-tool
  -p opencode-session`; `cargo test -p opencode-server` (46 + 3 integration tests pass).
- Known follow-ups owned by later cards: registry-driven lookup, configurable `subagent_depth`,
  parent-derived subagent permissions, and the `<task ...>` output wrapper (`FEAT-046`); TUI child
  navigation (`FEAT-047`); manual restart/resume QA.
- Note: the two existing `opencode-tool` task unit tests exercise the tool contract with mock
  callbacks and still pass; the child-session contract is covered by the new server fixtures.


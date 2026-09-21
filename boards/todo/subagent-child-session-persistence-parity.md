---
id: "FEAT-045"
title: "Subagent child sessions are real, persisted, and parent-linked"
priority: "P1"
type: "feature"
area: "FEAT"
spec: ""
status: "todo"
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

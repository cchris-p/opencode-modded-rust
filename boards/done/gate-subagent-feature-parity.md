---
id: "GATE-004"
title: "Gate: subagent features must match vanilla OpenCode exactly"
priority: "P1"
type: "gate"
area: "GATE"
spec: ""
status: "done"
predecessors: ""
created: "2026-09-21"
---

# Gate: subagent features must match vanilla OpenCode exactly

## Summary

Hard gate. Bring the Rust product's subagent feature set to parity with the reference OpenCode
implementation: first-class subagent agents, the `task` tool, real parent/child sessions, TUI child
navigation (including `ctrl+x` then `down` = "view subagents"), background subagents, and the
observable surfaces that make subagents inspectable. No subagent-dependent story may be treated as
complete until this gate passes.

This card holds the reference behavior to target, the current Rust gaps, the parity decisions, and
the acceptance criteria. Implementation is split across the child items listed under "Parity gaps
split into child items".

## Gate requirement

> Subagent features must work the same way they work in vanilla OpenCode: the `task` tool creates a
> real child session, the TUI can navigate into and between subagent sessions, and background
> subagents behave and are surfaced the same way.

Parity is judged against the reference source (below), not a paraphrase. Where the Rust runner has no
equivalent mechanism, the observable behavior and display must still match; internals that cannot be
reused are reference-only and must be called out explicitly.

## Reference source

- The reference is `$HOME/repos/opencode-modded` (`https://github.com/cchris-p/opencode-modded`, a
  mirror of `anomalyco/opencode`). Per `AGENTS.md`, vanilla behavior is referenced from the `dev`
  branch and the checkout is fetched to the latest `dev` before comparing.
- This gate pins the reference at `f54ce313b99a6661d7758ad042f7a6e05c8e0972` (`dev`, package
  `1.18.31`, 2026-09-21) and explicitly re-pins the previously recorded line.
- The old frozen commit `e62912b5d18b73316c7bfd6e894b040698f6c880` is unreachable from the remote
  (force-pushed; GitHub API 422, not an ancestor of `dev`) and must not be cited as current.
- Drift check: every subagent-relevant reference file is byte-identical between the first-fetched
  `dev` tip `e207624c4` (`1.18.29`, 2026-09-06) and the pinned `f54ce313b99a`. The 86 intervening
  commits touch no subagent, agent-mode, task-tool, TUI child-navigation, or CLI subagent file, so
  the line references below remain valid.

## Vanilla behavior to target (reference `f54ce313b99a`)

### 1. Agent roles

- Agent `mode` is `"subagent" | "primary" | "all"` (`packages/opencode/src/agent/agent.ts:38`).
- `build` (`:141-155`) and `plan` (`:156-180`) are primaries.
- `general` (`:182-195`) and `explore` (`:196-216`) are **subagents**.
- Subagents are filtered out of the primary picker (`packages/tui/src/context/local.tsx:78`).
- `@agent-name` mentions in a prompt invoke a specialized subagent (home tip
  `packages/tui/src/feature-plugins/home/tips-view.tsx:202`).

### 2. The `task` tool

- Parameters: `description`, `prompt`, `subagent_type`, optional `task_id`, optional `command`, and
  `background` (`packages/opencode/src/tool/task.ts:43-62`).
- Background requires `experimentalBackgroundSubagents` and fails otherwise (`:97-102`).
- Depth limit `cfg.subagent_depth ?? 1`, computed by walking `parentID` (`:104-117`).
- Permission ask for `task` with the `subagent_type` pattern unless `bypassAgentCheck` (`:119-129`).
- Agent lookup via the agent registry; unknown type errors (`:131-134`).
- `task_id` resumes the existing child session (`:136-138`).
- A **real child session** is created via `sessions.create({ parentID, title, agent, permission })`
  with title `"<description> (@<agent> subagent)"` (`:156-172`).
- Child permissions derive from the parent session plus default `todowrite`/`task` denies
  (`packages/opencode/src/agent/subagent-permissions.ts:14-26`).
- Output is wrapped as `<task id="..." state="completed|running|error">` with an optional
  `<summary>` and a `<task_result>` block (`task.ts:64-79`).
- Background runs are promoted async and inject a synthetic completion/error message back into the
  parent session (`:227-319`).

### 3. Parent/child session model

- Child sessions are linked by `parentID` and persisted as sessions.
- TUI `children()` includes the current session (when root) plus direct children, sorted
  (`packages/tui/src/routes/session/index.tsx:202-205`).

### 4. TUI navigation ("ctrl+x down view subagents")

- Leader default is `ctrl+x` (`packages/tui/src/config/keybind.ts:39,46`).
- `session_child_first` = `<leader>down` "Go to first child session" (`:103`).
- `session_parent` = up (`:106`); `session_child_cycle` = right (`:104`);
  `session_child_cycle_reverse` = left (`:105`).
- `moveFirstChild` enters the first session whose `parentID` is set; `moveChild` cycles siblings
  (`index.tsx:436-470`).
- Child navigation commands are enabled only when the current session has a `parentID`
  (`:1050-1085`).
- Assistant messages that contain a `task` tool part render `"<shortcut> view subagents"`
  (`:1489,1512-1513`), plus `ctrl+b background` for running foreground tasks when the background
  capability is on.
- `SubagentFooter` shows the agent label (from `@<agent> subagent`), "(index of total)" siblings,
  context/cost usage, and Parent/Prev/Next affordances (`packages/tui/src/routes/session/subagent-footer.tsx`).
- `session_background` = `ctrl+b` backgrounds synchronous subagents
  (`packages/tui/src/config/keybind.ts:98`).

### 5. CLI

- The CLI run footer has subagent tabs/details
  (`packages/opencode/src/cli/cmd/run/footer.subagent.tsx`, `subagent-data.ts`).

## Current Rust implementation and gaps

- **Subsessions are in-memory, not sessions.** `crates/opencode-tool/src/task.rs:117-132` routes
  through `do_create_subsession`/`do_prompt_subsession`, implemented as in-memory
  `SubsessionState`/`PersistedSubsession` maps keyed by synthetic `task_<agent>_<uuid>` ids
  (`crates/opencode-agent/src/executor.rs:533-607`,
  `crates/opencode-session/src/prompt.rs:1750-1805`). They have no `parent_id`, no DB row, no
  persisted messages, and no restart resume.
- **Child-session machinery exists but is unused by the task tool.** `Session::child`
  (`crates/opencode-session/src/session.rs:399-431`), `create_child` (`crates/opencode-server/src/routes.rs:494-501`),
  `sessions.parent_id` (`crates/opencode-storage/src/schema.rs:11`), and `list_children`
  (`crates/opencode-storage/src/repository.rs:406`) exist, but the `task` tool never calls them.
- **Task tool contract diverges.** Static `get_available_agents()` catalog instead of the
  `AgentRegistry` (`crates/opencode-tool/src/task.rs:171-209`); `run_in_background` accepted but
  unused (`:28-29`); no `subagent_depth` limit; no subagent permission derivation; output format is
  `task_id: ...\n\n<task_result>...` rather than vanilla's `<task ...>` wrapper; unknown
  `subagent_type` still creates a subsession.
- **`general` is a primary (and currently disabled), not a subagent.** See
  `wiki/agent-modes-and-custom-agents.md:100-131` and `FEAT-031` (done).
- **No `@agent-name` mention routing.**
- **TUI navigation is dead code.** `session_parent`/`session_child_cycle`/`session_child_cycle_reverse`
  are registered (`crates/opencode-tui/src/context/keybind.rs:179-184`) but the leader handler maps
  only `n l m a t b s q u r` and never handles `KeyCode::Down` or child/parent navigation
  (`crates/opencode-tui/src/app/app.rs:428-437`). There is no "view subagents" hint
  (`crates/opencode-tui/src/components/session_tool.rs:34` renders `task` as `"#"`), no subagent
  footer, and `SubagentDialog` is instantiated (`app.rs:89,231`) but `.open()` is never called.
- **No background subagents / notification injection / `ctrl+b`** for running foreground tasks.
- **CLI hides child sessions** (`crates/opencode-cli/src/main.rs:1095,1330,3525,3545` filter
  `parent_id.is_none()`) and has no subagent surface.
- **No subagent parity fixtures** beyond the two task-tool unit tests styled around the in-memory
  path (`crates/opencode-tool/src/task.rs:247-378`).

## Parity gaps split into child items

1. `FEAT-045` Subagent child sessions are real, persisted, and parent-linked
   (`boards/todo/subagent-child-session-persistence-parity.md`).
2. `FEAT-046` Task tool contract parity
   (`boards/todo/task-tool-contract-parity.md`).
3. `FEAT-047` TUI subagent navigation and "view subagents"
   (`boards/todo/tui-subagent-navigation-parity.md`).
4. `FEAT-048` Background subagents and notification injection
   (`boards/todo/background-subagents-parity.md`).
5. `FEAT-049` Agent-role and `@agent-name` mention parity
   (`boards/todo/agent-role-and-mention-parity.md`).
6. `CLI-010` CLI subagent surface parity
   (`boards/todo/cli-subagent-surface-parity.md`).
7. `FEAT-051` Subagent parity verification fixtures
   (`boards/todo/subagent-parity-verification-fixtures.md`).

## Scope

- Agent roles: `general`/`explore` as subagents, primary/subagent filtering, `@agent-name` routing.
- `task` tool contract: parameters, registry-driven lookup, depth limit, permissions, output shape.
- Real child sessions: creation with `parent_id` and vanilla title, persistence, resume, listing.
- TUI child navigation and subagent surfaces, including the `ctrl+x down` binding.
- Background subagents and parent notification injection, gated as in the reference.
- CLI child/subagent visibility where a run surface exists.
- Focused verification fixtures per child plus side-by-side parity evidence for the gate.

## Non-goals

- General upstream sync or broad OpenCode parity beyond subagents.
- Rebuilding the web/app subagent timeline UI; the Rust product has no browser app.
- Parallel general-purpose subagent orchestration beyond what the task tool already provides.
- Reworking the whole permission system beyond the `task`/`todowrite` subagent rules.
- Persisted background jobs across server restart unless a child decides it is needed.

## Parity decisions (resolved)

- **Child sessions are the target model.** The Rust in-memory subsession map is a divergence, not a
  deliberate deviation; child sessions must be real, persisted, and parent-linked so the TUI and CLI
  can navigate them.
- **`general` should be a subagent, not a primary.** Re-enabling it as a `Subagent`-mode agent with
  the reference role is the parity choice; the current "disabled" state is a documented interim.
- **Background subagents are experimental in the reference**, so the gate may ship them behind the
  same experimental gate, but the observable behavior (async return + completion injection + `ctrl+b`)
  must match when enabled.
- **Verification fixtures are a first-class child item**, not an afterthought.

## Acceptance criteria

- The `task` tool creates a real child session with `parent_id` and title `"<description> (@<agent>
  subagent)"`, and `task_id` resumes that same session.
- Subagent sessions survive restart and are listed as children of their parent.
- Unknown `subagent_type` errors; `subagent_depth` limits nesting; subagent permissions derive from
  the parent session with default `todowrite`/`task` denies.
- TUI: from a root session, `ctrl+x` then `down` enters the first child subagent session; from a
  child, `up` returns to the parent and left/right cycle siblings.
- TUI renders the `"<shortcut> view subagents"` hint on task tool parts and a subagent footer with
  label, "(index of total)", and Parent/Prev/Next.
- Background subagents (when the experimental gate is on) return immediately, notify the parent on
  completion/failure, and can be backgrounded from a running foreground task with `ctrl+b`.
- `general` is available as a subagent, and `@agent-name` mention routing invokes the matching
  subagent.
- CLI surfaces child/subagent sessions where a run surface exists (or the absence is recorded as a
  documented partial).
- Each child item's fixtures pass, and side-by-side parity evidence against the (re-pinned) reference
  is recorded.

## Likely touchpoints

- `crates/opencode-tool/src/task.rs`
- `crates/opencode-tool/src/tool.rs`
- `crates/opencode-agent/src/agent.rs`, `crates/opencode-agent/src/executor.rs`
- `crates/opencode-session/src/prompt.rs`, `crates/opencode-session/src/session.rs`
- `crates/opencode-server/src/routes.rs`
- `crates/opencode-storage/src/repository.rs`
- `crates/opencode-tui/src/app/app.rs`, `context/keybind.rs`, `components/dialogs/subagent.rs`,
  `components/session_tool.rs`, `components/session.rs`, `api.rs`
- `crates/opencode-cli/src/main.rs`
- `crates/opencode-config/src/schema.rs` (`subagent_depth`, agent modes)

## Verification

- Re-pin or restore the reference line, then compare the subagent surfaces side-by-side.
- `cargo fmt --all`
- `cargo check --workspace`
- `cargo test -p opencode-tool -p opencode-agent -p opencode-session -p opencode-server -p opencode-tui`
- Manual smoke with `ort-build` then `ort`: spawn a subagent, navigate into it with `ctrl+x down`,
  return with `up`, cycle with left/right, resume with `task_id`, and (with the experimental flag)
  background a task and confirm the parent notification.

## Related Items

- `GATE-002` Question tool full parity - sibling gate; pattern for gate/child structure.
- `FEAT-031` Investigate the builtin `general` agent and decide whether to remove it - done; decided
  to disable `general` for now and flag subagent parity as a separate item (this gate).
- `CLI-002` Route `opencode run` through the canonical session runtime - on hold, gated by
  `CLI-001`/`CLI-006`; its subagent dispatch path overlaps `FEAT-046`.
- `PHASE-002` Transport/runtime parity foundation - phase parent for related parity work.
- `wiki/agent-modes-and-custom-agents.md` - current agent model and the `general` decision.

## Notes

- Created 2026-09-21. Reference initially captured from a stale `dev` tip (`e207624c4`); re-pinned to
  `f54ce313b99a` on 2026-09-21 after confirming the subagent reference files are byte-identical
  across the drift.
- `bd` does not parse dependency metadata; the block relationship is expressed in each child's
  `## Parent` section plus this gate's child list.
- 2026-09-23: execution program composed as `H-009`
  (`handoffs/GATE-004-subagent-feature-parity-program-handoff.md`). It fixes the strict child order
  `FEAT-045 -> FEAT-046 -> FEAT-047 -> FEAT-049 -> FEAT-048 -> FEAT-051 -> CLI-010`, corrects the
  stale line references in this card, and runs one PR at a time with human-gated merges.
- 2026-09-23: `CLI-010` accepted as a documented partial (PR #101): child sessions are no longer
  filtered from `session list`/`session find` (shown with `parentId`/Parent column) and `session
  show` lists children; the interactive run footer and CLI abort command are deferred to `CLI-002`
  (blocked by `GATE-002`). All other non-deferred children (`FEAT-045`–`FEAT-049`, `FEAT-051`) are
  implemented and merged; closing this gate is the final step once `CLI-010` lands.
- 2026-09-23: **GATE-004 closed.** All seven program PRs merged (`FEAT-045` #91, `FEAT-046` #96,
  `FEAT-047` #97, `FEAT-049` #98, `FEAT-048` #99, `FEAT-051` #100, `CLI-010` #101). `FEAT-046` is
  `done`; the remaining child cards are in `qa` pending recorded post-merge QA reports. Program
  handoff `H-009` archived.
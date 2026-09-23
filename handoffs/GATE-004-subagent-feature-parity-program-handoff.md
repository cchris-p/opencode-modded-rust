---
id: "H-009"
title: "GATE-004 subagent feature parity — Program Handoff"
status: "open"
created: "2026-09-23"
updated: "2026-09-23"
owner: ""
target: "development"
blocked_reason: ""
needs_human: ""
items: ["GATE-004", "FEAT-045", "FEAT-046", "FEAT-047", "FEAT-048", "FEAT-049", "FEAT-051", "CLI-010"]
---

# GATE-004 Subagent Feature Parity — Program Handoff

## Objective

Close `GATE-004`: bring the Rust product's subagent feature set to parity with the reference
OpenCode line pinned at `f54ce313b99a6661d7758ad042f7a6e05c8e0972` (`dev`, package `1.18.31`,
2026-09-21). Concretely: the `task` tool must create a **real persisted child session** with
`parent_id`; the task contract (registry lookup, depth limit, permission derivation, `<task ...>`
output) must match the reference; the TUI must navigate into/between subagents with the
`ctrl+x down` "view subagents" affordance and a subagent footer; background subagents must exist
behind the reference's experimental gate; `general` must be a subagent with `@agent-name` mention
routing; the CLI must surface or explicitly document child sessions; and fixtures must pin all of it.

The gate is the binding acceptance spec (`boards/todo/gate-subagent-feature-parity.md`). This
handoff is the execution program that closes it. It is **not** a re-scoping of the gate.

Reference freshness verified at handoff time: `git -C $HOME/repos/opencode-modded fetch origin dev`
then `origin/dev` == the pin (no drift). Re-verify before each side-by-side comparison; if `dev`
moves, re-check the subagent files before relying on line numbers.

## Current State

Cards (all children of `GATE-004`, gap numbers as on the gate):

| Item | Title | Priority | Lane | Gap |
|------|-------|----------|------|-----|
| `FEAT-045` | Subagent child sessions are real, persisted, and parent-linked | P1 | `todo` | 1 |
| `FEAT-046` | Task tool contract parity | P1 | `todo` | 2 |
| `FEAT-047` | TUI subagent navigation and "view subagents" | P1 | `todo` | 3 |
| `FEAT-048` | Background subagents and notification injection | P2 | `todo` | 4 |
| `FEAT-049` | Agent-role and `@agent-name` mention parity | P2 | `todo` | 5 |
| `CLI-010` | CLI subagent surface parity | P2 | `hold` | 6 |
| `FEAT-051` | Subagent parity verification fixtures | P2 | `todo` | 7 |

Current code reality (verified at commit `60e701d`; **card line numbers have drifted — use these**):

- **Two disjoint "subsession" mechanisms exist.**
  1. Real child sessions: only produced by `POST /session/` with `parent_id` →
     `SessionManager::create_child` (`crates/opencode-session/src/session.rs:1151-1162`) →
     `Session::child` (`session.rs:399-426`, sets `parent_id` at `:410`) and persisted because the
     server syncs in-memory sessions to storage (`crates/opencode-server/src/routes.rs:263-267`
     `persist_sessions_if_enabled` → `ServerState::sync_sessions_to_storage`).
  2. Ephemeral `task_<agent>_<uuid>` ids: held in `AgentExecutor.subsessions`
     (`crates/opencode-agent/src/executor.rs:43-60`) and in
     `Session.metadata["subsessions"]` (`crates/opencode-session/src/prompt.rs:110-125`). These
     never create a `Session`, never set `parent_id`, and are not persisted as sessions.
- **The task tool uses mechanism 2.** `TaskTool::execute` calls `ctx.do_create_subsession`
  (`crates/opencode-tool/src/task.rs:117-132`). The callbacks are installed by
  `SessionPrompt::with_persistent_subsession_callbacks`
  (`crates/opencode-session/src/prompt.rs:1756-1830`) for the coding-session path, and by
  `AgentExecutor::with_subsession_callbacks` (`crates/opencode-agent/src/executor.rs:528-610`) for
  the nested tool path. `SubtaskExecutor::execute` (`prompt.rs:2926-2967`) also routes through the
  same callbacks.
- **The server does not provide subsession callbacks.** `run_prompt_turn`
  (`crates/opencode-server/src/routes.rs:2614-2956`) builds `SessionPrompt` with only
  `with_ask_callback`/`with_ask_question_callback` (`:2869-2875`). This is the key integration seam
  for `FEAT-045`.
- **Task tool is registry-bypassing.** `get_available_agents` (`task.rs:176-209`) is a static list
  (`explore/plan/title/summary/compaction/build`) with no `general`; unknown `subagent_type` still
  creates a subsession; `run_in_background` (`task.rs:28-29,71-74`) is parsed but never read.
- **No depth limit config.** No `subagent_depth` / depth key exists in
  `crates/opencode-config/src/schema.rs` (or anywhere under `crates/opencode-config`).
- **`general` is dormant.** `AgentInfo::general()` exists with `mode: Primary`
  (`crates/opencode-agent/src/agent.rs:237-259`) but `BuiltinAgent::General` is omitted from
  `BuiltinAgent::all()` (`agent.rs:53-61`), so `AgentRegistry::new` (`agent.rs:459-469`) does not
  register it. `AgentRegistry::list_primary()`/`list_subagents()` (`agent.rs:637-665`) exist but have
  **no callers**.
- **Subagent session permission derivation is absent.** The reference lives in
  `packages/opencode/src/agent/subagent-permissions.ts`; Rust has no equivalent.
- **TUI navigation is dead.** `session_parent`/`session_child_cycle`/`session_child_cycle_reverse`
  are bound to `ctrl+o`/`ctrl+j`/`ctrl+k` (`crates/opencode-tui/src/context/keybind.rs:179-184`) but
  the leader branch in `app.rs:449-473` matches only `Char` keys `n l m a t b s q u r`; no
  `KeyCode::Down/Up/Left/Right` is handled and none of the three bindings is consumed.
  `SubagentDialog` is instantiated/rendered/scrollable (`app.rs:108,944,1019-1022,1133-1140,1830-1838,4109,4171`);
  `.open()` is never called. No `session_background` binding exists. `task` renders as `"#"` glyph
  only (`crates/opencode-tui/src/components/session_tool.rs:70`), no hint row, no footer.
- **TUI has no children API.** `crates/opencode-tui/src/api.rs` has no `/session/{id}/children`
  method even though the server exposes it (`routes.rs:642-649`). The session list dialog drops
  children (`crates/opencode-tui/src/components/dialogs/session_list.rs:60`).
- **CLI silently hides children.** Root filters at `crates/opencode-cli/src/main.rs:1133,1411,4009,4029`
  (not the gate's older `1095/1330/3525/3545`). `GET /session/{id}/children` is a declared op
  (`main.rs:3763`) but unused by any CLI surface.
- **No fixtures beyond two in-memory-shaped unit tests** (`task.rs:247-378`).

Repo workflow: this repository hosts both the board and the product code, so board/handoff docs are
committed **directly to `development`** (verified: `fea8b36`, `06edf1f`), while implementation lands
per-item on `feature/<ID>-<slug>` branches with PRs targeting `development`, merged only on explicit
human direction. `development` is the default branch locally; `main` is release-only.

## The 7 PRs, In Order

| # | Board ID | Item | Priority | PR branch | Repo(s) |
|---|----------|------|----------|-----------|---------|
| 1 | `FEAT-045` | Real persisted child sessions | P1 | `feature/FEAT-045-subagent-child-sessions` | rust product |
| 2 | `FEAT-046` | Task tool contract parity | P1 | `feature/FEAT-046-task-tool-contract-parity` | rust product |
| 3 | `FEAT-047` | TUI subagent navigation + "view subagents" | P1 | `feature/FEAT-047-tui-subagent-navigation` | rust product |
| 4 | `FEAT-049` | Agent-role + `@agent-name` mention parity | P2 | `feature/FEAT-049-agent-role-mention-parity` | rust product |
| 5 | `FEAT-048` | Background subagents + notification injection | P2 | `feature/FEAT-048-background-subagents` | rust product |
| 6 | `FEAT-051` | Subagent parity fixtures + side-by-side evidence | P2 | `feature/FEAT-051-subagent-parity-fixtures` | rust product |
| 7 | `CLI-010` | CLI subagent surface parity (likely documented partial) | P2 | `feature/CLI-010-cli-subagent-surface` | rust product |

**Dependency order (strict): 1 → 2 → 3 → 4 → 5 → 6 → 7.**

- `FEAT-045` is foundational: nothing else can create/navigate a real child until sessions are real.
- `FEAT-046` depends on `045` (it writes to the real child session and adds depth/permissions/output).
- `FEAT-047` depends on `045` (real navigation targets) and `046` (the `view subagents` hint hangs off
  the task tool part).
- `FEAT-049` depends on `046` (both replace the static task catalog with registry-driven lookup);
  `049` adds the `general` subagent and mention routing on top.
- `FEAT-048` depends on `046` (owns the `background` parameter) and `047` (shares the hint row and
  adds `ctrl+b`).
- `FEAT-051` consolidates fixtures + the side-by-side evidence runbook across all prior items.
- `CLI-010` depends on the gate (and on the CLI run-surface prerequisites `CLI-001`/`CLI-006`, now
  `done`; `CLI-002` is still blocked by `GATE-002`). It is last because it may close as a documented
  partial.

**Swap-able independent pairs:** `FEAT-049` and `FEAT-047` touch mostly disjoint files and could be
swapped; `FEAT-049` and `FEAT-048` could be swapped. Keep the default order above so the human
liaison always has one next item. `FEAT-047` ↔ `FEAT-048` must **not** be swapped (shared TUI hint row
in `session_tool.rs`/`session.rs`/`app.rs`).

## Operating Model — One PR at a Time, Human-Gated

Repeat this cycle exactly seven times, in the table order. **Never start PR *N+1* before the human
liaison confirms PR *N* is merged into `development`.**

1. **Gate check (human):** confirm the previous PR (if any) is merged and `development` is current.
2. **Branch (agent):** `git checkout development && git pull --ff-only && git checkout -b <branch>`.
3. **Board (agent):** move the card `todo` → `doing`; `CLI-010` also moves `hold` → `doing` when it
   starts. Update `status` and the lane directory together.
4. **Implement + verify (agent):** implement, run the per-item verification commands, record results.
   Author the item's fixture slice here (see `FEAT-051`) so the card's acceptance is test-backed.
5. **Open PR + board update (agent):** open a ready-to-review PR targeting `development` using the
   template below; keep the PR branch checked out; append implementation notes (branch, PR URL,
   verification results, limitations); move the card `doing` → `qa` with `status: "qa"`; ask the human
   liaison to review/test and decide on merge.
6. **STOP (human):** the liaison tests on the checked-out branch and explicitly says whether to merge.
   Keep the PR open and the card in `qa` until then.
7. **After merge (agent):** delete the merged branch (remote first), pull `development`, and run
   post-merge QA on `development`. Move the card `qa` → `done` only after QA passes. When all
   non-deferred children are `done` (or `CLI-010` is explicitly recorded as a documented partial),
   close `GATE-004` and archive this handoff.

`main` merges remain a separate human-managed release step.

## Shared Decisions — Lock Early

These are binding for the program; do not reopen without a new user decision. Where a choice is
genuinely implementation-shaped, the recommended shape is stated and the acceptance is behavioral.

1. **Child sessions are the target model; the in-memory map is demoted.** `FEAT-045` replaces the
   `task_*` synthetic path with a real `Session` carrying `parent_id` and the reference title
   `"<description> (@<agent> subagent)"`. Durability comes from the server's existing
   `sync_sessions_to_storage` after the turn; no new storage schema is needed (`sessions.parent_id`
   already exists at `crates/opencode-storage/src/schema.rs:11`; `SessionRepository::list_children`
   at `crates/opencode-storage/src/repository.rs:406-431` already exists but is unused).
   - **Integration seam (confirm at implementation):** the server must supply the real
     create/prompt callbacks because it owns `ServerState.sessions` and the provider registry. The
     recommended low-blast-radius shape is: thread a session-create + prompt capability from
     `run_prompt_turn` into `SessionPrompt` (new optional callbacks or a trait object), and have
     `SessionPrompt::execute_tool_calls` (`prompt.rs:1633-1703`) **prefer server-provided callbacks
     over its in-memory metadata-map fallback**. `AgentExecutor`'s nested path is updated the same
     way. Do not invent a third session store.
   - Keep `do_create_subsession`/`do_prompt_subsession` in `ToolContext` as the capability surface
     (`crates/opencode-tool/src/tool.rs:338-368,494-542`); if a genuinely unset path remains, it may
     fall back but must be documented on `FEAT-045`.
2. **Task output is the reference wrapper.** `<task id="..." state="running|completed|error">` with
   optional `<summary>` and `<task_result>` (or `<task_error>` on failure), per
   `packages/opencode/src/tool/task.ts:64-79`. The Rust `"task_id: ...\n\n<task_result>..."`
   string (`task.rs:135-139`) is replaced. Tool metadata must carry `parentSessionId`, `sessionId`,
   and `model`.
3. **`general` becomes a `Subagent`** with the reference role/prompt ("General-purpose agent for
   researching complex questions and executing multi-step tasks…", `agent.ts:182-195`): add
   `BuiltinAgent::General` back to `BuiltinAgent::all()` and change `AgentInfo::general()` to
   `AgentMode::Subagent`. Do **not** make it a default or a primary. Its permission should match the
   reference intent (defaults + `todowrite` deny); if it lands on `build_agent_ruleset`'s default arm
   (`crates/opencode-permission/src/ruleset.rs:266-358`), add an explicit arm or config so `todowrite`
   is denied, and document any deviation in `wiki/agent-modes-and-custom-agents.md`.
4. **Registry-driven subagent lookup.** The task tool's available set and unknown-type error come from
   `AgentRegistry` filtered by mode (`Subagent`/`All`), not the static catalog. Unknown
   `subagent_type` fails **without creating a session** (
   `Unknown agent type: <x> is not a valid agent type`).
5. **`subagent_depth` is a top-level snake_case config key, default `1`.** Depth is computed by
   walking `parent_id` from the calling session (`sessions.parent_id`), failing at
   `depth >= subagent_depth` with the reference message:
   `Subagent depth limit reached (<n>). Increase "subagent_depth" to allow nested subagents.`
6. **Subagent session permissions derive from the parent session** via a new Rust equivalent of
   `subagent-permissions.ts`: keep parent `external_directory` rules and parent `deny` rules, then add
   `todowrite`/`task` denies unless the subagent's own ruleset already permits them.
7. **TUI navigation follows the reference leader bindings.** Add `session_child_first` = `<leader>down`
   and handle `KeyCode::Down` (first child), `KeyCode::Up` (parent), `KeyCode::Left`
   (cycle reverse), `KeyCode::Right` (cycle) inside the leader branch (`app.rs:449-473`). Enable them
   only when the active session has a parent (for parent/cycle) or children (for child-first),
   matching the reference's enablement. Existing `ctrl+o`/`ctrl+j`/`ctrl+k` registrations are either
   kept as documented direct aliases or removed — no dead bindings may remain. The `task` tool part
   renders `"<shortcut> view subagents"` on assistant messages.
8. **Background subagents stay behind an experimental gate.** Add a Rust equivalent of
   `experimentalBackgroundSubagents` (reference env
   `OPENCODE_EXPERIMENTAL_BACKGROUND_SUBAGENTS`), default **off**, and fail `background: true` with a
   clear reference-style error when off. Expose the capability to the TUI the way the reference
   exposes `capabilities.experimentalBackgroundSubagents`. `ctrl+b` (`session_background`) backgrounds
   a running foreground task and appears in the hint row only when enabled. Cancelling the
   parent/run cancels the background task.
9. **CLI-010 may close as a documented partial.** `CLI-001`/`CLI-006` are `done`, but the CLI still
   has no interactive run footer and `CLI-002` (routing) is blocked by `GATE-002`. Per the card and the
   `GATE-001` precedent, either surface children where a run surface exists, or record the absence as
   a documented partial on `CLI-010` and `GATE-004`, while still stopping the silent root filtering
   where a user can inspect sessions (`main.rs:1133,1411,4009,4029`) and ensuring abort reaches the
   active subagent. A documented partial does not block gate close.

## Per-Item Runbooks

### PR 1 — `FEAT-045` Real persisted child sessions

- **Branch:** `feature/FEAT-045-subagent-child-sessions`
- **Scope:** `task` creates one real child session (`parent_id`, reference title, agent/model
  recorded); child + messages persist across restart; `task_id` resumes the same child; `list_children`
  returns it; no `task_*` id exposed as a session identity; in-memory maps demoted/documented;
  existing in-memory task tests updated to the child-session contract.
- **Touchpoints:** `crates/opencode-tool/src/task.rs`; `crates/opencode-tool/src/tool.rs`
  (`ToolContext` callbacks); `crates/opencode-session/src/prompt.rs`
  (`execute_tool_calls:1633-1703`, `with_persistent_subsession_callbacks:1756-1830`,
  `SubtaskExecutor:2897-2967`); `crates/opencode-session/src/session.rs`
  (`create_child:1151-1162`, `children:1362-1368`); `crates/opencode-agent/src/executor.rs`
  (`with_subsession_callbacks:528-610`); `crates/opencode-server/src/routes.rs`
  (`run_prompt_turn:2614-2956`, child routes `:542-649`); `crates/opencode-server/src/server.rs`
  (`ServerState`, `sync_sessions_to_storage`).
- **Approach:** implement Shared Decisions 1 and 2; wire the server-owned create/prompt capability;
  set the child title, agent, and model; persist via the existing sync; make `task_id` resolve a real
  child or error clearly instead of map lookup. Add a restart/resume integration test.
- **Verify:** `cargo fmt --all`; `cargo check --workspace`; `cargo test -p opencode-tool
  -p opencode-session -p opencode-server`; manual `ort-build` then `ort` — spawn a subagent, restart,
  confirm the child is listed and reopens.
- **Board:** `todo` → `doing` → `qa`; add Dev Notes with the integration shape chosen and any fallback
  left in place.

### PR 2 — `FEAT-046` Task tool contract parity

- **Branch:** `feature/FEAT-046-task-tool-contract-parity`
- **Scope:** registry-driven lookup with unknown-type error; `subagent_depth` (Shared Decision 5);
  parent-derived subagent permissions (Shared Decision 6); subagent model wins over parent; `<task ...>`
  output (Shared Decision 2); `run_in_background` deferred to `FEAT-048` but must error like the
  reference when the experimental gate is off; parameters/JSON schema match the reference field set
  (`description`, `prompt`, `subagent_type`, optional `task_id`, optional `command`, `background`).
- **Touchpoints:** `crates/opencode-tool/src/task.rs`; `crates/opencode-agent/src/agent.rs`
  (`AgentRegistry`, `list_subagents`); `crates/opencode-config/src/schema.rs` (`subagent_depth`);
  new `subagent-permissions` equivalent (likely `crates/opencode-agent/src/agent.rs` or a new module);
  `crates/opencode-permission/src/ruleset.rs` if a permission arm is needed.
- **Verify:** `cargo fmt --all`; `cargo check -p opencode-tool -p opencode-agent -p
  opencode-config`; unit tests for registry lookup, unknown agent, depth limit, permission derivation,
  output formatting; side-by-side output against `packages/opencode/src/tool/task.ts`.
- **Board:** `todo` → `doing` → `qa`; note any deviation.

### PR 3 — `FEAT-047` TUI subagent navigation and "view subagents"

- **Branch:** `feature/FEAT-047-tui-subagent-navigation`
- **Scope:** `ctrl+x down` enters the first child; `up` returns to parent; `left`/`right` cycle
  siblings; enablement rules; compute/sort direct children from `parent_id`; render the
  `"view subagents"` hint on task tool parts; render a subagent footer with label, `(index of total)`,
  and Parent/Prev/Next; wire or remove `SubagentDialog` (no dead dialog); returning to parent restores
  the parent view/session.
- **Touchpoints:** `crates/opencode-tui/src/context/keybind.rs` (`:179-184`, add `session_child_first`);
  `crates/opencode-tui/src/app/app.rs` (leader branch `:449-473`, SubagentDialog wiring
  `:108,944,1019-1022,1830-1838,4109,4171`); `crates/opencode-tui/src/components/session_tool.rs`
  (`:70`, add hint row); `crates/opencode-tui/src/components/session.rs` (footer/children);
  `crates/opencode-tui/src/api.rs` (add `GET /session/{id}/children`); optionally
  `crates/opencode-tui/src/components/dialogs/session_list.rs:60`.
- **Verify:** `cargo fmt --all`; `cargo check -p opencode-tui`; `cargo test -p opencode-tui`; manual
  side-by-side — spawn two subagents, `ctrl+x down`, cycle left/right, return `up`.
- **Board:** `todo` → `doing` → `qa`; note the chosen binding/alias behavior and dialog disposition.

### PR 4 — `FEAT-049` Agent-role and `@agent-name` mention parity

- **Branch:** `feature/FEAT-049-agent-role-mention-parity`
- **Scope:** register `general` as a `Subagent` (Shared Decision 3); task tool offers exactly the
  registry's subagent-capable agents (Shared Decision 4); parse `@agent-name` mentions in prompts and
  route to that subagent with validation and a clear unknown-name error; primaries cannot be invoked
  as subagents; custom agents still declare `mode` and land in the right surface; update
  `wiki/agent-modes-and-custom-agents.md`.
- **Touchpoints:** `crates/opencode-agent/src/agent.rs` (`BuiltinAgent::all:53-61`, `general:237-259`,
  `list_subagents:657-665`); `crates/opencode-tool/src/task.rs`; prompt parsing in
  `crates/opencode-session/src/prompt.rs` and/or `crates/opencode-server/src/agentic.rs`;
  `crates/opencode-permission/src/ruleset.rs`; `wiki/agent-modes-and-custom-agents.md`.
- **Verify:** `cargo fmt --all`; `cargo check -p opencode-agent -p opencode-tool -p opencode-session`;
  `cargo test -p opencode-agent -p opencode-tool`; manual `@explore` in the TUI creates a child;
  side-by-side role list against `agent.ts`.
- **Board:** `todo` → `doing` → `qa`; the wiki update ships in this PR.

### PR 5 — `FEAT-048` Background subagents and notification injection

- **Branch:** `feature/FEAT-048-background-subagents`
- **Scope:** experimental flag + gate (Shared Decision 8); `background: true` returns a running-state
  output immediately and keeps the child running; on completion/failure inject a synthetic `<task ...>`
  message into the parent session; add `session_background` (`ctrl+b`) and the hint-row affordance;
  expose the capability to the TUI; cancel propagates.
- **Touchpoints:** `crates/opencode-tool/src/task.rs`; `crates/opencode-session/src/prompt.rs`
  (async run + parent injection); `crates/opencode-server/src/routes.rs` (capability exposure);
  `crates/opencode-config/src/schema.rs`; TUI `context/keybind.rs`, `app/app.rs`,
  `components/session_tool.rs`, `api.rs` (capability).
- **Verify:** `cargo fmt --all`; `cargo check`/`cargo test -p opencode-tool -p opencode-session
  -p opencode-tui`; gate-off error, gate-on immediate return, completion/failure injection, cancel
  propagation; manual background task notification; output text side-by-side with `task.ts`.
- **Board:** `todo` → `doing` → `qa`; document the flag name and capability surface.

### PR 6 — `FEAT-051` Subagent parity verification fixtures + evidence

- **Branch:** `feature/FEAT-051-subagent-parity-fixtures`
- **Scope:** consolidate the fixture slices authored in PRs 1–5 into the card's coverage table so every
  `GATE-004` acceptance bullet maps to a fixture; add a documented side-by-side parity evidence
  runbook; ensure no fixture asserts on synthetic `task_*` ids.
- **Touchpoints:** tests across `opencode-tool`, `opencode-agent`, `opencode-session`,
  `opencode-server`, `opencode-tui`; `boards/todo/subagent-parity-verification-fixtures.md`.
- **Verify:** `cargo fmt --all`; `cargo test -p opencode-tool -p opencode-agent -p opencode-session
  -p opencode-server -p opencode-tui`; walk the gate's acceptance bullets against the coverage table.
- **Board:** `todo` → `doing` → `qa`; record the evidence runbook on the card or a linked invariant.

### PR 7 — `CLI-010` CLI subagent surface parity (likely documented partial)

- **Branch:** `feature/CLI-010-cli-subagent-surface`
- **Scope:** either surface/nest child sessions where a run surface exists, or record the absence as a
  documented partial on `CLI-010` and `GATE-004` (Shared Decision 9); stop the silent root filtering
  (`main.rs:1133,1411,4009,4029`); ensure CLI abort/cancel reaches the active subagent; no live path
  silently drops children.
- **Touchpoints:** `crates/opencode-cli/src/main.rs`; `GET /session/{id}/children` client wiring.
- **Verify:** `cargo fmt --all`; `cargo check -p opencode-cli`; `cargo test -p opencode-cli` (or its
  test target); manual CLI task run and child observability per the chosen surface.
- **Board:** `hold` → `doing` → `qa` (or straight to a documented partial resolution); update
  `GATE-004`.

## Verification Standards (every PR)

- `cargo fmt --all`.
- `cargo check --workspace` (or the item's crate subset while iterating; run the workspace check before
  opening the PR).
- `cargo test` for the affected crates from the item's runbook.
- Manual TUI smoke where the item has UI behavior: `ort-build` then `ort` (always rebuild before
  launch; `ort` starts a fresh server for the activated workspace).
- Paste exact commands and observed results into the PR body and the card's Dev Notes.
- Side-by-side reference comparisons must state the pin (`f54ce313b99a`) and re-fetch `dev` first.

## PR Description Template

```markdown
## Summary
<one paragraph: what and why, which board item under GATE-004>

## Acceptance Criteria
- [x] <AC text from the board card, checked off as implemented>

## Code Touchpoints
- <file:line or file list>

## Verification
- <exact commands run + results>

## Known Limitations / Deviations
- ...

## Board Card
<FEAT-0XX> (boards/todo|qa/<file>); program: GATE-004 (H-009), PR <n>/7
```

## Human Liaison Checklist

- Per-PR: confirm the previous PR is merged before the next branch starts.
- `FEAT-045`: verify the child session has a stable id (not `task_*`), survives a real server
  restart, and resumes via `task_id` without duplicating.
- `FEAT-046`: verify unknown `subagent_type` errors and creates nothing; depth limit text; output
  wrapper parity.
- `FEAT-047`: verify `ctrl+x down`/`up`/`left`/`right` and the hint/footer; no dead binding or dialog
  remains.
- `FEAT-049`: verify `general` appears as a subagent (not a primary) and `@explore`/`@general` route;
  unknown mentions error.
- `FEAT-048`: verify gate-off error, immediate return, parent notification, `ctrl+b`, cancel.
- `FEAT-051`: verify every gate acceptance bullet has a fixture.
- `CLI-010`: accept or reject the documented-partial resolution.
- Do not merge any PR until the liaison has tested the checked-out branch and explicitly says to merge.

## Out of Scope / Future Work

- General upstream sync or broad OpenCode parity beyond subagents.
- Browser/web-app subagent timeline UI (the Rust product has no browser app).
- Parallel general-purpose subagent orchestration beyond what the task tool provides.
- Persisted background jobs across server restart unless a child decides it is needed.
- Reworking the whole permission system beyond the `task`/`todowrite` subagent rules.
- `CLI-002` routing (blocked by `GATE-002`); `FEAT-028` tool-call compaction; `FEAT-008` sidebar.

## Risks And Rollback

- **`FEAT-045` is the highest-risk item** (crosses tool/session/server/agent crates). Land it as one
  focused commit so it can be reverted independently; verify restart/resume before PR 2.
- **`FEAT-048` (experimental background)** adds async lifecycle and parent message injection; keep it
  off by default and prove cancel propagation.
- **`FEAT-047`/`FEAT-048` share TUI files**; serialize them and rebase PR 5 on merged PR 3.
- **Line drift:** card line numbers are stale; always re-verify against the current tree.

## Execution Notes

### 2026-09-23 — Handoff composed

- Created `H-009` from `GATE-004` plus children `FEAT-045/046/047/048/049/051`, `CLI-010`.
- Verified the reference pin: `$HOME/repos/opencode-modded` `origin/dev` ==
  `f54ce313b99a6661d7758ad042f7a6e05c8e0972` (no drift); re-mapped current Rust touchpoints at
  `60e701d` (card line numbers corrected).
- Started PR 1 (`FEAT-045`) per user direction.

### 2026-09-23 — PR 1 (`FEAT-045`) opened

- Branch `feature/FEAT-045-subagent-child-sessions`; PR #91 targets `development`
  (https://github.com/cchris-p/opencode-modded-rust/pull/91).
- Server now supplies real `create_subsession`/`prompt_subsession` callbacks so `task` creates a real
  persisted child session; in-memory callbacks remain only as a documented fallback. Interim
  `subagent_depth = 1` guard added (config key deferred to `FEAT-046`).
- Fixture slice: `subagent_child_session_tests` in `opencode-server`. Verification green:
  `cargo fmt --all`, `cargo check --workspace`, `cargo test -p opencode-tool -p opencode-session
  -p opencode-server`.
- `FEAT-045` moved `todo -> doing -> qa`; awaiting human test/merge on PR #91 before PR 2 (`FEAT-046`)
  starts.

### 2026-09-23 — PR 1 (`FEAT-045`) merged

- PR #91 merged into `development` at merge commit `7510bf2a5a430fc836de9ec9a9562a02d23a6163` on
  explicit user direction.
- Branch cleanup complete: remote and local `feature/FEAT-045-subagent-child-sessions` deleted.
- `FEAT-045` remains in `qa` pending a recorded post-merge QA report; the merge alone does not
  promote it to `done`.
- Gate check for PR 2: `FEAT-046` may start now that PR 1 is merged; `development` is current at
  `7510bf2` in the local checkout.

### 2026-09-23 — PR 2 (`FEAT-046`) opened

- Branch `feature/FEAT-046-task-tool-contract-parity`; PR #96 targets `development`
  (https://github.com/cchris-p/opencode-modded-rust/pull/96).
- Registry-driven subagent resolution (`ToolContext::resolve_subagent` + `resolve_task_subagent`),
  configurable `subagent_depth` (default 1), parent-derived child permissions
  (`derive_subagent_session_permission`), subagent-model precedence, and the reference `<task ...>`
  wrapper. `background: true` is gated behind the experimental flag (execution stays `FEAT-048`).
- Verification green: `cargo fmt --all`, `cargo check --workspace`,
  `cargo test -p opencode-tool -p opencode-agent -p opencode-config -p opencode-server`.
- `FEAT-046` moved `todo -> doing -> qa`; awaiting human test/merge on PR #96 before PR 3
  (`FEAT-047`) starts.

### 2026-09-23 — PR 2 (`FEAT-046`) merged

- PR #96 merged into `development` at merge commit
  `05752d9109ba3c12babd1963118618269aac4919` on explicit user direction.
- Branch cleanup complete: remote and local `feature/FEAT-046-task-tool-contract-parity` deleted.
- Post-merge QA on `development` passed: `cargo fmt --all`, `cargo check --workspace`,
  `cargo test -p opencode-tool -p opencode-agent -p opencode-config -p opencode-server`.
- `FEAT-046` promoted `qa -> done` after post-merge QA (explicit user completion).
- Gate check for PR 3: `FEAT-047` may start now that PR 2 is merged; `development` is current at
  `05752d9` in the local checkout.

### 2026-09-23 — PR 3 (`FEAT-047`) opened

- Branch `feature/FEAT-047-tui-subagent-navigation`; PR #97 targets `development`
  (https://github.com/cchris-p/opencode-modded-rust/pull/97).
- Reference leader bindings (`<leader>down`, `up`, `left`, `right`) with dead `ctrl+o`/`ctrl+j`/
  `ctrl+k` removed; new `ApiClient::get_session_children`; child/sibling navigation and cycle order
  mirrored from `index.tsx`; subagent footer (label, `(index of total)`, Parent/Prev/Next); the
  `ctrl+x down view subagents` hint on `task` messages; dead `SubagentDialog` removed.
- Verification green: `cargo fmt --all`, `cargo check --workspace`,
  `cargo test -p opencode-tui --lib` (134 passed), including new navigation/footer tests.
- `FEAT-047` moved `todo -> doing -> qa`; awaiting human test/merge on PR #97 before PR 4
  (`FEAT-049`) starts. Manual TUI side-by-side is pending human verification.

### 2026-09-23 — PR 3 (`FEAT-047`) merged

- PR #97 merged into `development` at merge commit
  `f48c4c51c956fe74bf571695703118eaddb93eeb` on explicit user direction.
- Branch cleanup complete: remote and local `feature/FEAT-047-tui-subagent-navigation` deleted.
- `FEAT-047` kept in `qa` by explicit user direction (not promoted to `done`); post-merge QA on
  `development` is still outstanding.
- Gate check for PR 4: `FEAT-049` may start now that PR 3 is merged; `development` is current at
  `f48c4c5` in the local checkout.

### 2026-09-23 — PR 4 (`FEAT-049`) opened

- Branch `feature/FEAT-049-agent-role-mention-parity`; PR #98 targets `development`
  (https://github.com/cchris-p/opencode-modded-rust/pull/98).
- `general` registered as a `Subagent` (reference description, no dedicated prompt) and never a
  primary/default; `build_agent_ruleset("general")` adds the reference `todowrite` deny.
- `@agent-name` mentions now resolve file-first, then fall back to a registered subagent, producing
  a `PartInput::Agent` (reference-style "call the task tool with subagent: X" instruction). A
  registered non-subagent mention fails with a clear error; unknown tokens stay plain text.
- Verification green: `cargo fmt --all`, `cargo check --workspace`,
  `cargo test -p opencode-permission -p opencode-agent -p opencode-tool -p opencode-server --lib`,
  `cargo test -p opencode-session --lib resolve_prompt_parts`.
- `FEAT-049` moved `todo -> doing -> qa`; awaiting human test/merge on PR #98 before PR 5
  (`FEAT-048`) starts. TUI `@` autocomplete surfacing subagents is a documented follow-up.

### 2026-09-23 — PR 4 (`FEAT-049`) merged

- PR #98 merged into `development` at merge commit
  `24207e0edaff90b1ade5d9c5f88e459653940245` on explicit user approval.
- Branch cleanup complete: remote and local `feature/FEAT-049-agent-role-mention-parity` deleted;
  local `development` fast-forwarded to `24207e0`.
- `FEAT-049` remains in `qa` pending a recorded post-merge QA report (the merge alone does not
  promote it to `done`).
- Gate check for PR 5: `FEAT-048` may start now that PR 4 is merged; `development` is current at
  `24207e0` in the local checkout.

### 2026-09-23 — PR 5 (`FEAT-048`) opened

- Branch `feature/FEAT-048-background-subagents`; PR #99 targets `development`
  (https://github.com/cchris-p/opencode-modded-rust/pull/99).
- Experimental gate via `experimental.background_subagents` / `OPENCODE_EXPERIMENTAL_BACKGROUND_SUBAGENTS`;
  `background: true` returns a running-state `<task ...>` immediately and runs the child detached.
  Completion/error injects a synthetic `<task ... state="completed|error">` message into the parent.
  New `POST /session/{id}/background` + TUI `ctrl+b` (`session_background`) promote a running
  foreground task; the task hint row adds `ctrl+b background` when the capability is on.
- Verification green: `cargo fmt --all`, `cargo check --workspace`,
  `cargo test -p opencode-tool -p opencode-agent -p opencode-config -p opencode-server --lib`,
  `cargo test -p opencode-tui --lib` (135 passed), and `opencode-session` excluding the pre-existing
  environmental `instruction::` failures.
- `FEAT-048` moved `todo -> doing -> qa`; awaiting human test/merge on PR #99 before PR 6
  (`FEAT-051`) starts. Documented deviation: the parent observes the injected result on its next
  turn rather than auto-resuming.

## Completed With

- PR 1 (`FEAT-045`) — PR #91 / merge commit `7510bf2a5a430fc836de9ec9a9562a02d23a6163` (merged
  2026-09-23). Card `FEAT-045` in `qa`.
- PR 2 (`FEAT-046`) — PR #96 / merge commit `05752d9109ba3c12babd1963118618269aac4919` (merged
  2026-09-23). Card `FEAT-046` in `done`.
- PR 3 (`FEAT-047`) — PR #97 / merge commit `f48c4c51c956fe74bf571695703118eaddb93eeb` (merged
  2026-09-23). Card `FEAT-047` kept in `qa` pending post-merge QA.
- PR 4 (`FEAT-049`) — PR #98 / merge commit `24207e0edaff90b1ade5d9c5f88e459653940245` (merged
  2026-09-23). Card `FEAT-049` in `qa` pending post-merge QA.


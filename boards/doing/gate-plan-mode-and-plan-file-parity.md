---
id: "GATE-003"
title: "Gate: plan mode and the plan file must match vanilla OpenCode exactly"
priority: "P1"
type: "gate"
area: "GATE"
spec: ""
status: "doing"
predecessors: ""
created: "2026-09-25"
---

# Gate: plan mode and the plan file must match vanilla OpenCode exactly

## Summary

Hard gate. Plan mode and its plan-file behavior must match the reference OpenCode exactly. No
plan-mode-dependent story may be treated as complete until the plan agent, the plan file location and
permissions, and the injected plan-mode prompt match vanilla.

Vanilla **does** write a plan file, but not the way the Rust product currently does. Vanilla writes to
a session-scoped, timestamped file under the project's `.opencode/plans/` directory (or the global data
`plans/` directory when the project has no VCS), injects the exact plan-file path and workflow into the
model prompt, and permits editing **only** that plan file while denying every other edit. The Rust
product uses a single fixed `.opencode/PLAN.md`, injects a short custom prompt that never names the
path, and its plan agent denies **all** edits with no allow rule for the plan file — so the model has
no consistent, permitted way to write the plan.

This card records the vanilla behavior to target, the current Rust gaps, and the acceptance criteria.

## Gate requirement

> Plan mode and the plan file must match vanilla OpenCode exactly: same plan file path and naming, same
> edit allow-list, same injected plan-mode prompt and workflow, and the same observable plan-file
> lifecycle (create when absent, edit incrementally when present, report the path on exit).

Parity is judged against the frozen reference source (below), not a paraphrase. Where the Rust runner
has no equivalent mechanism, the observable behavior must still match; internals that cannot be reused
are reference-only and called out under "Resolved parity decisions".

## Vanilla behavior to target (frozen reference)

### 0. Source of truth

`$HOME/repos/opencode-modded` at the repo's recorded reference commit
`f54ce313b99a6661d7758ad042f7a6e05c8e0972` (branch `dev`, package `1.18.31`, 2026-09-21) per
`AGENTS.md`. Line references below were verified at that commit.

### 1. Plan file path and naming (`session.ts:331-336`)

```
export function plan(input: { slug: string; time: { created: number } }, instance: InstanceContext) {
  const base = instance.project.vcs
    ? path.join(instance.worktree, ".opencode", "plans")
    : path.join(Global.Path.data, "plans")
  return path.join(base, [input.time.created, input.slug].join("-") + ".md")
}
```

- The plan file is **session-scoped and timestamped**: `<time.created>-<slug>.md`.
- Location is `<worktree>/.opencode/plans/` when the project has VCS, otherwise
  `<Global.Path.data>/plans/`.
- It is **not** a single fixed `PLAN.md`.

### 2. Plan agent permissions (`agent.ts:156-181`)

The `plan` primary agent merges the defaults with:

- `question: "allow"`, `plan_exit: "allow"`, `task.general: "deny"`.
- `external_directory[<Global.Path.data>/plans/*]: "allow"`.
- `edit`: `"*": "deny"` **except** `.opencode/plans/*.md` and the
  relative global-data `plans/*.md` path, both `allow`.

So the model may edit **only** the plan file and is denied every other edit. This allow-list is what
makes "create/modify the plan file" legal in plan mode.

### 3. Plan-mode prompt injection (`reminders.ts:51-90`, `session/prompt/plan-mode.txt`)

- When the effective agent is `plan` and it was not already the previous assistant agent, the reminder
  is injected as a **synthetic text part** on the last user message (`reminders.ts:70-88`).
- The plan path is computed with `Session.plan`, and `ensureDir(path.dirname(plan))` is called when the
  file does not exist (`reminders.ts:73-75`).
- The `${planInfo}` placeholder is filled with either
  `A plan file already exists at <abs path>. You can read it and make incremental edits using the edit tool.`
  or
  `No plan file exists yet. You should create your plan at <abs path> using the write tool.`
  (`reminders.ts:81-85`).
- `plan-mode.txt` supplies the full workflow: read-only contract, plan-file-only edit exception, Phase 1
  explore agents, Phase 2 design agent, Phase 3 review/questions, Phase 4 "write your final plan to the
  plan file", Phase 5 call `plan_exit`. It instructs building the plan incrementally in the file.
- A parallel read-only reminder (`session/prompt/plan.txt`, plus the Anthropic variant
  `plan-reminder-anthropic.txt`) reinforces the no-edit constraint.
- On the plan -> non-plan transition (`reminders.ts:51-68`), a `BUILD_SWITCH` reminder is injected; if a
  plan file now exists, it appends `A plan file exists at <abs path>. You should execute on the plan
  defined within it`.

### 4. Plan tools (`tool/plan.ts`, `tool/registry.ts`, `agent.ts:149`)

- Only `plan_exit` is a registered model tool (`registry.ts` imports `PlanExitTool` only;
  `tool/plan.ts:15-79`).
- `plan_exit` computes the plan path with `Session.plan`, reports it **relative to the worktree** in the
  question ("Plan at `<rel path>` is complete..."), and on confirmation injects a synthetic user
  message that references the plan by absolute path:
  `The plan at <abs path> has been approved, you can now edit files. Execute the plan`
  (`tool/plan.ts:29-69`).
- `plan_enter` is **not** a registered tool at this commit. It is a permission action only
  (`agent.ts:149`, `cli/cmd/run.ts:439`, `packages/core/src/plugin/agent.ts:112,128`). The TUI reacts to
  a `plan_enter` part by setting the agent to `plan` (`packages/tui/src/routes/session/index.tsx:337`),
  but no `plan_enter` tool is defined; `tool/plan-enter.txt` is unreferenced. Entering plan mode is
  client/CLI-driven, not a model tool call.

## Current Rust implementation and gaps

- **Fixed plan file.** `PLAN_FILE: &str = "PLAN.md"` and
  `get_plan_path` returns `<worktree>/.opencode/PLAN.md` (`crates/opencode-tool/src/plan.rs:17,238-242`).
  No VCS-vs-global-data branch, no session slug, no `time.created` timestamp; every session shares one
  file (`crates/opencode-tool/src/plan.rs:84-85,172-173`).
- **Prompt never names the path and does not match vanilla.** `PROMPT_PLAN`
  (`crates/opencode-session/src/prompt.rs:3383-3397`) is a short custom prompt ("Create a detailed plan
  in the plan file") with no plan-file path, no `${planInfo}` existence/incremental-edit semantics, and
  no Phase 1-5 workflow from `plan-mode.txt`. The reminder is pushed as a plain `PartType::Text` with no
  `<system-reminder>` wrapper (`prompt.rs:3422-3434`), unlike vanilla's synthetic reminder shape.
- **Plan agent cannot write the plan file.** `build_agent_ruleset("plan", ..)` denies `edit` with
  `pattern: "*"` and adds **no** allow rule for `.opencode/plans/*.md` (or any plan file)
  (`crates/opencode-permission/src/ruleset.rs:277-296`). Combined with the missing path in the prompt,
  the model has no permitted, identified file to write, which is the opposite of vanilla's
  edit-everything-except-the-plan-file allow-list.
- **`plan_enter` is a model tool.** The Rust product registers `PlanEnterTool` and `PlanExitTool`
  (`crates/opencode-tool/src/registry.rs:250-251`), lists `plan_enter` in the tool-id surface
  (`crates/opencode-server/src/routes.rs:7252`), and asks the user to switch via a model-invoked tool
  (`plan.rs:62-148`) with the fixed `PLAN.md` path. Vanilla exposes only `plan_exit` as a model tool
  and treats `plan_enter` as a client-driven permission action. The Rust TUI does **not** consume a
  `plan_enter` tool part: plan mode is entered by selecting the `plan` agent
  (`crates/opencode-tui/src/components/dialogs/agent_select.rs:252`), so removing the tool does not
  break entry.
- **No `ensureDir` / existence handling.** The Rust path is a fixed file with no directory creation and
  no "exists -> edit incrementally, absent -> create" branching.
- **No plan prompt invariants doc.** There is no `invariants/` entry for plan mode or the plan file.

## Target behavior for the Rust product (derived, implementable)

1. Resolve the plan file with a single `Session.plan`-equivalent function:
   - VCS project: `<worktree>/.opencode/plans/<time.created>-<slug>.md`.
   - No-VCS project: `dirs::data_local_dir()/opencode/plans/<time.created>-<slug>.md`
     (`~/Library/Application Support/opencode/plans/...` on macOS; the Rust product data root is
     `dirs::data_local_dir()/opencode`, per `crates/opencode-storage/src/database.rs:139` and
     `crates/opencode-tui/src/trace.rs:50`).
   Use the session's `slug` and `time.created` (ms). Every caller (reminder injection and both tools)
   must use this one function.
2. Grant the plan agent edit permission for the plan file only: deny `edit: "*"` and allow the resolved
   plan path (both the worktree `.opencode/plans/*.md` and the global data `plans/*.md` forms), plus the
   matching `external_directory` allow. Every other edit stays denied.
3. Inject the vanilla plan-mode prompt (the `plan-mode.txt` content) with `${planInfo}` filled from the
   resolved path and its existence, as a synthetic reminder part, after `ensureDir`. Inject the
   `BUILD_SWITCH` reminder on the plan -> non-plan transition, including the "a plan file exists at ..."
   line when present.
4. `plan_exit` reports the plan path relative to the worktree in its question and references the
   absolute path in the injected synthetic message.
5. Remove the model `plan_enter` tool to match vanilla; plan-mode entry stays client-driven via `plan`
   agent selection. `plan_exit` remains the only plan-mode model tool.

## Implementation constraints (must be satisfied)

1. **One path function.** Plan path resolution must not be duplicated literal strings across
   `plan.rs` and `prompt.rs`; a single helper is the source of truth.
2. **Session scoping.** Two sessions in the same worktree must not share one plan file.
3. **Permission/prompt consistency.** The path advertised in the prompt must be the exact path the plan
   agent is allowed to edit; a mismatch (as today) fails this gate.
4. **No regression to build behavior.** Build mode keeps full edit access; only the plan agent gains the
   narrow plan-file allow.
5. **Tests.** Add coverage for: path resolution (VCS vs no-VCS, session scoping, timestamped name),
   plan-agent permission (plan file allowed, other edits denied), and reminder content (path present,
   create-vs-edit wording, `BUILD_SWITCH` on transition).

## Resolved parity decisions

- **`plan_enter` tool: remove it.** The Rust TUI does not consume a `plan_enter` tool part; plan mode
  is entered by selecting the `plan` agent. Remove `PlanEnterTool` (registry.rs:250), its tool id
  (routes.rs:7252), and any test that asserts it is model-visible; keep the `plan_enter` permission
  for client-driven entry, matching vanilla. `plan_exit` remains a model tool.
- **Global data plans dir: use the Rust product data root.**
  `dirs::data_local_dir()/opencode/plans/`, not vanilla's `~/.local/share/opencode/plans/`. The
  `external_directory` allow must reference whichever global plans path the resolver returns.
- **Reminder variant: single provider-agnostic reminder.** Port `session/prompt/plan-mode.txt` as the
  one plan-mode reminder; do not add the Anthropic-only `plan-reminder-anthropic.txt` variant in this
  card.

## Implementation anchors (current Rust)

- **Session identity for the path:** `Session.slug: String`
  (`crates/opencode-session/src/session.rs:288`) and `Session.time.created: i64` in milliseconds
  (`crates/opencode-session/src/session.rs:85-97`) map to vanilla's `input.slug` and
  `input.time.created`.
- **Reminder injection:** `insert_reminders(messages, agent_name, was_plan)` is called at
  `crates/opencode-session/src/prompt.rs:1226-1227`. It currently has no session, worktree, or data-root
  context; extend it (or the surrounding call) to pass the resolved plan path so it can fill
  `${planInfo}` and call `ensureDir`.
- **Tool path resolution:** `get_plan_path(&ctx)` (`crates/opencode-tool/src/plan.rs:238-242`) only has
  `ctx.worktree`. `ToolContext` (`crates/opencode-tool/src/tool.rs:476-512`) carries `session_id` and
  `session_inspect`, so resolve `slug`/`time.created` through `session_inspect` or add a dedicated
  session-plan-path callback; do not hard-code `PLAN.md`.
- **Permission change site:** `build_agent_ruleset("plan", ..)` in
  `crates/opencode-permission/src/ruleset.rs:277-296` - add the plan-file edit allow(s) and the
  `external_directory` allow next to the existing `edit: "*" -> Deny`.
- **Prompt text:** `PROMPT_PLAN`/`BUILD_SWITCH` in `crates/opencode-session/src/prompt.rs:3383-3408`
  are replaced by the ported `plan-mode.txt` content and the plan-file transition text.

## Scope

- Plan file path/naming/location resolution matching vanilla.
- Plan-agent edit allow-list for the plan file and the matching `external_directory` allow.
- Plan-mode prompt injection: path/existence info, `plan-mode.txt` workflow, synthetic reminder shape,
  directory creation, and `BUILD_SWITCH` on transition.
- `plan_exit` path reporting (relative in the question, absolute in the injected message).
- Removing the model `plan_enter` tool.
- Tests for path resolution, permissions, and reminders.

## Non-goals

- General prompt/agent parity beyond plan mode (owned by other gates/cards).
- Re-architecting the permission engine or the reminder pipeline beyond what plan mode requires.
- ScopeMux plan nodes (`WI-031`/`WI-032`/`SCOPE-003`), which are a separate durable-map concept and are
  unrelated to the plan-mode plan file.
- Changing plan mode into build mode or allowing broad edits in plan mode.

## Done when

- [x] The plan file resolves to a session-scoped timestamped file under `.opencode/plans/` (VCS) or the
      product's data `plans/` directory (no VCS), via one shared helper.
- [x] The plan agent can edit the resolved plan file and is denied every other edit; build mode is
      unaffected.
- [x] The plan-mode prompt names the plan path, distinguishes create vs incremental edit, carries the
      vanilla workflow, and is injected as a synthetic reminder after the directory is ensured.
- [x] `plan_exit` reports the worktree-relative path in its question and the absolute path in the
      injected message.
- [x] The model `plan_enter` tool is removed (registry, tool-id surface, tests) and `plan_exit` is the
      only plan-mode model tool; client-driven entry still works.
- [x] `BUILD_SWITCH` is injected on the plan -> non-plan transition, with the plan-file line when a plan
      file exists.
- [x] Side-by-side parity evidence against the pinned reference commit is recorded.
- [x] Tests for path resolution, the plan-agent permission set, and reminder content pass.

## Recommended verification

- Create two sessions in one VCS worktree, enter plan mode in each, and confirm two distinct
  timestamped files under `.opencode/plans/`, never a shared `PLAN.md`.
- In plan mode, confirm the model can create/edit the plan file and is denied editing a regular source
  file, then switch to build and confirm full edit access returns.
- Confirm the injected plan reminder contains the exact resolved path and the correct create-vs-edit
  wording, and that the plan -> build transition reminder appears.
- Compare path, naming, prompt text, permission set, and `plan_exit` messaging side-by-side with the
  reference at `f54ce313b99a6661d7758ad042f7a6e05c8e0972`.
- `cargo test -p opencode-tool -p opencode-permission -p opencode-session` for the new coverage.

## Related Items

- `BUG-016` Plan-mode session stalls after tool calls without tool results - the plan-mode tool-loop
  failure whose fix should not cement a divergent plan-file contract.
- `BUG-023` Root-cause why the plan-mode session stalled after tool calls without results - root-cause
  evidence for the same surface.
- `wiki/agent-modes-and-custom-agents.md` - current documented plan-mode permissions and mode model.
- `GATE-001`/`GATE-004` - the vanilla-parity gate convention this card follows.

## Notes

- Created on 2026-09-25; no pre-existing `GATE-003` card existed.
- Verified against the frozen reference pin `f54ce313b99a6661d7758ad042f7a6e05c8e0972`; if a later board
  item re-pins the reference, re-evaluate the line references here.

## Dev Notes

Implemented on branch `gate-003-plan-mode-parity`.

- **One path helper.** Added `opencode_core::plan_file_path` / `plans_dir` / `opencode_data_dir`
  (`crates/opencode-core/src/plan.rs`). It resolves `<worktree>/.opencode/plans/<created>-<slug>.md`
  for a VCS project (`.git` present, file or dir) and `<data_dir>/plans/...` otherwise. Both the
  reminder injection and `plan_exit` call it; no fixed `PLAN.md` remains.
- **Reminder injection.** `insert_reminders` now takes the resolved `PlanReminder { path, exists }`
  plus `last_assistant_was_plan` and injects the ported `plan-mode.txt` as a synthetic text part only
  on entry into plan mode, creating the plans directory first when the file is absent. The
  create-vs-edit `${planInfo}` wording matches the reference. The plan -> non-plan transition injects
  the ported `build-switch.txt`, appending the "A plan file exists at ..." line when present.
  `session_plan_reminder(session)` is the single builder; the prompt loop also attaches the resolved
  path to `ToolContext.plan_path`.
- **Permissions + tool visibility.** `opencode_permission::plan_file_edit_rules` appends `edit` allows
  for the worktree and global plans directories plus an `external_directory` allow for the global
  plans dir. `resolve_agentic_context` appends these to the `plan` ruleset (where the worktree is
  known) before `resolve_tools` runs. `resolve_tools` now hides tools using the reference `disabled`
  semantics (last matching rule for the permission is a `*` deny) instead of `tool_permission_decision`
  on pattern `*`, so a trailing plan-file allow keeps `edit`/`write` declared while every other edit is
  still denied per call. The explicit `allowed_tools` allow-list (e.g. `explore`) is preserved.
- **Tools.** Removed `PlanEnterTool` (registry + `list_tool_ids` surface); `plan_enter` remains a
  permission action only. `plan_exit` reports the worktree-relative path in its question and the
  absolute path in the injected synthetic message, and uses the ported `plan-exit.txt` description.
- **Docs.** Added `invariants/plan-mode.md`, linked from `invariants/README.md`, and updated the
  plan-agent row in `wiki/agent-modes-and-custom-agents.md`.

Parity evidence (pin `f54ce313b99a6661d7758ad042f7a6e05c8e0972`):

- Path/naming: `session.ts` `Session.plan` `[time.created, slug].join("-") + ".md"` ->
  `plan_file_path` `format!("{created_ms}-{slug}.md")`. VCS vs data base matches.
- Prompt: `plan-mode.txt` ported verbatim (including `${planInfo}`); `build-switch.txt` ported
  verbatim; plan-exit description ported from `plan-exit.txt`.
- Permissions: reference `plan` agent `edit: { "*": "deny", ".opencode/plans/*.md": "allow",
  <global plans>: "allow" }`, `external_directory[<global plans>/*]: allow` ->
  `plan_file_edit_rules` (`edit` allows for both plan dirs, `external_directory` allow; pattern is
  `<plans_dir>/*` because the matcher has no `prefix/*.md` form).
- Tools: reference registers `PlanExitTool` only; `plan_enter` is a permission
  (`agent.ts:149`). Rust now matches.
- Tool-availability divergence note: the reference always exposes `edit`/`write` and gates per path;
  Rust previously hid them via blanket pattern-`*` evaluation. Switching `resolve_tools` to the
  reference `disabled` semantics removes that divergence.

Verification: `cargo test -p opencode-core -p opencode-permission -p opencode-tool -p opencode-session`
(164 passed) and `cargo test -p opencode-server` (59 + 3 passed). Two pre-existing macOS failures in
`opencode-session::instruction::tests` (`test_find_up_walks_parents`, `test_find_up_stops_at_stop_dir`)
fail identically on the base commit (temp-dir symlink/canonicalization); they are unrelated and
untouched by this card.

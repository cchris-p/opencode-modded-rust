---
id: "FEAT-051"
title: "Subagent parity verification fixtures"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "qa"
predecessors: ""
created: "2026-09-21"
---

# Subagent parity verification fixtures

## Summary

Child of `GATE-004` (parity gap 7). Add focused fixtures that pin the subagent behaviors the gate
depends on, so the parity work in `FEAT-045` through `CLI-010` is verifiable and regressions are
caught.

## Parent

`GATE-004` subagent feature parity (`boards/todo/gate-subagent-feature-parity.md`), gap 7.

## Problem

- The only Rust subagent tests are the two task-tool unit tests built around the in-memory subsession
  path (`crates/opencode-tool/src/task.rs:247-378`); they will not survive the child-session
  migration unchanged.
- There are no fixtures for child session creation, `task_id` resume, depth limits, subagent
  permissions, TUI child navigation, background notification injection, or agent-role filtering.
- Parity claims in `GATE-004` need reproducible checks, not just manual smoke.

## Vanilla reference

Reference `f54ce313b99a` test coverage to mirror conceptually:

- `packages/opencode/test/tool/task.test.ts`
- `packages/opencode/test/agent/agent.test.ts`
- `packages/app/e2e/regression/subagent-child-navigation.spec.ts`
- `packages/opencode/test/cli/run/subagent-data.test.ts`

## Scope / deliverables

- Task tests: creates exactly one child session with `parent_id`; title format `"<desc> (@<agent>
  subagent)"`; `task_id` resumes the same session; unknown agent errors; depth limit enforced.
- Permission tests: subagent session ruleset derives parent denies plus default `todowrite`/`task`
  denies.
- TUI tests: child set computation and ordering; `ctrl+x down` selects the first child; left/right
  cycle; `up` returns to parent; enablement rules.
- Background tests: gate-off error; gate-on immediate return; completion and failure notification
  injection; cancel propagation.
- Agent-role tests: registry subagent filtering; `@agent-name` routing; unknown mention error.
- A documented side-by-side parity evidence procedure against the (re-pinned) reference.

## Acceptance criteria

- Each `GATE-004` acceptance criterion has at least one automated or explicitly scripted fixture.
- The fixtures fail before the corresponding child item lands and pass after.
- Fixtures are resilient to the in-memory → child-session migration (no assertions on synthetic
  `task_*` ids).
- A short parity evidence runbook is recorded in this card or a linked invariant.

## Verification

- `cargo test -p opencode-tool -p opencode-agent -p opencode-session -p opencode-server -p opencode-tui`
- Confirm every `GATE-004` acceptance bullet maps to a fixture in this card's coverage table.

## Dev Notes

- Consolidation of the per-item fixture slices from `FEAT-045`–`FEAT-049`; no production behavior
  changed. Added the fixtures for previously-uncovered gaps:
  - `opencode-config::tests::experimental_background_subagents_defaults_off_and_honors_config`.
  - `opencode-server::subagent_child_session_tests::abort_on_drop_cancels_unless_disarmed`.
  - `opencode-server::subagent_child_session_tests::cancelled_background_run_injects_nothing_into_the_parent`.
- Migration resilience checked: the `task` tool fixtures assert on real `ses_*` ids supplied by the
  create/prompt callbacks; the only `task_*` assertions are in the in-memory fallback roundtrip test.
- Verification: `cargo fmt --all`; `cargo check --workspace`;
  `cargo test -p opencode-tool -p opencode-agent -p opencode-config -p opencode-permission
  -p opencode-server --lib`; `opencode-session --lib` excluding the pre-existing environmental
  `instruction::` failures; `opencode-tui --lib` green. CLI coverage (bullet 8) is deferred to
  `CLI-010` (PR 7/7) and recorded as a documented partial.

## Fixture coverage (GATE-004 acceptance → fixture)

Consolidated from PRs 1–5 (`FEAT-045`–`FEAT-049`) plus the fixtures added here. Test names are
crate-local; run them with the crate's `--lib` target.

| # | GATE-004 acceptance | Fixture(s) |
|---|---------------------|------------|
| 1 | Real child session with `parent_id` + `"<desc> (@<agent> subagent)"` title; `task_id` resumes the same session | `opencode-server` `subagent_child_session_tests::task_child_is_real_parent_linked_and_persisted`; `opencode-tool` `task::tests::task_reuses_existing_task_id_without_creating_subsession` |
| 2 | Unknown `task_id` errors without creating a session; child survives persistence and is listed as a child | `opencode-server` `subagent_child_session_tests::unknown_task_id_errors_instead_of_creating_a_session`; `opencode-server` `server::tests::storage_roundtrip_restores_sessions_and_messages` (persistence/list) |
| 3 | Unknown `subagent_type` errors; `subagent_depth` limits nesting; child permissions derive from parent with default `todowrite`/`task` denies | `opencode-server` `subagent_child_session_tests::{resolve_task_subagent_rejects_unknown_and_primary_agents, nested_task_beyond_depth_is_rejected, child_session_receives_parent_derived_permission_denies}`; `opencode-tool` `task::tests::{unknown_subagent_errors_without_creating_a_session, depth_limit_error_propagates_from_resolver}`; `opencode-agent` `agent::tests::derive_subagent_permission_{denies_todo_and_task_unless_permitted, keeps_parent_denies_and_external_directory, honors_subagent_task_and_todo_permission}` |
| 4 | TUI `ctrl+x down` enters the first child; `up` returns to parent; left/right cycle siblings | `opencode-tui` `context::keybind::tests::{subagent_navigation_defaults_follow_the_reference, leader_chord_renders_prefixed_shortcut}`; `opencode-tui` `app::app::tests::{first_child_is_the_lowest_id, first_sibling_skips_the_current_session, cycle_sibling_matches_reference_direction_order, cycle_sibling_is_a_noop_without_siblings}` |
| 5 | TUI renders `"<shortcut> view subagents"` on task parts + subagent footer label / "(index of total)" / Parent/Prev/Next | `opencode-tui` `components::session::tests::{task_hint_uses_leader_chord_and_muted_suffix, task_hint_includes_background_when_capability_enabled, subagent_label_reads_agent_name_from_title}` |
| 6 | Background (gate on) returns immediately, notifies parent on completion/failure, `ctrl+b` promotes a foreground task, cancel propagates | `opencode-tool` `task::tests::{background_requires_the_experimental_gate, background_starts_without_waiting_and_reports_running_state, promoted_foreground_task_reports_running_state, render_task_output_matches_reference_for_completed_and_error}`; `opencode-server` `subagent_child_session_tests::{render_background_task_message_matches_reference, inject_background_result_appends_synthetic_parent_message, cancelled_background_run_injects_nothing_into_the_parent, abort_on_drop_cancels_unless_disarmed}`; `opencode-config` `schema::tests::experimental_background_subagents_defaults_off_and_honors_config` |
| 7 | `general` available as a subagent; `@agent-name` routes to the matching subagent; unknown/non-subagent mentions handled | `opencode-agent` `agent::tests::{general_is_a_subagent_and_never_a_primary, resolve_subagent_accepts_subagents_and_rejects_primaries_and_unknown}`; `opencode-permission` `ruleset::tests::general_ruleset_denies_todowrite_and_keeps_default_allows`; `opencode-session` `prompt::tests::{resolve_prompt_parts_agent_fallback, resolve_prompt_parts_routes_each_subagent_once, resolve_prompt_parts_deduplicates, resolve_prompt_parts_rejects_non_subagent_mention, resolve_prompt_parts_leaves_unknown_mentions_as_text}`; `opencode-server` `subagent_child_session_tests::{resolve_task_subagent_accepts_general_as_subagent, resolve_prompt_parts_routes_subagent_and_rejects_primary_mention, agent_mention_names_separates_subagents_from_primaries}` |
| 8 | CLI surfaces child/subagent sessions where a run surface exists, or the absence is a documented partial | Pending `CLI-010` (PR 7/7); recorded as an explicit documented partial on `CLI-010`/`GATE-004`. |

Migration resilience: no gate fixture asserts a synthetic `task_*` id. The only `task_*`-keyed
assertions live in the in-memory fallback unit tests (`opencode-session` persisted-subsession
roundtrip), which exercise the documented fallback path, not the gate behavior.

## Parity evidence runbook (reference `f54ce313b99a`)

1. Refresh the reference and confirm the pin:
   - `git -C "$HOME/repos/opencode-modded" fetch origin dev`
   - `git -C "$HOME/repos/opencode-modded" rev-parse origin/dev` ==
     `f54ce313b99a6661d7758ad042f7a6e05c8e0972`. If `dev` moved, re-map line numbers before
     comparing.
2. Side-by-side source comparison (Rust vs reference):
   - `packages/opencode/src/tool/task.ts` ↔ `crates/opencode-tool/src/task.rs`
   - `packages/opencode/src/agent/agent.ts`, `agent/subagent-permissions.ts` ↔
     `crates/opencode-agent/src/agent.rs`
   - `packages/opencode/src/config/config.ts` (`subagent_depth`) ↔
     `crates/opencode-config/src/schema.rs`
   - `packages/tui/src/config/keybind.ts`, `routes/session/index.tsx`,
     `routes/session/subagent-footer.tsx` ↔ `crates/opencode-tui/src/{context/keybind.rs, app/app.rs,
     components/session.rs, components/session_tool.rs}`
   - `packages/opencode/src/cli/cmd/run/{footer.subagent.tsx, subagent-data.ts}` ↔
     `crates/opencode-cli/src/main.rs`
3. Automated fixtures: `cargo fmt --all`; `cargo check --workspace`;
   `cargo test -p opencode-tool -p opencode-agent -p opencode-session -p opencode-server
   -p opencode-tui`.
4. Manual TUI smoke: `ort-build` then `ort`; spawn a subagent; enter it with `ctrl+x down`; return
   with `up`; cycle with left/right; resume with `task_id`. With
   `OPENCODE_EXPERIMENTAL_BACKGROUND_SUBAGENTS=true`: run a `background: true` task, confirm the
   running-state return and the parent notification, and promote a foreground task with `ctrl+b`.
5. Record the date, the confirmed pin, the exact commands, and observed results on the gate card.

### PR Link

- PR #100 (https://github.com/cchris-p/opencode-modded-rust/pull/100) — `feature/FEAT-051-subagent-parity-fixtures` → `development`.
- Program: GATE-004 (H-009), PR 6/7. Awaiting human test/merge on the checked-out branch.

### 2026-09-23 - Merged into `development`

- Merged via PR #100 (merge commit `5ada12291c49d27157254f18f3fa528bbf100bff`) on explicit user
  approval.
- Branch cleanup complete: remote and local `feature/FEAT-051-subagent-parity-fixtures` deleted;
  local `development` fast-forwarded to `5ada122`.
- Card remains in `qa` pending a recorded QA report. The merge alone does not move it to `done`.

## Related Items

- `GATE-004` subagent feature parity.
- `FEAT-045` through `CLI-010` - the work these fixtures verify.
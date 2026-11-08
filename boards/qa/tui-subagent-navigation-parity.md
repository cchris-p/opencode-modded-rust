---
id: "FEAT-047"
title: "TUI subagent navigation and \"view subagents\""
priority: "P1"
type: "feature"
area: "FEAT"
spec: ""
status: "qa"
predecessors: ""
created: "2026-09-21"
---

# TUI subagent navigation and "view subagents"

## Summary

Child of `GATE-004` (parity gap 3). Make the Rust TUI able to enter, cycle, and leave subagent child
sessions, and surface the `ctrl+x` then `down` "view subagents" hint and a subagent footer, matching
the reference.

## Parent

`GATE-004` subagent feature parity (`boards/todo/gate-subagent-feature-parity.md`), gap 3.

## Problem

- `session_parent`, `session_child_cycle`, and `session_child_cycle_reverse` are registered
  (`crates/opencode-tui/src/context/keybind.rs:179-184`) but the leader handler maps only
  `n l m a t b s q u r` and never handles `KeyCode::Down` or child/parent navigation
  (`crates/opencode-tui/src/app/app.rs:428-437`). The bindings are dead.
- `crates/opencode-tui/src/components/session_tool.rs:34` renders a `task` part as `"#"`; there is no
  "view subagents" hint.
- `SubagentDialog` is instantiated (`app.rs:89,231`) but `.open()` is never called.
- There is no subagent footer showing the current subagent's label, position, or sibling navigation.
- Child sessions do not appear because the task tool does not create real children (`FEAT-045`).

## Vanilla reference

Reference `f54ce313b99a`:

- Leader default `ctrl+x` (`packages/tui/src/config/keybind.ts:39,46`).
- `session_child_first` = `<leader>down` (`:103`); `session_parent` = up (`:106`);
  `session_child_cycle` = right (`:104`); `session_child_cycle_reverse` = left (`:105`).
- `moveFirstChild` enters the first child; `moveChild` cycles siblings; commands enabled only when
  the current session has a `parentID` (`packages/tui/src/routes/session/index.tsx:436-470,1050-1085`).
- Task tool parts render `"<shortcut> view subagents"` (`:1489,1512-1513`).
- `SubagentFooter` shows label, "(index of total)", context/cost, and Parent/Prev/Next
  (`packages/tui/src/routes/session/subagent-footer.tsx`).

## Scope / deliverables

- Add `session_child_first` bound to `ctrl+x` then `down`, and handle it in the leader key path.
- Handle `session_parent` (up), `session_child_cycle` (right), and `session_child_cycle_reverse`
  (left), enabled only when the active session has a parent.
- Compute direct children from `parent_id` and sort them the same way the reference does.
- Render `"ctrl+x down view subagents"` (or the current shortcut) on assistant messages containing a
  `task` tool part.
- Render a subagent footer for child sessions with the agent label, "(index of total)", and
  Parent/Prev/Next affordances; add or wire the `SubagentDialog` only where it adds value, or remove
  it if redundant.
- Ensure returning to the parent restores the parent's view and active session correctly.

## Acceptance criteria

- From a root session with children, `ctrl+x` then `down` switches to the first child subagent.
- From a child, `up` returns to the parent; left/right cycles sibling children in order.
- The binding does nothing when the current session has no children/parent, matching the reference's
  enablement rules.
- Task tool messages show the "view subagents" hint.
- A child session shows a footer with label, position, and Parent/Prev/Next.
- Dead keybind/dialog code is either wired or removed; no unused subagent dialog remains.
- `cargo test -p opencode-tui` passes with new navigation tests.

## Verification

- Unit tests for child selection and cycle order, and for the leader `Down` mapping.
- Manual side-by-side with the reference: spawn two subagents, enter with `ctrl+x down`, cycle with
  left/right, return with `up`.
- `cargo check -p opencode-tui`.

## Related Items

- `GATE-004` subagent feature parity.
- `FEAT-045` child session persistence - required for real navigation targets.
- `FEAT-048` background subagents - owns the `ctrl+b` background affordance shown in the same hint row.

## Dev Notes

Program: `GATE-004` (H-009), PR 3/7. Branch `feature/FEAT-047-tui-subagent-navigation`.
PR: https://github.com/cchris-p/opencode-modded-rust/pull/97

### What changed

- **Reference navigation bindings.** `session_child_first` = `<leader>down`, `session_parent` =
  `up`, `session_child_cycle` = `right`, `session_child_cycle_reverse` = `left`
  (`crates/opencode-tui/src/context/keybind.rs`). The dead `ctrl+o`/`ctrl+j`/`ctrl+k`
  registrations were removed.
- **Leader handling.** The leader branch in `app.rs` now dispatches `KeyCode::Down/Up/Left/Right`
  to new `CommandAction::SessionChildFirst/SessionParent/SessionChildCycle`/`...Reverse` variants.
- **Real navigation.** New `ApiClient::get_session_children`
  (`GET /session/{id}/children`) and `App::navigate_session_child_first`,
  `navigate_session_parent`, `navigate_session_child_cycle`. Child/sibling selection and cycle
  order replicate the reference (`children()` sorted by id; `moveChild` uses `index - direction`).
  Enablement falls out of the data: a session with no children/parent is a no-op.
- **Family cache.** `App::refresh_session_family` caches a parent's children (and the session's own
  children) so the footer needs no per-frame requests.
- **Subagent footer.** `SessionView::render_subagent_footer` shows the agent label from the
  `@<agent> subagent` title, `(index of total)` by creation order, and `Parent`/`Prev`/`Next`
  shortcuts. It is shown only for child sessions; root sessions keep the footer disabled.
- **View subagents hint.** Assistant messages containing a `task` tool part render
  `ctrl+x down view subagents` under the parts, matching the reference.
- **Dead dialog removed.** `SubagentDialog`/`SubagentInfo`/`SubagentMessage` were never opened or
  constructed and are redundant now that navigation enters real child sessions; the module and all
  wiring were deleted.

### Decisions / deviations

- Background (`ctrl+b`) is intentionally not rendered; `FEAT-048` owns it and the reference gates it
  behind `experimentalBackgroundSubagents`.
- `render_session_footer`'s general (directory/status) row remains disabled for root sessions; only
  the new child-session footer is activated.

### Verification

- `cargo fmt --all`
- `cargo check --workspace`
- `cargo test -p opencode-tui --lib` (134 passed / 0 failed), including new tests:
  `context::keybind::tests::subagent_navigation_defaults_follow_the_reference`,
  `context::keybind::tests::leader_chord_renders_prefixed_shortcut`,
  `app::app::tests::{first_child_is_the_lowest_id, first_sibling_skips_the_current_session,
  cycle_sibling_matches_reference_direction_order, cycle_sibling_is_a_noop_without_siblings}`,
  `components::session::tests::{subagent_label_reads_agent_name_from_title,
  task_hint_uses_leader_chord_and_muted_suffix}`.
- Manual side-by-side (`ort-build` + `ort`) with two subagents is pending human verification.

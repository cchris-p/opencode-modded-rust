---
id: "FEAT-047"
title: "TUI subagent navigation and \"view subagents\""
priority: "P1"
type: "feature"
area: "FEAT"
spec: ""
status: "todo"
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

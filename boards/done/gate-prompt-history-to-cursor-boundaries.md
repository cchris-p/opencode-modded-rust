---
id: "FEAT-037"
title: "Gate prompt history navigation to cursor boundaries"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "done"
created: "2026-09-21"
---

# Gate prompt history navigation to cursor boundaries

## Summary

The prompt input box uses bare `Up`/`Down` to page through previously sent messages, but it does so
unconditionally. `Up`/`Down` should only walk history when the cursor is at the boundary of the
draft — at the very start for `Up` (previous message) and at the very end for `Down` (next message,
when one exists). Everywhere else, the arrows must stay in the editor and move the cursor, mirroring
how vanilla opencode handles it.

This card narrows and makes actionable one specific behavior of the broader prompt-history work in
`FEAT-036`.

## Evaluation of current behavior

History recall already exists and is persisted; it is the *trigger condition* that is wrong.

- Bare `Up`/`Down` are consumed in the prompt key handler and always call history navigation, with no
  cursor check: `crates/opencode-tui/src/components/prompt.rs:531-536`.
- `history_previous` / `history_next` blindly walk the in-memory list and set the cursor to the end of
  the loaded entry: `crates/opencode-tui/src/components/prompt.rs:706-745`.
- The same navigation is also reachable through `Alt+Up` / `Alt+Down`
  (`crates/opencode-tui/src/context/keybind.rs:168-169`) and dispatched at
  `crates/opencode-tui/src/app/app.rs:512-518`. That explicit keybind is the correct "always recall"
  escape hatch; the bare arrows should be the gated ones.
- Consequence in a multi-line draft: the input supports embedded newlines
  (`input_newline` / `input_newline_alt` insert `\n` at
  `crates/opencode-tui/src/app/app.rs:582-590`), but a bare arrow still pages history instead of
  moving the cursor between lines, so editing a multi-line prompt is effectively broken.
- History only contains submitted messages (`take_input` calls `push_history`, then persists to
  `prompt-history.json`: `crates/opencode-tui/src/components/prompt.rs:567-589`,
  `prompt.rs:100-103`). `Ctrl+C`/clear resets the recall cursor but does not add anything
  (`prompt.rs:591-600`). That storage gap is `FEAT-036`'s concern, not this card's.

## Reference behavior (vanilla opencode)

Vanilla opencode's prompt binds history navigation with a cursor guard
(`packages/tui/src/component/prompt/index.tsx`, `prompt.history.previous` / `prompt.history.next` in
the frozen TS reference line):

- Previous: if `input.cursorOffset !== 0`, it first moves the cursor toward the top of the buffer and
  does **not** touch history; only at offset `0` does it load the previous history item.
- Next: if `input.cursorOffset !== plainText.length`, it first moves the cursor toward the bottom and
  does **not** touch history; only at the end does it load the next item.
- After loading an item, the cursor is placed at the corresponding boundary (start for previous, end
  for next).

The Rust fork has no vertical/line cursor movement yet, so the immediate analog is the boundary gate
plus a "send the cursor to the boundary first" step.

## Expected behavior

- `Up`:
  - Cursor at offset `0` (start of draft) → load the previous history entry, cursor lands at start.
  - Cursor anywhere else → do not change history; move the cursor upward within the draft.
- `Down`:
  - Cursor at end of draft (`input.len()`) → load the next history entry if one exists, cursor lands
    at end.
  - Cursor anywhere else → do not change history; move the cursor downward within the draft.
- A single-line draft therefore behaves as: first `Up`/`Down` press snaps the cursor to the
  start/end but does not recall; the next press recalls. This matches vanilla's two-step feel.
- `Alt+Up` / `Alt+Down` remain an unconditional history recall regardless of cursor position.

## Scope

- Add a cursor-boundary gate around bare `Up`/`Down` history navigation in the prompt key handler.
- Implement (or minimally stub) vertical cursor movement so a non-boundary `Up`/`Down` keeps the
  arrows useful inside multi-line drafts rather than doing nothing.
- Keep `Alt+Up` / `Alt+Down` (and the `HistoryPrevious` / `HistoryNext` command actions) as the
  ungated recall path.
- Keep `history_index` / `history_draft` reset semantics correct when the user edits after a recall.
- Add focused tests for the gate: at-start vs mid-draft for `Up`, at-end vs mid-draft for `Down`,
  and single-line snap-then-recall.

## Non-goals

- Changing what is stored, where history is persisted, or how `Ctrl+C`-cleared drafts are captured
  (owned by `FEAT-036`).
- Redesigning the history list, adding a history picker, or changing the frecency/suggestion system.
- Changing `Alt+Up` / `Alt+Down` bindings or the command actions.
- Full multi-line editor work beyond the vertical movement needed for this gate.

## Open questions

- Should a single non-boundary `Up`/`Down` move the cursor by visual wrapped line, logical newline
  line, or just to the buffer boundary? Vanilla moves by visual line; the fork currently renders
  wrapped lines from `wrap_prompt_input`.
- When invoking `Up`/`Down` from a non-boundary position, should the cursor merely snap to the
  boundary (vanilla) or should a second press happen without an intermediate frame? Confirm with a
  real TUI test.
- Does the recall cursor belong at the start (previous) and end (next), matching vanilla, when the
  loaded entry is multi-line?

## Done when

- Bare `Up` only recalls history when the cursor is at offset `0`; bare `Down` only recalls when the
  cursor is at the end of the draft.
- `Up`/`Down` at a non-boundary position never mutate history and keep the draft intact.
- Multi-line drafts can be navigated with the arrows without losing the draft to a history load.
- `Alt+Up`/`Alt+Down` still recall unconditionally.
- Unit tests cover the gate and the snap-then-recall sequence.
- `cargo test -p opencode-tui` passes for the new tests (existing unrelated failures excepted).

## Recommended verification

- `ort-build`, then `ort`; type a prompt and confirm `Up` at the start recalls the previous message
  while `Up` mid-draft does not.
- Type a multi-line draft (`Ctrl+J` newlines) and confirm arrows move within it instead of paging
  history.
- Confirm `Down` at the end returns to the draft that was being edited before recall.
- Confirm `Alt+Up`/`Alt+Down` still page history from any cursor position.
- `cargo test -p opencode-tui --lib prompt -- --test-threads=1`.

## Related Items

- `FEAT-036` Persist and recall typed input-box messages after send or clear - the parent
  "historical message" feature; this card is the actionable slice that fixes the recall trigger
  condition (and its open question "up/down arrow history").
- `FEAT-006` Make Ctrl+D exit the TUI and Ctrl+C clear the prompt - same prompt-input key handling
  and history-on-clear intersection.
- `BUG-018` Prompt input cursor renders out of place - same prompt cursor/geometry surface; keep the
  vertical-movement implementation consistent with `wrap_prompt_input`.
- `BUG-013` Cursor on the input field needs to always be visible - cursor behavior in the prompt.
- `FEAT-004` Add in-session send-to-fork commands - reuses prompt-box contents and history.
- `PHASE-001` V1 daily-driver hardening.

## Notes

- Current behavior summary: bare arrows page history at any cursor position
  (`crates/opencode-tui/src/components/prompt.rs:531-536`); `Alt+Up`/`Alt+Down` are the registered
  history keybinds (`crates/opencode-tui/src/context/keybind.rs:168-169`).
- Vanilla reference: `packages/tui/src/component/prompt/index.tsx`, `prompt.history.previous` /
  `prompt.history.next` commands guard on `cursorOffset === 0` / `cursorOffset === plainText.length`
  in the frozen TS reference line.
- Keep the first implementation small: gate the bare arrows, preserve `Alt+*` recall, and add the
  minimum vertical movement needed so non-boundary arrows are not dead keys.

## Completion

- 2026-09-21: Implementation shipped in commit `05f1c1e` and merged into `development` via PR #70
  (merge commit `e0a059f69fee9ebdcc27fbf4d12c0f8a89fdb1b7`). The card had remained in `todo` after the
  merge; this closeout moves it to `done`.
- Delivered: bare `Up`/`Down` are gated on cursor boundaries, `move_cursor_vertical` navigates
  multi-line drafts, `Alt+Up`/`Alt+Down` remain the ungated recall path, and unit tests cover the gate
  and the snap-then-recall sequence.
- Verification: `cargo test -p opencode-tui --lib` (68 pass), including
  `bare_up_recalls_history_only_at_cursor_start`,
  `bare_down_recalls_history_only_at_cursor_end`,
  `bare_arrows_navigate_multiline_draft_without_recall`, and
  `explicit_history_navigation_is_ungated`. The `Alt+Up`/`Alt+Down` path is unchanged.
- The BUG-030 history-next fix (`ac394d8`) coexists in `prompt.rs` after the merge; verified both
  behaviors are present. Live TUI acceptance was not independently run; behavior is covered by the
  unit tests above.


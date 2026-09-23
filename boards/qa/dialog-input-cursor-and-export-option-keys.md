---
id: "BUG-031"
title: "Dialog text inputs lack cursor navigation and the export option toggles capture digits"
priority: "P2"
type: "bug"
area: "BUG"
spec: ""
status: "qa"
created: "2026-09-21"
updated: "2026-09-23"
---

# Dialog text inputs lack cursor navigation and the export option toggles capture digits

## Summary

Every dialog title/filename text field in the TUI is append-only / pop-only: it has no caret, so
`Left`/`Right`/`Home`/`End`/`Delete` do nothing, typing always appends, and `Backspace` only removes
the last character. Editing the middle of an existing value is impossible without deleting the tail.
This affects all three editable rename/filename fields:

1. The session rename dialog input (`SessionRenameDialog`).
2. The inline rename field in the sessions list (`SessionListDialog::rename_input`).
3. The export dialog filename/path field (`SessionExportDialog::filename`), which additionally
   cannot accept the digits `1`, `2`, or `3` because those keys are hard-wired to the three
   Include-options toggles.

Both defects share one root cause: these fields store a plain `String` with append/pop-only handlers
instead of a cursor-aware edit buffer like the main prompt already uses. The export dialog
additionally overloads bare digits as option mnemonics.

## Reported behavior

- In any rename field (session rename dialog and sessions-list inline rename), `Left`/`Right` do not
  move a caret; only deleting from the end works.
- In the export dialog, pressing `1`, `2`, or `3` toggles an option instead of typing the digit, so
  numeric filenames and paths are impossible.
- The export dialog filename field also has no cursor movement.

## Code evidence

Rename dialog:

- `SessionRenameDialog` holds only `open`, `session_id`, and `input: String` — no cursor field:
  `crates/opencode-tui/src/components/dialogs/session_rename.rs:11-15`.
- `handle_input` appends and `handle_backspace` pops the tail only:
  `crates/opencode-tui/src/components/dialogs/session_rename.rs:42-48`.
- Render always draws the `▏` marker after the whole string, not at a caret position:
  `crates/opencode-tui/src/components/dialogs/session_rename.rs:90-97`.
- Key dispatch handles only `Esc`/`Backspace`/`Enter`/plain `Char`; `Left`/`Right`/`Home`/`End`/
  `Delete` fall through to `_ => {}`:
  `crates/opencode-tui/src/app/app.rs:1192-1223`.

Sessions-list inline rename:

- `SessionListDialog` holds `rename_session_id` plus `rename_input: String` — no cursor field:
  `crates/opencode-tui/src/components/dialogs/session_list.rs:36-37`.
- `start_rename_selected` seeds `rename_input` from the session title with no caret:
  `crates/opencode-tui/src/components/dialogs/session_list.rs:154-169`.
- `handle_rename_input` appends and `handle_rename_backspace` pops the tail only:
  `crates/opencode-tui/src/components/dialogs/session_list.rs:176-182`.
- Render draws the `▏` marker after the whole string:
  `crates/opencode-tui/src/components/dialogs/session_list.rs:331-336`.
- Key dispatch while renaming handles only `Esc`/`Backspace`/`Enter`/plain `Char`;
  `Left`/`Right`/`Home`/`End`/`Delete` fall through:
  `crates/opencode-tui/src/app/app.rs:1449-1483`.

Export dialog:

- `SessionExportDialog` holds `filename: String` with no cursor:
  `crates/opencode-tui/src/components/dialogs/session_export.rs:11-18`.
- `handle_input` maps `'1'`/`'2'`/`'3'` to option toggles and only pushes all other chars:
  `crates/opencode-tui/src/components/dialogs/session_export.rs:48-55`; `handle_backspace` pops the
  tail (`:57-59`).
- Render draws the `▏` marker after the whole string (`:109-116`) and the hint line advertises
  `1/2/3 toggle options` (`:149-155`).
- Key dispatch again has no cursor keys:
  `crates/opencode-tui/src/app/app.rs:1225-1290`.

Existing correct pattern to reuse:

- The main prompt implements a cursor-aware single/multi-line input: `cursor_position`
  (`crates/opencode-tui/src/components/prompt.rs:155`), insert-at-caret (`prompt.rs:545-546`),
  `Backspace` (`:555-557`), `Delete` (`:562-566`), `Left`/`Right` (`:568-582`), `Home`/`End`
  (`:583-587`), `Alt+Left`/`Alt+Right` word skipping (`:569-581`, also `Alt+b`/`Alt+f` at
  `:521-534`), and the boundary helpers `prev_char_boundary`/`next_char_boundary`
  (`prompt.rs:1596-1615`) plus `prev_word_boundary`/`next_word_boundary` (`prompt.rs:1617-1650`).
- General rule these fields violate: `invariants/option-selection.md:23-25` ("A text-entry field
  must be caret-editable: `Left`/`Right` move by one character, `Home`/`End` move to the boundaries,
  typed characters insert at the caret, and `Backspace`/`Delete` remove around the caret.
  Append-only and pop-only editing is a defect.").

## Expected behavior

- All three fields support caret movement: `Left`/`Right` move by one character, `Home`/`End` jump
  to start/end, `Backspace`/`Delete` delete around the caret, and typing inserts at the caret.
- `Alt+Left`/`Alt+Right` skip by word in all three fields, matching the main prompt and any other
  input field that skips words (`prev_word_boundary`/`next_word_boundary` semantics: skip
  non-word characters, then a run of word characters).
- The visible `▏` caret renders at the caret offset, not always at the end.
- The export filename/path field accepts every printable digit, including `1`, `2`, and `3`.
- The three export options (Include thinking / tool details / assistant metadata) remain togglable
  through focus selection (`Tab`/`Shift+Tab` + `Space`).
- Existing behavior (rename persistence, path resolution, directory creation, confirmation display,
  `Ctrl+C` copy transcript, empty-title/filename validation) is unchanged.

## Locked decisions (refined 2026-09-23)

These resolve the previously open questions. They follow `invariants/option-selection.md` and the
existing prompt cursor model; do not re-litigate during implementation.

- **Shared buffer, not a second editing model.** Introduce one small shared cursor-aware struct and
  use it for all three fields. Do not copy the prompt's inline cursor logic three times.
- **Forward `Delete` is supported** in all three fields, alongside `Backspace`. The invariant
  mandates both (`invariants/option-selection.md:23-25`).
- **`Alt+Left`/`Alt+Right` word skipping is required** in all three fields and must reuse the
  prompt's `prev_word_boundary`/`next_word_boundary` (and `is_word_char`) so behavior matches
  exactly. When the export filename field is not focused, `Alt+Left`/`Alt+Right` move option focus
  like plain `Left`/`Right`.
- **`Enter` exports unconditionally** in the export dialog, regardless of which row has focus.
  `Enter` is the submit action; `Space` is the option activation key.
- **Export option persistence is unchanged.** The dialog is created once in `App::new` and
  `open()` does not touch the option flags, so options already persist across opens today; this card
  keeps that behavior and neither adds nor removes persistence.
- **Export focus selection method (locked).** The options move to focus selection, chosen over
  modified numeric mnemonics.
  - `Tab`/`Shift+Tab` cycles focus across the filename field and the three option rows (wrapping).
  - `Space` toggles the focused option; when the filename field is focused, `Space` types a space.
  - `Left`/`Right` move the caret when the filename field is focused, and move the option focus
    otherwise. `Home`/`End` affect the filename caret only.
  - `Up`/`Down` also move option focus.
  - Bare digits, including `1`, `2`, and `3`, become ordinary typeable characters.
  - The dialog hint line must be updated to state these real keys.
  - Rejected alternative: modified numeric mnemonics (`Ctrl+1`/`Ctrl+2`/`Ctrl+3`). `Alt`+digit is
    disallowed on macOS (Option+digit emits characters), and a modifier requirement is less
    discoverable than focus selection.

## Implementation plan

1. Add a shared cursor-aware input buffer, e.g.
   `crates/opencode-tui/src/components/dialogs/text_input.rs` (declared in `dialogs/mod.rs`):
   - State: `value: String`, `cursor: usize` (byte offset, always on a char boundary, clamped to
     `value.len()`).
   - API: `new()`, `set(String)` (resets caret to end), `clear()`, `value() -> &str`,
     `cursor() -> usize`, `split_at_cursor() -> (&str, &str)`, `insert_char(char)`,
     `backspace()`, `delete()`, `move_left()`, `move_right()`, `move_word_left()`,
     `move_word_right()`, `home()`, `end()`.
   - Reuse the prompt's boundary helpers by promoting `prev_char_boundary`/`next_char_boundary`
     (`crates/opencode-tui/src/components/prompt.rs:1596-1615`) and
     `prev_word_boundary`/`next_word_boundary` (plus `is_word_char`, `prompt.rs:1617-1650`) to
     `pub(crate)` and importing them, rather than defining new boundary math. Keep multibyte-safe
     behavior (CJK, combining marks).
2. Replace the plain `String` fields with the shared buffer:
   - `SessionRenameDialog.input` → buffer; `open()` uses `set(title)`; `confirm()` reads
     `value().trim()` and clears as today.
   - `SessionListDialog.rename_input` → buffer; `start_rename_selected()` uses `set(title)`;
     `confirm_rename()`/`cancel_rename()` read/clear as today.
   - `SessionExportDialog.filename` → buffer; `open()` uses `set(default_filename)`; `filename()`
     returns `value()`.
3. Add an export focus enum, e.g. `ExportFocus { Filename, Thinking, ToolDetails, Metadata }`
   defaulting to `Filename`, plus `cycle_focus(forward: bool)` and a `toggle_focused_option()`.
4. Wire keys in `App::handle_dialog_key` (`crates/opencode-tui/src/app/app.rs`):
   - Rename dialog (`:1192-1223`) and sessions-list rename (`:1449-1483`): handle
     `Left`/`Right`/`Home`/`End`/`Delete` plus `Alt+Left`/`Alt+Right` (word skip) via the buffer;
     keep `Esc`/`Backspace`/`Enter`/`Char`.
   - Export dialog (`:1225-1290`): `Tab`/`Shift+Tab` cycle focus; `Space` toggles focused option or
     types a space; `Left`/`Right`/`Home`/`End`/`Backspace`/`Delete` (and `Alt+Left`/`Alt+Right`
     word skip) drive the filename caret when `Filename` is focused and move focus otherwise; digits
     are ordinary chars; `Enter` still exports; `Ctrl+C` copy transcript unchanged.
5. Update render for all three fields to draw the caret at the cursor offset using
   `split_at_cursor`: `before`, `▏`, `after` (matching the existing `▏` glyph and color).
6. Update the export dialog hint line (`session_export.rs:149-155`) and the sessions-list rename
   footer (`session_list.rs:347-353`) to state the real keys (`Tab`/`Shift+Tab` focus, `Space`
   toggle, arrows edit/move, `Enter` save/export, `Esc` cancel).

## Scope

- Introduce the shared cursor-aware buffer and adopt it in `SessionRenameDialog`,
  `SessionListDialog::rename_input`, and `SessionExportDialog::filename`.
- Add caret and word-skip key handling (`Left`/`Right`/`Alt+Left`/`Alt+Right`/`Home`/`End`/
  `Delete`) and caret-offset rendering in the three fields.
- Rebind the export option toggles to focus selection and update dialog hint text.
- Keep `Enter`, `Esc`, empty-value validation, and the existing `Ctrl+C` copy-transcript behavior.
- Add focused unit tests for buffer editing (including multibyte) and for the export digit/focus
  behavior.

## Non-goals

- Redesigning the rename or export workflow, naming convention, or save location.
- Persisting export options between opens.
- Full multi-line editor features (selection, word-delete, undo) in dialogs. Word *movement*
  (`Alt+Left`/`Alt+Right`) is in scope; word *deletion* (`Alt+Backspace`/`Alt+Delete`) is not.
- Changing the main prompt input, which already has correct cursor behavior.
- Changing what a rename or export writes or how it is persisted.
- Fixing the list/filter search fields (command palette, model select, skill/theme list, prompt
  stash, sessions-list filter). They have the same append-only defect but are a separate follow-up;
  if the shared buffer lands as planned, adopting it there later is a small, mechanical change.

## Acceptance criteria

- In the session rename dialog and the sessions-list inline rename, `Left`/`Right` move the caret,
  `Home`/`End` jump, `Delete`/`Backspace` delete around the caret, and typing inserts at the caret;
  a title can be edited in the middle without deleting the tail, and the saved title matches.
- The caret `▏` is rendered at the caret offset in all three fields.
- `Alt+Left`/`Alt+Right` skip by word in all three fields, matching the main prompt's
  `prev_word_boundary`/`next_word_boundary` behavior (including punctuation and multibyte text).
- In the export dialog, the filename/path field has the same caret behavior and accepts `1`, `2`,
  and `3` as ordinary characters (typed and in the default filename).
- The three export options are still togglable via focus selection (`Tab`/`Shift+Tab` + `Space`),
  and toggling them still changes the exported transcript as before.
- `cargo check -p opencode-tui` and `cargo test -p opencode-tui` pass, including the new tests.

## Done when

- All acceptance criteria above are met.
- The shared buffer is unit-tested for insert/backspace/delete/move/home/end/word-move at middle
  offsets and across multibyte text.
- `cargo test -p opencode-tui` passes.

## Recommended verification

- `ort-build`, then `ort`.
- Rename the active session with `Ctrl+R`; place the caret mid-title with `Left`, insert and delete
  in the middle, then confirm the final title is what was typed.
- Open the sessions list, start inline rename, and repeat the mid-title edit; open a session and
  confirm the persisted title matches.
- In a rename field and the export filename, use `Alt+Left`/`Alt+Right` to jump across words and
  confirm the landing offsets match the main prompt for the same text and starting caret.
- Export with a filename containing digits and a directory path containing digits, for example
  `exports/2026/run-1.md`; confirm the digit keys type into the field, the file is created at the
  expected path, and the confirmation shows that path.
- In the export dialog, `Tab`/`Shift+Tab` to focus each option and `Space` to toggle; confirm the
  transcript includes or omits thinking/tool-details/metadata accordingly, and `Enter` still exports
  from any focus position.
- Repeat the export from a workspace root and from a subdirectory typed into the field.
- `cargo test -p opencode-tui`.

## Related Items

- `BUG-017` Export confirmation should show saved file path — closed; the filepath display works.
  This card is the follow-up defect found while retesting it.
- `BUG-018` Prompt input cursor renders out of place — main prompt cursor rendering; keep the fix
  consistent with the existing prompt cursor model.
- `BUG-013` Cursor on the input field needs to always be visible.
- `FEAT-029` Learn how manual session rename works end to end — rename flow reference, including the
  sessions-list rename entry point.
- `FEAT-025` Map Ctrl+R to rename — the rename entry point the session rename dialog serves.
- `invariants/option-selection.md` — the caret-editability and focus-selection rules this card
  enforces.

## Notes

- Prefer reusing the prompt's cursor approach (`cursor_position` + `prev_char_boundary` /
  `next_char_boundary`) rather than inventing a second editing model.
- The current export `handle_input` is the only place that swallows digits; digit handling must move
  entirely to the filename buffer.
- The export option state fields are public (`include_thinking`, `include_tool_details`,
  `include_metadata`) and read in `transcript_options_from_export_dialog`
  (`crates/opencode-tui/src/app/app.rs:2286`); only the key handling and focus state need to change.
- Keep the `▏` glyph, color, and single-line layout; this is a caret-position fix, not a cursor
  rendering redesign.
- Match the prompt's word-skip semantics exactly by importing `prev_word_boundary` /
  `next_word_boundary` / `is_word_char` rather than reimplementing them; differing word rules between
  the prompt and dialogs would be a new inconsistency.

## Implementation Notes

- Added a shared cursor-aware single-line buffer, `DialogTextInput`
  (`crates/opencode-tui/src/components/dialogs/text_input.rs`): caret-aware insert/backspace/delete,
  character and word movement, `Home`/`End`, and `split_at_cursor` for caret rendering.
- Reused the prompt's boundary helpers instead of reimplementing them: `prev_char_boundary`,
  `next_char_boundary`, `prev_word_boundary`, and `next_word_boundary` were promoted to `pub(crate)`
  in `crates/opencode-tui/src/components/prompt.rs` and imported by the buffer.
- Adopted the buffer in all three fields: `SessionRenameDialog::input`,
  `SessionListDialog::rename_input`, and `SessionExportDialog::filename`. Each now renders the `▏`
  caret at the caret offset (splitting the value around the cursor) instead of always at the end.
- Wired caret keys in `App::handle_dialog_key` for the session rename dialog, sessions-list inline
  rename, and export filename: `Left`/`Right` by character, `Alt+Left`/`Alt+Right` (and `Alt+b`/
  `Alt+f`) by word, `Home`/`End`, and forward `Delete`, matching the main prompt.
- Export options moved off bare-digit mnemonics to focus selection: `Tab`/`Shift+Tab` cycle focus
  across the filename field and the three option rows, `Space` toggles the focused option (and types
  a space in the filename field), `Up`/`Down` move focus, and `Enter` exports from any focus
  position. Digits `1`/`2`/`3` now type into the filename. The option rows render a `>` focus marker
  instead of the old `1`/`2`/`3` labels, and the hint line was updated.
- Option persistence across opens was left exactly as it was (the dialog is created once in
  `App::new`, so options persist; `open()` does not reset them). The card's earlier wording about
  resetting options was corrected to match the real behavior.
- Added unit tests: buffer editing (character/word movement, backspace/delete around the caret,
  multibyte safety) and export behavior (digits type, caret edit, Alt word skip, `Space` toggle vs.
  space typing, `Tab`/arrow focus movement, chars returning focus to the filename).
- Verification run: `cargo check -p opencode-tui` (no warnings) and `cargo test -p opencode-tui`
  (94 passed, 0 failed).

### PR Link

- (pending)


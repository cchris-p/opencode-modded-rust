---
id: "BUG-029"
title: "Dialog text inputs lack cursor navigation and the export option toggles capture digits"
priority: "P2"
type: "bug"
area: "BUG"
spec: ""
status: "todo"
created: "2026-09-21"
---

# Dialog text inputs lack cursor navigation and the export option toggles capture digits

## Summary

Two related defects in the rename and export dialog text inputs make them hard to edit:

1. The rename input has no cursor. `Left`/`Right` (and `Home`/`End`) do nothing, typing always
   appends to the end, and `Backspace` only deletes the last character. Editing the middle of an
   existing title is impossible without deleting everything after the edit point.
2. The export filename/path field cannot accept the digits `1`, `2`, or `3`, because those keys are
   hard-wired to the three Include-options toggles. A user cannot name a file or path containing
   those digits (for example `run-1.md`, `exports/2026/...`, or any `/tmp/...` path segment).

Both stem from the same root cause: these dialogs store their text as a plain `String` with
append/pop-only handlers instead of a cursor-aware input buffer like the main prompt already uses.
The export dialog additionally overloads bare digits as option mnemonics.

## Reported behavior

- In the rename dialog, `Left`/`Right` do not move a caret; only deleting from the end works.
- In the export dialog, pressing `1`, `2`, or `3` toggles an option instead of typing the digit, so
  numeric filenames and paths are impossible.
- The export dialog filename field also has no cursor movement.

## Code evidence

Rename dialog:

- `SessionRenameDialog` holds only `open`, `session_id`, and `input: String` — no cursor field:
  `crates/opencode-tui/src/components/dialogs/session_rename.rs:11-15`.
- `handle_input` appends and `handle_backspace` pops the tail only:
  `crates/opencode-tui/src/components/dialogs/session_rename.rs:42-48`.
- Key dispatch handles only `Esc`/`Backspace`/`Enter`/plain `Char`; `Left`/`Right`/`Home`/`End` fall
  through to `_ => {}`:
  `crates/opencode-tui/src/app/app.rs:1141-1172`.

Export dialog:

- `SessionExportDialog` holds `filename: String` with no cursor:
  `crates/opencode-tui/src/components/dialogs/session_export.rs:11-18`.
- `handle_input` maps `'1'`/`'2'`/`'3'` to option toggles and only pushes all other chars:
  `crates/opencode-tui/src/components/dialogs/session_export.rs:48-55`; `handle_backspace` pops the
  tail (`:57-59`).
- The hint line advertises `1/2/3 toggle options`:
  `crates/opencode-tui/src/components/dialogs/session_export.rs:149-155`.
- Key dispatch again has no cursor keys:
  `crates/opencode-tui/src/app/app.rs:1174-1242`.

Existing correct pattern to reuse:

- The main prompt already implements a cursor-aware single/multi-line input: `cursor_position`
  (`crates/opencode-tui/src/components/prompt.rs:75`), insert-at-caret
  (`prompt.rs:466-469`), `Left`/`Right`/`Home`/`End` (`prompt.rs:491-511`), and char-boundary helpers
  `prev_char_boundary`/`next_char_boundary`.

## Expected behavior

- Both dialog text fields support caret movement: `Left`/`Right` move by one character, `Home`/`End`
  jump to start/end, and `Backspace`/`Delete` delete around the caret. Typing inserts at the caret.
- The export filename/path field accepts every printable digit, including `1`, `2`, and `3`.
- The three export options (Include thinking / tool details / assistant metadata) remain togglable
  through focus selection (`Tab`/`Shift+Tab` + `Space`).
- Existing export behavior (path resolution, directory creation, confirmation display) is unchanged.

## Decision: option-selection method (locked)

The export dialog's options move to **focus selection**, chosen over modified numeric mnemonics.

- `Tab`/`Shift+Tab` cycles focus across the filename field and the three option rows.
- `Space` toggles the focused option.
- `Enter` still exports.
- Bare digits, including `1`, `2`, and `3`, become ordinary typeable characters in the filename
  field.
- The dialog hint line must be updated to state these real keys.

Rejected alternative: modified numeric mnemonics (`Ctrl+1`/`Ctrl+2`/`Ctrl+3`). `Alt`+digit is
disallowed on macOS (Option+digit emits characters), and a modifier requirement is less discoverable
than focus selection.

See `invariants/option-selection.md` for the general rules this decision follows.

## Scope

- Add a cursor-aware edit buffer to `SessionRenameDialog` (either a shared small struct or the same
  approach as the prompt) and wire `Left`/`Right`/`Home`/`End`/`Delete` in `app.rs`.
- Add the same cursor-aware editing to the `SessionExportDialog` filename field.
- Rebind the export option toggles to focus selection and update the dialog hint text.
- Keep `Enter`, `Esc`, and the existing `Ctrl+C` copy-transcript behavior intact.
- Add focused unit tests for caret movement and for typing digits into the export filename.

## Non-goals

- Redesigning the export workflow, naming convention, or save location.
- Full multi-line editor features (selection, word-delete, undo) in the dialogs.
- Changing the main prompt input, which already has correct cursor behavior.
- Changing what a rename or export writes or how it is persisted.

## Done when

- In the rename dialog, `Left`/`Right` move the caret, `Home`/`End` jump, and typing inserts at the
  caret; a title can be edited in the middle without deleting the tail.
- In the export dialog, the filename/path field has the same caret behavior and accepts `1`, `2`, and
  `3` as ordinary characters.
- The three export options are still togglable via focus selection (`Tab`/`Shift+Tab` + `Space`),
  and toggling them still changes the exported transcript as before.
- `cargo check -p opencode-tui` and `cargo test -p opencode-tui` pass for the new tests.

## Recommended verification

- `ort-build`, then `ort`.
- Rename the active session with `Ctrl+R`; place the caret mid-title with `Left`, insert and delete
  in the middle, and confirm the final title is what was typed.
- Export with a filename containing digits and a directory path containing digits, for example
  `exports/2026/run-1.md`; confirm the digit keys type into the field, the file is created at the
  expected path, and the confirmation shows that path.
- Toggle all three export options with the alternative control and confirm the transcript includes
  or omits thinking/tool-details/metadata accordingly.
- Repeat the export from a workspace root and from a subdirectory typed into the field.
- `cargo test -p opencode-tui`.

## Open questions

- Should `Delete` (forward delete) be supported alongside `Backspace`, or is `Backspace` sufficient?
- When the focus model is used, does `Enter` always export, or only when the filename field is
  focused?
- Should the export dialog remember the last-used options between opens, or reset each time (current
  behavior resets via `SessionExportDialog::new`)?

## Related Items

- `BUG-017` Export confirmation should show saved file path — closed; the filepath display works.
  This card is the follow-up defect found while retesting it.
- `BUG-018` Prompt input cursor renders out of place — main prompt cursor rendering; keep the fix
  consistent with the existing prompt cursor model.
- `BUG-013` Cursor on the input field needs to always be visible.
- `FEAT-029` Learn how manual session rename works end to end — rename flow reference.
- `FEAT-025` Map Ctrl+R to rename — the rename entry point this dialog serves.

## Notes

- Prefer reusing the prompt's cursor approach (`cursor_position` + `prev_char_boundary` /
  `next_char_boundary`) rather than inventing a second editing model.
- `SessionRenameDialog::handle_input` currently pushes raw chars; `SessionExportDialog::handle_input`
  is the one that must stop swallowing digits.
- The export option state fields are public (`include_thinking`, `include_tool_details`,
  `include_metadata`) and read in `transcript_options_from_export_dialog`
  (`crates/opencode-tui/src/app/app.rs:2232-2237`); only the key handling needs to change.
EOF

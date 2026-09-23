---
id: "BUG-039"
title: "Question prompt custom-answer layout clips the typed answer and does not wrap"
priority: "P2"
type: "bug"
area: "BUG"
spec: "invariants/option-selection.md"
status: "todo"
created: "2026-09-23"
updated: "2026-09-23"
---

# Question prompt custom-answer layout clips the typed answer and does not wrap

## Summary

When the `question` tool asks a question and the user picks the **"Type your own answer"** row (or
answers a free-text question), the inline question prompt renders with a broken layout. The
custom-answer input line, the submit hint, wrapping, and paste handling are all wrong, so the typed
answer can be partly or completely invisible, long text is truncated instead of wrapped, and pasted
content lands in the wrong place with the wrong background.

Reported: "When prompted for a single question (and likely multiple questions) and when I use the
type answer option, the layout looks so weird."

## Reported behavior

- Choosing the custom/typed-answer row makes the question box layout look wrong.
- The typed answer is not clearly presented; it can be clipped off the bottom of the box on a short
  terminal and there is no visible caret while typing.
- Long question text and option descriptions are cut off at the right edge rather than wrapping.
- Reported for both single-question and multi-question requests.
- Pasting while the custom-answer field is active does not lay out correctly: the pasted text is
  vertically contained but **not horizontally contained** (long content is clipped at the right
  border instead of wrapping), and the pasted text renders with a **different background color** than
  the question box.

## Reproduction

1. Trigger any question that has options and `custom` enabled (the tool default), e.g. a single
   question with two options.
2. Navigate to `Type your own answer` with `Up`/`Down` and press `Enter` (or press the number key for
   the custom row).
3. Type or paste an answer.
4. Observe:
   - On a short terminal the option list pushes the `>` input line and the
     `Enter to submit, Esc to cancel` hint below the box border, so they disappear.
   - Long question/description/answer lines are truncated at the right border, not wrapped.
   - There is no caret; the only feedback is the appended `>` line.
   - The custom row still shows `[ ]` even though the prompt is now in text-entry mode.
   - Pasted text (native bracketed paste) does not appear in the answer field with the box's
     background; it can appear on a different background instead.

Renderer behavior was reproduced with a temporary `TestBackend` harness (removed); no code changes
are included in this card.

## Investigation notes / code evidence

Rendering lives in `crates/opencode-tui/src/components/question.rs`.

1. **No wrapping.** The prompt is a single `Paragraph::new(content)` with no `.wrap(...)`
   (`question.rs:494-503`). Ratatui's `Paragraph` defaults to `wrap: None`
   (`~/.cargo/registry/.../ratatui-0.27.0/src/widgets/paragraph.rs:120`), so every line is clipped at
   the box width. Long question text (`question.rs:374-379`), descriptions (`:425-430`), and the
   typed answer line (`:453-462`) are truncated, e.g. `...exceed the eighty c` and `...that als`.

2. **Height capped with no scroll, so the active input can be pushed out of the box.** Box height is
   computed from raw content length and capped at `area.height - 2` (`question.rs:475`), and the popup
   is bottom-anchored (`:479-484`). With more content than fits, `Paragraph` renders from the top and
   drops the tail. On a 60x14 terminal with four options + descriptions + custom row, the
   `Type your own answer` row, the `>` input line, and the hint are all clipped off entirely — the
   user cannot see what they type.

3. **The custom-answer input is append-only with no caret.** `type_char` pushes to a `String` and
   `backspace` pops the tail (`question.rs:204-232`); the renderer just prints `> {text}`
   (`:453-462`). There is no cursor offset, no `Left`/`Right`/`Home`/`End`/`Delete`, and no caret
   glyph. `invariants/option-selection.md:23-25` requires text-entry fields to be caret-editable, and
   the main prompt already provides the model (`components/prompt.rs`). This is the same class of
   defect as `BUG-031`.

4. **Custom row state is inconsistent.** `confirm()` enters text mode when the custom row is focused
   but does not set `custom_selected` (`question.rs:272-276`), so the row renders `[ ]` while the
   prompt is in text-entry mode (`:433-450`). Selecting the row via a digit sets it, so the marker
   depends on how the user reached the row.

5. **Box is left-anchored and capped at 80 columns.** `width = area.width.saturating_sub(2).min(80)`
   and `popup_area.x = area.x + 1` (`question.rs:476-484`). On wide terminals the box is a
   left-aligned 80-column panel with dead space to the right, which reads as misaligned. This is
   cosmetic and overlaps with the deliberate "compact bottom prompt" deviation in `GATE-002`.

6. **Modal backdrop blanks the transcript.** `show_modal_overlay` includes `question_prompt.is_open`
   (`crates/opencode-tui/src/app/app.rs:4032-4034`) and paints the full frame with
   `theme.background_menu` before the prompt renders (`app.rs:4092-4096`, prompt at `:4118`). So the
   question box floats on an otherwise empty screen; this is true for every question mode, but it
   amplifies how "off" the custom-answer state looks because there is no surrounding context.

7. **Paste is never routed to the question prompt (background mismatch).** `Event::Paste` is handled
   at the top level and unconditionally calls `self.prompt.insert_text(text)` — the session prompt —
   with no check for `self.question_prompt.is_open` (`crates/opencode-tui/src/app/app.rs:801-805`).
   The question-open key branch returns early for every key (`app.rs:406-427`), so `Ctrl+V`
   (`input_paste`, `app.rs:557-558`) is also swallowed while the question is open. Native bracketed
   paste therefore lands in the session prompt behind the modal instead of the custom-answer field.
   The session prompt paints `theme.background_element` (`components/prompt.rs:369,397,455,459`) while
   the question box paints `theme.background_panel` (`question.rs:501`); those are distinct theme
   tokens (`theme/mod.rs:71-72`, built at `:325-328`), which is the "different background color" the
   reporter sees. A paste handler that routes to the question input when it is open is required.

8. **The question input line does not wrap and embedded newlines collapse.** Even when text reaches
   the question input (e.g. delivered as character events), it is rendered as one `> {text}` span
   (`question.rs:453-462`) inside a `Paragraph` with no `.wrap(...)` (`:494-503`). A pasted multi-line
   string has its newlines dropped and is shown as one concatenated line that is clipped at the right
   border. `TestBackend` reproduction: `line one ...\nline two\nline three` renders as
   `> line one is long and wraps in a prompt but not hereline twoline three` on a single clipped row.
   This is the "vertically contained but not horizontally contained" symptom.

Existing correct patterns to reuse:

- `Paragraph::wrap(Wrap { trim: false })`, `Paragraph::line_count(width)`, and
  `Paragraph::scroll((row, col))` are all available in ratatui 0.27
  (`~/.cargo/registry/.../ratatui-0.27.0/src/widgets/paragraph.rs:172,189,277`).
- The main prompt already wraps and scrolls multi-line input (`components/prompt.rs:312-323,377-399`)
  and owns the cursor/boundary helpers `prev_char_boundary`, `next_char_boundary`,
  `prev_word_boundary`, `next_word_boundary` (`prompt.rs:1596-1652`).
- The dialog cursor buffer `DialogTextInput`
  (`crates/opencode-tui/src/components/dialogs/text_input.rs`) is exactly the single-line caret
  model this input needs and is already used for the `BUG-031` fields.

## Locked decisions (2026-09-23)

Locked for implementation. Do not re-litigate during the build.

- **Implement A + C + E + F.** Wrap/layout, caret-aware input, custom-row marker, and paste routing
  are all in scope.
- **Reject Option B (collapse the option list while typing).** Keep the option list visible in text
  mode. The compact bottom prompt with the list intact stays; this preserves the current interaction
  model and avoids a new `GATE-002` deviation.
- **Defer Option D (box geometry) as optional polish, out of scope here.** Do not change the bottom
  anchor or the 80-column cap in this card; file a follow-up if the un-centered look still bothers.
- **Wrapping.** Use `.wrap(Wrap { trim: false })` and size the box from `Paragraph::line_count`
  measured at the inner content width, not from the raw content-line count.
- **Never hide the active input.** When wrapped content exceeds the available height, scroll the
  paragraph to the tail so the `>` input line and the hint stay visible; the option list scrolls
  rather than clipping the input.
- **Caret input.** Replace the append-only `String` with `DialogTextInput`; render a `▏` caret at the
  cursor via `split_at_cursor`; support `Left`/`Right`/`Home`/`End`/`Delete` and `Alt+Left`/`Alt+Right`
  word skipping; typing inserts at the caret. This satisfies `invariants/option-selection.md:23-25`.
- **Paste.** Route both bracketed paste (`Event::Paste`) and `Ctrl+V` to the question input when the
  prompt is open, and normalize the pasted text to a single line (CR/LF/tab to spaces) so it is a
  valid one-shot answer. The session prompt keeps its current paste behavior when no question is open.
- **Single-line answers.** `Enter` submits; newlines never become part of the answer.
- **Descriptions wrap; no ellipsis.** Long question text and option descriptions wrap inside the box.
- **Review screen shares the renderer.** The wrapping/height/scroll fix applies to the
  `QuestionType::Review` screen too; do not change its flow semantics.
- **No protocol/schema changes.** This is a TUI-only bug.

Resolved open questions:

- Primary trigger is horizontal clipping plus paste misrouting; the un-centered box is secondary and
  deferred (Option D).
- The custom-answer input **is** subject to the caret-editability invariant; it must be
  caret-editable with a visible caret.
- Long descriptions wrap, not ellipsize.

## Implementation plan

1. **Reuse the cursor buffer.**
   - Re-export `DialogTextInput` from `crates/opencode-tui/src/components/dialogs/mod.rs`
     (`pub use text_input::DialogTextInput;`) so `question.rs` can import
     `super::dialogs::DialogTextInput`.
   - Add `DialogTextInput::insert_str(&mut self, text: &str)` that normalizes `\r\n`/`\r`/`\n`/`\t`
     to a single space, inserts at the caret, and advances the caret. Unit-test it (including
     multibyte and newline flattening).
2. **Convert the question input to the buffer** (`crates/opencode-tui/src/components/question.rs`).
   - Change `QuestionPrompt.text_input` from `String` to `DialogTextInput`.
   - Update `ask`/`close` to `clear()` the buffer.
   - Update `type_char` (caret insert), `backspace` (`delete`-style around caret), `space`,
     `cancel_text_input`, and `confirm` to use `value()` / `insert_char` / `backspace`.
   - Add `insert_text(&mut self, text: &str)`, `move_left`, `move_right`, `move_home`, `move_end`,
     `delete`, `move_word_left`, `move_word_right`.
   - In `confirm()`, set `custom_selected = true` before switching into text mode (Option E).
3. **Fix rendering** (`question.rs::render`).
   - Render the input as three spans using `split_at_cursor`: `before`, `▏` (caret, `theme.primary`),
     `after`.
   - Build a `Paragraph` with `.wrap(Wrap { trim: false })`.
   - Compute `inner_width = width - 2`, `inner_height = area.height - 2`,
     `wrapped_rows = paragraph.line_count(inner_width)`.
   - Set `height = (wrapped_rows + 2).min(area.height.saturating_sub(2))` and bottom-anchor as today.
   - When `wrapped_rows + 2 > area.height - 2`, apply
     `.scroll((wrapped_rows + 2 - (area.height - 2), 0))` so the input line and hint remain visible.
   - Recompute clickable option rows for the wrapped layout: accumulate the wrapped row count per
     preceding content line (measure each line at `inner_width`) instead of assuming one row per
     content line, so `option_rows`/`handle_click` stay correct.
4. **Route paste and caret keys** (`crates/opencode-tui/src/app/app.rs`).
   - Add a helper such as `insert_text_into_active_input(&mut self, text: &str)` that sends text to
     `question_prompt.insert_text` when `question_prompt.is_open`, else `prompt.insert_text`.
   - Use it in the `Event::Paste(text)` arm (`app.rs:801-805`).
   - Make the `input_paste` keybind (`app.rs:557-558`) reachable while the question is open: handle it
     before/inside the question-open branch and route through the helper (reading the clipboard for
     `Ctrl+V`).
   - Extend the question-open key branch (`app.rs:406-427`) with `Left`/`Right`/`Home`/`End`/`Delete`
     and `Alt+Left`/`Alt+Right` (word skip) mapped to the new buffer methods; keep
     `Up`/`Down`/`Space`/`Enter`/`Esc`/digit behavior.
5. **Tests.**
   - `DialogTextInput::insert_str` newline/multibyte behavior.
   - Question prompt: caret insert/backspace/delete/move, `insert_text` flattening, marker set on
     entering text mode, wrapped-height and scroll behavior (render via `TestBackend` and assert the
     `>` line/hint are on-screen at a short height).
   - App-level paste routing: `Event::Paste` and `Ctrl+V` insert into the question input while open
     and into the session prompt otherwise (unit-testable via the helper).
6. **Verify.** `cargo fmt`, `cargo check -p opencode-tui`, `cargo test -p opencode-tui`.

## Scope

- Wrap question text, option labels/descriptions, and the input line; size and scroll the box so the
  active input is always visible.
- Make the custom-answer/free-text input caret-editable with a visible caret, reusing
  `DialogTextInput`.
- Route bracketed paste and `Ctrl+V` to the question input while the question prompt is open.
- Set the custom-row marker consistently when text mode is entered.
- Update/keep tests.

## Non-goals

- Changing the question/answer protocol, schema, or tool contract.
- Changing the multi-question sequential flow, the review/confirm screen semantics, or the compact
  bottom-prompt placement.
- Box centering/width polish (Option D) and collapsing the option list (Option B) — deferred/out of
  scope.
- Reworking the session prompt's own input or paste behavior when no question is open.
- Full multi-line answer editing (answers stay single-line; newlines are flattened).

## Acceptance criteria

- Long question text and option descriptions wrap within the box; nothing is clipped horizontally.
- At every supported terminal size, the `>` input line and the submit hint stay visible while typing;
  the option list scrolls instead of hiding the input.
- The custom-answer input is caret-editable: typing inserts at the caret, `Left`/`Right`/`Home`/`End`/
  `Delete` work, `Alt+Left`/`Alt+Right` skip by word, and a `▏` caret renders at the cursor offset.
- Pasting (bracketed paste and `Ctrl+V`) while the question prompt is open inserts the text into the
  active question input as a single line with the question box background; the session prompt is not
  modified while the question is open, and paste still works normally when no question is open.
- The custom row shows `[x]` when the prompt is in text mode, regardless of how the row was selected.
- The review screen wraps and keeps its options reachable.
- `cargo check -p opencode-tui` and `cargo test -p opencode-tui` pass, including the new tests.

## Done when

- All acceptance criteria are met and the locked decisions are reflected in the code.
- Focused tests cover buffer editing (including newline flattening and multibyte), wrapped-height and
  input visibility, paste routing, and the custom-row marker.
- `cargo test -p opencode-tui` passes.
- Manual smoke confirms the behavior at both a tall and a short terminal size.

## Recommended verification

- `ort-build`, then `ort`.
- Trigger a question with options + custom enabled; select `Type your own answer`; confirm the caret
  appears, type text, and move the caret with `Left`/`Right`/`Home`/`End`/`Delete`; confirm the marker
  shows `[x]`.
- Paste a multi-line blob with `Ctrl+V` and with the terminal's native paste; confirm it lands in the
  answer field as a single flattened line with the question box background and is not clipped.
- Shrink the terminal until the box would overflow; confirm the input line and hint stay visible and
  the option list scrolls.
- Trigger a question with a long question line and long option descriptions; confirm they wrap.
- Trigger a multi-question request and confirm the review screen wraps and still submits/returns.
- Trigger a free-text question, paste, and confirm the same behavior.
- `cargo fmt`, `cargo check -p opencode-tui`, `cargo test -p opencode-tui`.

## Likely touchpoints

- `crates/opencode-tui/src/components/question.rs` — layout, wrapping, height/scroll, caret render,
  custom-row state, input buffer.
- `crates/opencode-tui/src/components/dialogs/text_input.rs` — add `insert_str`; reuse the buffer.
- `crates/opencode-tui/src/components/dialogs/mod.rs` — re-export `DialogTextInput`.
- `crates/opencode-tui/src/app/app.rs:406-427` (question key routing), `:557-560` (`input_paste`), and
  `:801-805` (`Event::Paste`) — caret keys and paste routing.
- `crates/opencode-tui/src/components/prompt.rs` — wrapping/cursor patterns to mirror.
- `invariants/option-selection.md` — rules this must satisfy.
- `boards/qa/gate-question-tool-full-parity.md` (`GATE-002`) — parent gate; no new deviation expected.

## Related items

- `GATE-002` Question tool full parity — parent gate; the compact bottom prompt is a documented
  deliberate deviation and this fix stays within it.
- `FEAT-041` TUI question prompt UX parity — added descriptions, digit keys, custom answers, and the
  review screen; this bug is a follow-up defect in that work.
- `BUG-031` Dialog text inputs lack cursor navigation — same append-only text-entry defect class and
  the source of the reusable `DialogTextInput` buffer.
- `invariants/option-selection.md` — caret-editability rule for text-entry fields.

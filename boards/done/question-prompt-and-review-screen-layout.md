---
id: "BUG-039"
title: "Question prompt and review screen layout: wrapping, width, and cleanup"
priority: "P2"
type: "bug"
area: "BUG"
spec: "invariants/option-selection.md"
status: "done"
created: "2026-09-23"
updated: "2026-09-25"
---

# Question prompt and review screen layout: wrapping, width, and cleanup

> **Consolidated 2026-09-25.** A follow-up card (`BUG-042`) proposed for the same surfaces was
> folded into this card and removed. This is now the single card for question-prompt and
> review-screen layout. The 2026-09-23 sections below record the original defect and the
> implementation that already landed for wrapping/caret/paste; the **Follow-up scope (2026-09-25)**
> section adds the remaining width, no-scroll, and dead-code cleanup work.

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

- `Paragraph::wrap(Wrap { trim: false })` and `Paragraph::line_count(width)` are available in ratatui
  0.27 (`~/.cargo/registry/.../ratatui-0.27.0/src/widgets/paragraph.rs:172,189,277`).
  `Paragraph::scroll(...)` also exists but is **not used** per the 2026-09-25 no-scroll decision.
- The main prompt already wraps multi-line input (`components/prompt.rs:312-323,377-399`)
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
- **Option D (box geometry) was deferred here.** The width and layout cleanup is now in scope in
  **Follow-up scope (2026-09-25)** below; the bottom anchor still stays.
- **Wrapping.** Use `.wrap(Wrap { trim: false })` and size the box from `Paragraph::line_count`
  measured at the inner content width, not from the raw content-line count.
- **No vertical or horizontal scrolling (revised 2026-09-25).** Supersedes the earlier
  "scroll to the tail" rule. The box grows upward to fit wrapped content; horizontal content always
  wraps (never scrolls); vertical overflow beyond the available height is statically clipped from the
  top so the `>` input line and hint stay visible. No `scroll` offset or scrollable region is used.
  See **Follow-up scope (2026-09-25)** for the exact sizing rule.
- **Caret input.** Replace the append-only `String` with `DialogTextInput`; render a `▏` caret at the
  cursor via `split_at_cursor`; support `Left`/`Right`/`Home`/`End`/`Delete` and `Alt+Left`/`Alt+Right`
  word skipping; typing inserts at the caret. This satisfies `invariants/option-selection.md:23-25`.
- **Paste.** Route both bracketed paste (`Event::Paste`) and `Ctrl+V` to the question input when the
  prompt is open, and normalize the pasted text to a single line (CR/LF/tab to spaces) so it is a
  valid one-shot answer. The session prompt keeps its current paste behavior when no question is open.
- **Single-line answers.** `Enter` submits; newlines never become part of the answer.
- **Descriptions wrap; no ellipsis.** Long question text and option descriptions wrap inside the box.
- **Review screen shares the renderer.** The wrapping/height fix applies to the
  `QuestionType::Review` screen too; do not change its flow semantics.
- **No protocol/schema changes.** This is a TUI-only bug.

Resolved open questions:

- Primary trigger is horizontal clipping plus paste misrouting; the un-centered/narrow box is now
  addressed by the 2026-09-25 width decision below.
- The custom-answer input **is** subject to the caret-editability invariant; it must be
  caret-editable with a visible caret.
- Long descriptions wrap, not ellipsize.

## Follow-up scope (2026-09-25)

The wrapping/caret/paste work above landed, but the user still reports the prompt as "so squashed
and text appears outside of the defined box." The remaining work is consolidation cleanup of the
question prompt and its multi-question review/confirm screen. Locked decisions:

- **Scope is the question prompt and the review/confirm screen only.** The permission/confirm
  (`[y]/[n]/[a]/[p]`) prompt is out of scope.
- **Wrapping inside the box is the primary fix.** No content may render past the border.
- **No vertical or horizontal scrolling.** Horizontal content always wraps. Vertical content grows
  the box upward from its bottom anchor. This supersedes the 2026-09-23 "scroll to the tail" rule.
- **Horizontal width (resolved).** Target two-thirds of the area width:
  `width = (area.width * 2 / 3)` clamped to `[24, area.width.saturating_sub(2)]`. There is no
  80-column cap. Horizontally center the box: `x = area.x + (area.width - width) / 2`.
- **Vertical sizing and overflow (resolved).** `height = wrapped_rows + 2`, bottom-anchored at
  `y = area.y + area.height.saturating_sub(height + 1)`, clamped to `area.height.saturating_sub(1)`
  (one-row top margin). If `wrapped_rows` still exceeds the clamped inner height, statically drop
  leading content lines until it fits, so the `>` input line and the final hint stay visible. This is
  a fixed top-clip, not a scroll offset.
- **Keep the prompt bottom-anchored**; do not convert it to a centered modal or change the compact
  bottom-prompt placement (`GATE-002` deviation).
- **Remove the unused `ConfirmDialog`** (`crates/opencode-tui/src/components/dialogs/confirm.rs`).
  A workspace-wide search confirms the only references are its own re-exports
  (`crates/opencode-tui/src/components/dialogs/mod.rs:71`,
  `crates/opencode-tui/src/components/mod.rs:26`); it is never constructed or rendered.

Prerequisite verification (resolve before coding):

- Confirm the landed fix is in the checkout: `git merge-base --is-ancestor d5cd9fb HEAD` (or confirm
  `crates/opencode-tui/src/components/question.rs` contains `.wrap(Wrap { trim: false })` and
  `DialogTextInput`).
- Rebuild the current binary: `ort-build`.
- Launch it: `ort` (build before launching so the fresh server runs the latest binary).
- Reproduce: trigger a question with options + descriptions (ask the agent to use the question tool)
  from a realistic session; also exercise the multi-question review screen.
- Capture and record on this card: terminal size (cols x rows), the exact question/review content,
  whether it is the prompt or the review screen, and a screenshot/copy of the rendered box.
- If, with wrapping present, content still renders outside the border, that capture is the repro this
  card fixes. If content is already contained, the remaining work is only the width/no-scroll/cleanup
  items and the wrap-contention is a stale-binary artifact to note and close.

### Residual implementation plan (2026-09-25)

All paths in `crates/opencode-tui/src/components/question.rs` unless noted. The 2026-09-23 plan below
is already landed; this is the remaining work.

1. **Width.** Replace `width = area.width.saturating_sub(2).min(80)` (`:544`) with:
   `width = ((area.width as u32 * 2 / 3) as u16).clamp(24, area.width.saturating_sub(2))`. Horizontally
   center `popup_area`: `x = area.x + (area.width - width) / 2`.
2. **Height and no scroll.** Remove the `scroll` computation and `.scroll((scroll, 0))` (`:571-573`,
   `:602-603`). Compute `wrapped_rows` as today via `line_wrapped_rows` at the new `inner_width`.
   Set `height = (wrapped_rows + 2).min(area.height.saturating_sub(1))` and bottom-anchor at
   `y = area.y + area.height.saturating_sub(height + 1)`.
3. **Top-clip overflow.** When `wrapped_rows + 2 > area.height.saturating_sub(1)`, the height clamp
   binds: drop leading `content` lines and subtract their wrapped row counts until
   `wrapped_rows <= height - 2` (the inner height), keeping the `>` input line and final hint. Recompute
   row offsets and `row_indices` after trimming so `option_rows`/`handle_click` map to the shifted
   rows (clipped rows use the existing `u16::MAX` sentinel).
4. **Horizontal no-scroll.** Ensure the `Paragraph` keeps `Wrap { trim: false }` and no horizontal
   scroll/offset. Every line must wrap to `inner_width`; verify the caret input line and long
   descriptions never exceed the inner width.
5. **Review screen.** Confirm `QuestionType::Review` renders through the same path and inherits the
   width, wrapping, and top-clip behavior; no flow changes.
6. **Remove `ConfirmDialog`.**
   - Delete `crates/opencode-tui/src/components/dialogs/confirm.rs`.
   - In `crates/opencode-tui/src/components/dialogs/mod.rs`: remove `mod confirm;` and
     `pub use confirm::ConfirmDialog;`.
   - In `crates/opencode-tui/src/components/mod.rs`: remove `ConfirmDialog` from the re-export list.
7. **Tests.**
   - `TestBackend` at a wide terminal (e.g. 120x40): assert box width is ~2/3 of the area and
     horizontally centered.
   - `TestBackend` at a short terminal (e.g. 40x10): assert no glyph is written outside the box
     border and the `>` input line and hint are visible (top-clip works).
   - Long question text, long option descriptions, and a long multi-question review summary: assert
     full wrapping with no horizontal clipping.
   - No test should set or depend on a vertical scroll offset.
8. **Verify.** `cargo fmt`, `cargo check -p opencode-tui`, `cargo test -p opencode-tui`.

## Implementation plan (2026-09-23, landed)

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

- Wrap question text, option labels/descriptions, and the input line; size the box (no scrolling) so
  the active input is always visible.
- Make the custom-answer/free-text input caret-editable with a visible caret, reusing
  `DialogTextInput`.
- Route bracketed paste and `Ctrl+V` to the question input while the question prompt is open.
- Set the custom-row marker consistently when text mode is entered.
- **Follow-up:** use two-thirds of the available width (no 80-column cap), horizontally centered, and
  grow the box upward to fit wrapped content with no vertical or horizontal scrolling.
- **Follow-up:** guarantee containment on the question prompt and review/confirm screen.
- **Follow-up:** remove the unused `ConfirmDialog` and its re-exports.
- Update/keep tests.

## Non-goals

- Changing the question/answer protocol, schema, or tool contract.
- Changing the multi-question sequential flow, the review/confirm screen semantics, or the compact
  bottom-prompt placement.
- The permission/confirm (`[y]/[n]/[a]/[p]`) prompt (`components/permission.rs`).
- Collapsing the option list while typing (Option B).
- Converting the prompt to a centered modal.
- Reworking the session prompt's own input or paste behavior when no question is open.
- Full multi-line answer editing (answers stay single-line; newlines are flattened).

## Acceptance criteria

- Long question text and option descriptions wrap within the box; nothing is clipped horizontally.
- The box uses two-thirds of the horizontal space on a wide terminal, is horizontally centered, and
  stays bottom-anchored.
- The box leaves at least one blank row between the transcript above it and its top border.
- The question is shown only in the question box, not duplicated by a transcript card.
- Wrapped text breaks on word boundaries; it is never cut off mid-word.
- There is no vertical or horizontal scrolling. The box grows upward to fit wrapped content; when
  content exceeds the available height, leading lines are statically clipped from the top so the
  input line and hint remain visible, and content never renders outside the border.
- At every supported terminal size, the `>` input line and the submit hint stay visible while typing.
- The custom-answer input is caret-editable: typing inserts at the caret, `Left`/`Right`/`Home`/`End`/
  `Delete` work, `Alt+Left`/`Alt+Right` skip by word, and a `▏` caret renders at the cursor offset.
- Pasting (bracketed paste and `Ctrl+V`) while the question prompt is open inserts the text into the
  active question input as a single line with the question box background; the session prompt is not
  modified while the question is open, and paste still works normally when no question is open.
- The custom row shows `[x]` when the prompt is in text mode, regardless of how the row was selected.
- The review screen wraps, is sized like the option screen, and keeps its options reachable.
- `ConfirmDialog` is removed and no references remain.
- `cargo check -p opencode-tui` and `cargo test -p opencode-tui` pass, including the new tests.

## Done when

- All acceptance criteria are met and the locked decisions are reflected in the code, including the
  2026-09-25 follow-up decisions.
- Focused tests cover buffer editing (including newline flattening and multibyte), wrapped-height and
  input visibility, paste routing, the custom-row marker, containment/width in `TestBackend`, and the
  removal of `ConfirmDialog`.
- `cargo test -p opencode-tui` passes.
- Manual smoke confirms the behavior at both a tall and a short terminal size.

## Recommended verification

- `ort-build`, then `ort`.
- Trigger a question with options + custom enabled; select `Type your own answer`; confirm the caret
  appears, type text, and move the caret with `Left`/`Right`/`Home`/`End`/`Delete`; confirm the marker
  shows `[x]`.
- Paste a multi-line blob with `Ctrl+V` and with the terminal's native paste; confirm it lands in the
  answer field as a single flattened line with the question box background and is not clipped.
- Shrink the terminal until the box would overflow; confirm the box grows upward to fit the wrapped
  content and, past the available height, statically clips leading lines so the input line and hint
  stay visible. Confirm there is no vertical scroll (content does not move as you type) and nothing
  renders past the border.
- On a wide terminal, confirm the box uses two-thirds of the width, is horizontally centered, and
  stays bottom-anchored.
- Trigger a question with a long question line and long option descriptions; confirm they wrap with no
  horizontal scrolling or clipping.
- Trigger a multi-question request and confirm the review screen wraps, widens like the option
  screen, and still submits/returns.
- Trigger a free-text question, paste, and confirm the same behavior.
- Confirm `ConfirmDialog` is gone and the TUI builds/tests clean.
- `cargo fmt`, `cargo check -p opencode-tui`, `cargo test -p opencode-tui`.

## Likely touchpoints

- `crates/opencode-tui/src/components/question.rs` — two-thirds width and horizontal centering,
  wrapped sizing with top-clip (no scroll), caret render, custom-row state, input buffer.
- `crates/opencode-tui/src/components/dialogs/text_input.rs` — add `insert_str`; reuse the buffer.
- `crates/opencode-tui/src/components/dialogs/confirm.rs` — delete the unused `ConfirmDialog`.
- `crates/opencode-tui/src/components/dialogs/mod.rs` — re-export `DialogTextInput`; drop the
  `confirm` module and its re-export.
- `crates/opencode-tui/src/components/mod.rs` — drop the `ConfirmDialog` re-export.
- `crates/opencode-tui/src/app/app.rs:406-427` (question key routing), `:557-560` (`input_paste`), and
  `:801-805` (`Event::Paste`) — caret keys and paste routing.
- `crates/opencode-tui/src/components/prompt.rs` — wrapping/cursor patterns to mirror.
- `invariants/option-selection.md` — rules this must satisfy.
- `boards/qa/gate-question-tool-full-parity.md` (`GATE-002`) — parent gate; no new deviation expected.

## Implementation Notes

Historical record of the landed 2026-09-23 work. Its tail-scroll behavior is **superseded** by the
2026-09-25 no-scroll decision above; keep the caret/paste/marker parts.

Implemented A + C + E + F on `bug/BUG-039-question-answer-layout`. No protocol/schema changes;
TUI-only.

- `crates/opencode-tui/src/components/dialogs/text_input.rs`: added
  `DialogTextInput::insert_str`, which inserts at the caret and flattens
  `\r\n`/`\r`/`\n`/`\t` to single spaces. Unit-tested for newline/tab flattening and multibyte
  caret insertion.
- `crates/opencode-tui/src/components/dialogs/mod.rs`: re-exported `DialogTextInput`.
- `crates/opencode-tui/src/components/question.rs`:
  - Replaced the append-only `String` answer buffer with `DialogTextInput`; `type_char`/`space`/
    `backspace` insert/delete at the caret, and `Left`/`Right`/`Home`/`End`/`Delete`/word-skip are
    exposed.
  - Added `insert_text` for paste; it enters custom-answer mode for a choice question with a custom
    row and flattens newlines.
  - Added `enter_custom_text_mode`, used when `confirm()` switches the custom row into text mode, so
    the row marker shows `[x]` regardless of how it was reached (Option E). `cancel_text_input` now
    clears the marker.
  - Render now wraps with `Wrap { trim: false }`, sizes the popup from per-line
    `Paragraph::line_count` at the inner width, and scrolls to the tail when wrapped content
    overflows so the `>` input line and hint stay visible. Clickable option rows are recomputed from
    wrapped row offsets; scrolled-away rows use a `u16::MAX` sentinel so the index mapping is
    preserved.
  - The input line renders three spans via `split_at_cursor` with a `▏` caret at the cursor offset.
- `crates/opencode-tui/src/app/app.rs`: added `insert_text_into_active_input`, used by
  `Event::Paste` and the `input_paste` keybind; the question-open key branch now routes `Ctrl+V` and
  the caret keys, and ignores `Alt`/`Ctrl` for plain `Char` typing.
- `crates/opencode-tui/Cargo.toml`: enabled ratatui's `unstable-rendered-line-info` feature, required
  for `Paragraph::line_count` (the measurement API named in the locked decisions).

Verification: `cargo fmt`, `cargo check -p opencode-tui`, and `cargo test -p opencode-tui`
(111 passed). New focused tests cover caret edit/delete/word movement, paste flattening and
custom-mode entry, custom-row marker on text-mode entry, and a `TestBackend` render at a short 40x10
terminal asserting the input hint and caret stay on-screen.

Not covered by an automated test: the `App`-level paste routing helper itself, because `App::new()`
initializes the real terminal and is not constructible in unit tests. The routing target
(`QuestionPrompt::insert_text`/`insert_str`) is tested directly; the thin `App` helper still needs the
manual smoke step below.

## Live QA feedback and implementation (2026-09-25)

Live TUI repro (fresh build, `ort`) confirmed the prompt still looked cramped and surfaced three
additional issues; all are addressed on `bug/BUG-039-question-prompt-layout`:

- **No gap above the box.** The box touched/overlapped the transcript. The box now reserves one blank
  row above and below (`max_height = area.height - 2`) and bottom-anchors within that.
- **Question shown twice.** The pending `question` tool call rendered a transcript card whose argument
  preview repeated the question JSON, while the prompt box also showed it. `render_tool_call` now
  returns no lines for a non-completed `question` call, so the prompt box is the only place the
  question appears. The redundant in-box `Question:` label was also removed (the border title labels
  it).
- **Mid-word wrapping / cramped width.** Width is now two-thirds of the area (clamped, centered) and
  the popup area is cleared before drawing so transcript glyphs cannot bleed through.

Implementation:

- `crates/opencode-tui/src/components/question.rs`: removed `Paragraph::scroll`;
  `width = clamp(area.width * 2 / 3, 24, area.width - 2)` centered via
  `x = area.x + (area.width - width) / 2`; `height` capped at `area.height - 2` with leading content
  lines dropped when wrapped content overflows (top-clip, no scroll); `Clear` rendered over the popup
  area before the paragraph; removed the `Question:` body label.
- `crates/opencode-tui/src/components/session_tool.rs`: suppress the transcript card for a pending or
  running `question` tool call.

Verification:

- `cargo fmt --all`.
- `cargo test -p opencode-tui --lib components::question` — 18 passed, including the new
  `wide_terminal_uses_two_thirds_width_and_centers` and the existing short-terminal input/hint test.
- Full `cargo test -p opencode-tui --lib` still hits the pre-existing unrelated
  `components::prompt::tests::tab_autocomplete_uses_first_candidate` failure (and a poisoned-lock
  cascade in `utf8_backspace_delete_and_cursor_are_char_safe` that passes in isolation); neither
  touches the question prompt.

## QA Verification - 2026-09-25

Live TUI verification on `bug/BUG-039-question-prompt-layout` after rebuilding and relaunching `ort`:

- The question prompt renders at two-thirds width, horizontally centered, with a blank row above and
  below and no overlap with the transcript.
- The question appears only in the prompt box; the pending `question` tool card no longer duplicates
  it in the transcript.
- Long question text and option descriptions wrap on word boundaries within the border.
- The multi-question review/confirm screen renders correctly.
- Maintainer: "Looks great now, we can close this."

## Related items

- `BUG-042` (folded/removed 2026-09-25) — follow-up for question-prompt and review-screen width,
  no-inner-scroll, and `ConfirmDialog` removal; now part of this card.
- `GATE-002` Question tool full parity — parent gate; the compact bottom prompt is a documented
  deliberate deviation and this fix stays within it.
- `FEAT-041` TUI question prompt UX parity — added descriptions, digit keys, custom answers, and the
  review screen; this bug is a follow-up defect in that work.
- `BUG-031` Dialog text inputs lack cursor navigation — same append-only text-entry defect class and
  the source of the reusable `DialogTextInput` buffer.
- `invariants/option-selection.md` — caret-editability rule for text-entry fields.
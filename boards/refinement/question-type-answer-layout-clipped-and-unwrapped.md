---
id: "BUG-039"
title: "Question prompt custom-answer layout clips the typed answer and does not wrap"
priority: "P2"
type: "bug"
area: "BUG"
spec: "invariants/option-selection.md"
status: "refinement"
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

This is a placeholder bug. It captures the reported symptom and the concrete layout/paste defects
found in the renderer; the exact intended design is left open for refinement (see "Refinement
Options").

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

Renderer reproduced with a temporary `TestBackend` harness (removed); no code changes are included in
this card.

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

## Why this matters

The custom answer is the escape hatch when none of the provided options fit. In that exact moment the
user may be unable to see what they are typing, cannot edit it with a caret, and cannot paste into
the field. That makes the question tool unreliable for its most flexible input path and violates the
option-selection invariants for text-entry fields.

## Refinement options

These are the decision points to resolve before implementation. Pick one or a combination.

- **Option A — Wrap + guarantee input visibility (required).**
  Add `.wrap(Wrap { trim: false })` to the prompt `Paragraph`; compute the box height from the
  *wrapped* line count; and when content still exceeds the area, reserve rows for the active input
  line and hint (scroll the option list) so the typed answer is never clipped. Reuse the wrapping
  approach already in `components/prompt.rs`. This also fixes the collapsed-newline/horizontal
  clipping symptom in item 8.
  - Pros: fixes both reported clipping symptoms without changing the interaction model. Localized.
  - Cons: needs a wrapped-height calculation; option-list scrolling adds a little complexity.

- **Option B — Collapse the option list while typing the custom answer (optional).**
  When text mode is entered, hide or dim the options and show a single, prominent, bordered input
  with a caret. Decide how the user gets back to the option list (`Esc` already cancels the edit).
  - Pros: the answer is always visible and obviously the active thing; shorter box; better on small
    terminals.
  - Cons: larger UX change; another deviation to document in `GATE-002`; needs an explicit
    back-to-options affordance.

- **Option C — Caret-aware custom input (required).**
  Replace the append-only `String` with a cursor-aware buffer (reuse `DialogTextInput` from
  `BUG-031` or the prompt's cursor model) and render a `▏` caret at the cursor offset. Support
  `Left`/`Right`/`Home`/`End`/`Delete`. The buffer also gives paste a correct insertion target and
  caret position.
  - Pros: satisfies `invariants/option-selection.md`; consistent with dialogs and the main prompt.
  - Cons: more surface area; needs key routing in `app.rs:406-426` to reach the buffer.

- **Option D — Box geometry cleanup (optional).**
  Center the panel or let it span the available width (raise/remove the 80-column cap) and keep the
  bottom anchor.
  - Pros: addresses the "misaligned" feel on wide terminals.
  - Cons: cosmetic; does not fix clipping.

- **Option E — Fix the custom-row marker (included).**
  Set `custom_selected = true` when `confirm()` enters text mode so the row shows `[x]`, matching the
  digit-selection path.
  - Pros: trivial, removes a confusing state inconsistency.
  - Cons: purely visual; depends on the Option B decision (if options collapse, this may be moot).

- **Option F — Route paste to the question input (required, newly added).**
  Handle `Event::Paste` and any paste keybind so that when `question_prompt.is_open` the pasted text
  is inserted into the question's active input (custom-answer or text question) using the Option C
  buffer, instead of the session prompt. Keep the session prompt path unchanged when no question is
  open.
  - Pros: fixes the wrong-field and wrong-background symptoms directly; required for paste to work at
    all in the question flow.
  - Cons: small event-loop change; must not double-handle while a text question is active.

Recommended default for refinement: **A + C + E + F**, keeping the current interaction model and the
bottom-anchored compact prompt. Treat **B** as the alternative if the maintainer prefers a cleaner
text-entry mode, and **D** as an optional polish item.

## Refinement decisions (2026-09-23 follow-up)

The new paste notes resolve part of the open refinement, but not all of it:

- **Resolved — "different background color" is not a layout/theme choice.** It is caused by paste
  being routed to the session prompt (`background_element`) instead of the question box
  (`background_panel`). Fixed by Option F, not by a styling decision.
- **Resolved — horizontal overflow is a wrapping defect.** Confirmed by the `TestBackend` render
  (newlines collapse, the line is clipped). Option A is required; it is not a matter of preference.
- **Still open — wrap vs. collapse (A vs. B).** Both contain the text; the choice changes how much
  of the box the input occupies and whether the option list stays visible while typing.
- **Still open — caret model (C).** Whether the one-shot custom answer must be fully caret-editable
  per `invariants/option-selection.md` or append-only is acceptable.
- **Optional — geometry (D) and marker (E).**

So the options are narrowed, not fully resolved: F and A are mandatory, C is strongly recommended
(and is the natural insertion target for F), and B/D remain design choices for the maintainer.

## Open questions

- Does the "weird" impression come mainly from clipping, from the missing caret, from paste landing
  in the wrong field, or from the un-centered box? A screenshot/terminal size from the reporter would
  confirm the primary trigger.
- Should the custom input keep the option list visible (current design) or collapse it (Option B)?
- Is the custom-answer input covered by the `invariants/option-selection.md` caret rule, i.e. must it
  be caret-editable, or is append-only acceptable for a short one-shot answer?
- Should long option descriptions wrap or be truncated with an ellipsis?
- Does the multi-question review screen (`QuestionType::Review`) share the same clipping/wrapping
  defect? It uses the same renderer, so it likely does.

## Likely touchpoints

- `crates/opencode-tui/src/components/question.rs` — layout, wrapping, height/scroll, caret render,
  custom-row state.
- `crates/opencode-tui/src/app/app.rs:406-426` — question key routing, and `:801-805` /
  `:557-560` — paste routing (Options C and F).
- `crates/opencode-tui/src/components/dialogs/text_input.rs` — reusable cursor buffer for Option C.
- `crates/opencode-tui/src/components/prompt.rs` — existing wrapping/cursor patterns to mirror.
- `invariants/option-selection.md` — rules this must satisfy.
- `boards/qa/gate-question-tool-full-parity.md` (`GATE-002`) — any UX deviation must be recorded here.

## Non-goals

- Redesigning the question/answer protocol, schema, or tool contract.
- Changing the multi-question sequential flow or the review/confirm screen behavior.
- Rebuilding the whole question prompt visual design beyond what the chosen option requires.

## Done when

- The chosen refinement option(s) are recorded on this card (and any deviation in `GATE-002`).
- The typed custom answer is always visible while typing, at every supported terminal size, and long
  question/description/answer text wraps instead of being silently truncated.
- Pasting while the question prompt is open inserts into the active question input with the question
  box background, not into the session prompt.
- If the caret option is chosen, the custom input is caret-editable and shows a caret, per
  `invariants/option-selection.md`.
- The custom-row selection marker is consistent regardless of how the row was selected.
- Focused tests cover wrapped-height/scroll behavior, paste routing, and the custom-answer input
  path; `cargo test -p opencode-tui` passes.
- Manual smoke: single question with custom answer, multi-select with custom answer, a free-text
  question, a long question/description, and a multi-line paste, at both a tall and a short terminal
  size.

## Related items

- `GATE-002` Question tool full parity — parent gate; the compact bottom prompt is a documented
  deliberate deviation and this fix must keep it documented.
- `FEAT-041` TUI question prompt UX parity — added descriptions, digit keys, custom answers, and the
  review screen; this bug is a follow-up defect in that work.
- `BUG-031` Dialog text inputs lack cursor navigation — same append-only text-entry defect class and
  the likely source of a reusable cursor buffer.
- `invariants/option-selection.md` — caret-editability rule for text-entry fields.

---
id: "BUG-059"
title: "Pasted text is not bounded within the prompt input box (control characters and CRLF break wrapping)"
priority: "P1"
type: "bug"
area: "BUG"
spec: ""
status: "doing"
created: "2026-10-07"
---

# Pasted text is not bounded within the prompt input box (control characters and CRLF break wrapping)

## Summary

Pasted text is not contained by the main prompt input box. Pasted content that carries
`\r\n`/`\r` line endings or `\t` characters is stored verbatim and then mis-handled by the
prompt's wrap-and-render path, so line breaks are lost, lines merge, and raw control
characters are written straight to the terminal. The result is text that visibly escapes or
corrupts the input box instead of being wrapped inside it.

Reported by the operator 2026-10-07 on a fresh `ort-build` + `ort` from `development`:

> "pasted text still not bounded within the text input box"

Confirmed surface: the main prompt input box (session/home composer). The question tool's
custom-answer box is a separate surface and was fixed under `BUG-039`; this card is the
main-prompt composer.

## Reproduction

1. `ort-build`, then `ort` on a current `development` checkout.
2. Paste multi-line text whose newlines are CRLF (`\r\n`) or lone `\r`, or that contains tab
   characters (e.g. code), into the prompt.
3. Observe: the box does not show the pasted lines as separate bounded rows. CRLF/CR line
   breaks are swallowed and adjacent lines merge; control characters are emitted raw to the
   terminal, desyncing the render from the box (carriage returns jump to column 0, tabs
   advance to a tab stop), so glyphs appear outside/over the box and its border.

Pasted LF-only plain text (`\n`, no tabs) renders bounded, which is why the defect reads as
specific to "pasted" content.

## Root cause

All paths in `crates/opencode-tui/src/components/prompt.rs` unless noted.

1. **Paste is stored verbatim.** `Prompt::insert_text` (`prompt.rs:735-740`) does a raw
   `self.input.insert_str(...)` with no normalization, so `\r`, `\t`, and `\r\n` from the
   clipboard enter the input. `Event::Paste` routes here via
   `App::insert_text_into_active_input` (`crates/opencode-tui/src/app/app.rs:889-891`,
   `:2105-2114`). Native bracketed paste is enabled
   (`crates/opencode-tui/src/app/terminal.rs:12`), and `Ctrl+V` uses the same path.

2. **CRLF/CR are treated as whitespace, not line breaks.** `wrap_prompt_input`
   (`prompt.rs:1385`) iterates graphemes with `input.grapheme_indices(true)`
   (`prompt.rs:1394`) and only treats a lone `grapheme == "\n"` as a line break
   (`prompt.rs:1398`). Per UAX #29 (GB3), `\r\n` is a **single** grapheme cluster, so it never
   matches `"\n"`; it and a lone `\r` fall through to
   `prompt_grapheme_is_whitespace` (`prompt.rs:1295`) and are accumulated as pending
   whitespace. The pending run is later spliced into the middle of a rendered line, so
   CRLF/CR line breaks are lost and pasted lines merge. Raw `\r\n`/`\r` then reaches the
   terminal.

3. **Control characters are width-1 in the wrap math but raw in the render.**
   `prompt_grapheme_width` (`prompt.rs:1299`) uses `UnicodeWidthStr::width`, which returns
   `1` for both `\t` and `\r`. The wrapped lines are rendered by a `Paragraph` built from
   `display_lines` with **no** `.wrap(...)` (`prompt.rs:389-409`), so the control characters
   are stored in buffer cells and the crossterm backend emits them raw via
   `Print(cell.symbol())` (`ratatui-0.27.0/src/backend/crossterm.rs:188`). The terminal then
   expands a tab to the next tab stop and a `\r` returns the cursor to column 0, so the real
   cursor desyncs from ratatui's cell grid and content escapes/overwrites the box.

4. **Whitespace runs at a line start can exceed the box width.** In the word-fit branch
   (`prompt.rs:1432`), the wrap test is skipped when `current.is_empty()`, so a run of
   whitespace longer than the inner width is appended unbounded in the `else` branch
   (`prompt.rs:1440-1447`). A `TestBackend` probe showed a whitespace-only line of width
   `200` and a leading-whitespace line of width `60` at an inner limit of `45`. Such
   over-width lines break the scroll/cursor math even though ratatui clips them.

## Evidence

Temporary `TestBackend` probes (removed; no code changes in this card):

- CRLF input `"line one ... here\r\nline two follows\r\nline three"` renders as one merged
  stream (`line two followsline three` on one row) instead of three bounded rows; the wrap
  report showed no line break at either `\r\n`.
- `wrap_prompt_input("col1\tcol2\t...", 45)` returns a single line whose cells contain raw
  `\t`; the same line would tab-expand past the box in a real terminal.
- `wrap_prompt_input(" ".repeat(200), 45)` returns one `200`-column line (`overflow=1`);
  `" ".repeat(60) + "word"` returns a `60`-column line (`overflow=1`).
- `UnicodeWidthStr::width("\t") == 1` and `width("\r") == 1`.

## Why this matters

The prompt composer is the primary daily-driver surface. Pasting is a core input method and
the clipboard payload is often CRLF text (Windows, HTML, email, many editors) or tab-indented
code. A raw `\r` emitted to the terminal does not just misplace a glyph; it can jump the
cursor and overwrite previously drawn cells, so the box, its border, and the status rows can
all render wrong. This blocks trust in paste for the V1 daily-driver loop.

## Scope

- Normalize pasted/inserted text at the model boundary so the box only ever holds printable,
  LF-separated content:
  - convert `\r\n` and lone `\r` to `\n`;
  - expand or replace `\t` (and any other non-printing C0 control characters other than
    `\n`) with spaces so `UnicodeWidthStr::width` matches what the terminal renders.
- Make `wrap_prompt_input` robust as a backstop: treat `\r\n`/`\r` as line breaks even if
  normalization is bypassed (history/stash/autocomplete paths), and never emit a wrapped line
  wider than the inner width (wrap whitespace-only/leading-whitespace runs).
- Keep the cursor visual-position math (`WrappedPromptInput::cursor_visual_position`,
  `move_cursor_vertical`) consistent with whatever normalization is chosen.
- Preserve existing live input behavior: LF newline (`Ctrl+J`/paste), Enter submit, history,
  stash, autocomplete, shell mode, and the isolated cursor/scroll behavior.

## Non-goals

- The question tool custom-answer box (`BUG-039`) or dialog text fields (`BUG-031`), which
  already flatten paste to a single line in
  `crates/opencode-tui/src/components/dialogs/text_input.rs`.
- Changing the prompt box height cap (`PROMPT_MAX_INPUT_LINES = 6`) or the home box's fixed
  height.
- Changing paste routing or clipboard read/write.
- Adding rich paste handling (images, structured content).

## Done when

- Pasting CRLF text produces one bounded row per original line; no line merges and no raw
  `\r` reaches the terminal.
- Pasting tab-indented text renders within the box (tabs normalized), with no cursor desync or
  glyphs outside the box border.
- No wrapped line produced by `wrap_prompt_input` exceeds the inner width for any input,
  including whitespace-only runs.
- The caret and vertical movement stay correct after a paste with CRLF/tabs.
- A regression test covers CRLF, lone `\r`, and tab paste into the main prompt.

## Recommended verification

- `env -u OPENSSL_DIR SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test -p opencode-tui --lib`
  (add `wrap_prompt_input`/`insert_text` tests: CRLF/CR split, tab normalization, no
  line over inner width).
- `cargo fmt --all`, `cargo check -p opencode-tui`.
- `ort-build` then `ort`: paste a CRLF multi-line blob, a tab-indented code block, and a
  long-space-aligned block; confirm every line stays inside the box, the caret tracks, and no
  glyph appears outside the border.

## Related Items

- `BUG-039` Question prompt and review screen layout - same "pasted text not horizontally
  contained" class on the question tool custom-answer box; already handled there with paste
  routing and single-line flattening.
- `BUG-031` Dialog text inputs lack cursor navigation - established `DialogTextInput` and its
  `insert_str` newline/tab flattening, the reference pattern for normalization.
- `BUG-013` Cursor on the input field needs to always be visible and `FEAT-037` prompt-history
  cursor gating - both touch the wrap/cursor math this fix must keep consistent.
- `PHASE-001` V1 daily-driver hardening - paste reliability is part of the primary input loop.

## Notes

- Reported by the operator 2026-10-07 via session `Jasper:pid152032`, on a fresh
  `ort-build` + `ort` from `development` (confirmed not a stale binary).
- LF-only prose without tabs already renders bounded; the defect is driven by the control
  characters and CRLF that clipboard payloads commonly contain.
- Investigation used temporary `TestBackend` probes and direct `wrap_prompt_input` calls; the
  probes were removed and no product code was changed in this card.

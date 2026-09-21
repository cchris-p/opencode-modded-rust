---
id: "BUG-024"
title: "BUG: Mouse-selection copy captures TUI decoration (gutters, bullets, box-drawing) instead of clean markdown"
priority: "P2"
type: "bug"
area: "BUG"
spec: "wiki/v1.md"
status: "todo"
created: "2026-09-21"
---

# BUG: Mouse-selection copy captures TUI decoration (gutters, bullets, box-drawing) instead of clean markdown

## Summary

Selecting text in the TUI and copying it puts the on-screen *rendering* on the clipboard, not the
underlying source. Every decorative character the markdown renderer and message gutters emit is
included verbatim: the `┃ ` user-message gutter, `│ ` blockquote/indent prefixes, code-block frame
lines, table box-drawing, and `• ` bullets. Copying a block of prose therefore yields text polluted
with vertical bars and indentation glyphs that are useless outside the terminal, and lists do not
come back as markdown. The user expectation is that selecting rendered content and copying it yields
clean markdown text that can be pasted into an editor, issue, or another session.

## Reported behavior

- Copying a selection includes the vertical line (`│` / `┃`) that the TUI draws.
- Indentation/blockquote lines are copied as `│ ` prefix characters instead of real indentation or
  `>` markers.
- Bullet list items copy as `• item` rather than `- item` (and ordered items as the rendered number).
- The result is not pasteable markdown; it has to be hand-cleaned every time.

## Why this exists

Copy is the primary way content leaves the TUI. If selection copy faithfully reproduces screen
decoration instead of the source text, the daily-driver workflow has no clean path from a rendered
answer to a reusable markdown artifact. This is a correctness bug in the copy path: the same content
already exists as markdown (the transcript is built from it), but the selection route bypasses it and
scrapes the rendered buffer.

## Code evidence

- Selection copy scrapes the rendered screen buffer rather than any source text:
  `crates/opencode-tui/src/app/app.rs:1901-1923` (`copy_selection`) reads `self.screen_lines` and
  calls `Selection::get_selected_text`.
- `screen_lines` is populated from captured terminal cells at draw time:
  `crates/opencode-tui/src/app/app.rs:94`, `:3913` (`self.screen_lines = captured_lines;`).
- `Selection::get_selected_text` slices that captured text by display column and trims trailing
  whitespace only; it has no concept of markdown: `crates/opencode-tui/src/ui/selection.rs:112-151`.
- Decoration is emitted into those captured lines by the renderers:
  - user-message gutter `┃ `: `crates/opencode-tui/src/components/session_message.rs:22`, used at
    `:27-30` and `:39-41`.
  - blockquote/indent prefix `│ ` (`ensure_prefix`): `crates/opencode-tui/src/components/markdown/renderer.rs:615-625`,
    invoked on every text-bearing event (`:222`, `:264`, `:288`, `:383`, `:389`, `:431`, `:441`).
  - bullet glyph `• ` and ordered prefix `N. `: `crates/opencode-tui/src/components/markdown/renderer.rs:223-234`.
  - code-block frame `╭`/`│ ` lines and `╰───`: `crates/opencode-tui/src/components/markdown/renderer.rs:482-509`.
  - table box-drawing `┌┬┐├┼┤└┴┘│─`: `crates/opencode-tui/src/components/markdown/renderer.rs:513-558`.
  - prompt gutter `┃`/`vertical_left`: `crates/opencode-tui/src/components/prompt.rs:260`, `:1284`.
- The existing full-session copy path already produces markdown and does not have this problem:
  `handle_copy_session` / `build_session_transcript` at `crates/opencode-tui/src/app/app.rs:2130-2154`
  (`TranscriptOptions`). This shows source markdown is available; the selection route is the outlier.
- The selection tests only cover glyph-boundary slicing against raw decoration
  (`crates/opencode-tui/src/ui/selection.rs:236-251`), so the decoration leak is untested.

## Suspected root cause

There are two independent copy mechanisms. `/copy` serializes the session from its source model,
while mouse selection extracts characters from the already-rendered ratatui buffer. The rendered
buffer is a lossy, decoration-laden projection of the markdown, so any selection made over rendered
content necessarily carries the gutters, bullets, and box-drawing with it. Fixing this at the slicing
layer alone cannot recover markdown that was never retained (e.g. `•` vs `-`, code fences, table
pipes), so the fix needs a source-aware mapping or a parallel markdown projection per rendered line.

## Scope

- Make mouse-selection copy yield clean, pasteable markdown for assistant text, user messages, list
  items, blockquotes, code blocks, and tables.
- Remove copy-visible TUI decoration: `┃ `/`│ ` gutters, `• ` bullets, code-block frames, and table
  box-drawing.
- Preserve markdown semantics that the renderer currently discards: `-`/`*` bullets, ordered lists,
  `>` blockquotes, fenced code blocks with language, and pipe tables.
- Keep the existing selection UX (drag to select, copy on release, toast) and column hit-testing
  behavior intact.
- Keep `/copy` (full transcript) behavior as-is; this card is about selection copy.
- Decide and document the approach (see Notes) and cover it with tests.

## Non-goals

- Rewriting the markdown renderer's on-screen appearance; the TUI may keep drawing gutters and
  box-drawing.
- Changing `/copy`, session export, or `TranscriptOptions`.
- Adding a new copy keybinding or command; this is fidelity of the existing selection copy.
- Rich-text/HTML or image clipboard formats.
- Copying content that only exists as decoration (e.g. the code-block frame when no code is inside).

## Done when

- Selecting assistant or user markdown and copying yields source markdown with no `┃`, `│`, `•`,
  `╭`, `╰`, or table box-drawing characters.
- Bullet lists copy as `- item` (or the source marker), ordered lists as `1. item`, blockquotes with
  `>`, code blocks with ``` fences and language, and tables as pipe tables.
- Partial selections (single line, first/last line, mid-line column bounds) still clip correctly and
  do not emit partial decoration glyphs.
- Selection UX is unchanged: drag, release-to-copy, and toast still work; `/copy` is unaffected.
- `cargo check -p opencode-tui` and `cargo test -p opencode-tui` pass, with tests asserting that
  copied selection text contains markdown markers and none of the decoration glyphs.

## Recommended verification

- `ort-build`, then `ort`; ask for a reply containing a bullet list, a blockquote, a fenced code
  block, and a table.
- Select the reply and paste into an editor; confirm it is valid markdown with no vertical bars or
  box-drawing and that bullets are `-`.
- Select a user message and confirm the `┃ ` gutter is not copied.
- Select a single line and a partial multi-line span; confirm clipping is correct and no stray glyphs
  appear at the boundaries.
- Run `/copy` and confirm the full-transcript markdown output is unchanged.
- Add unit tests over the selection/markdown-projection path covering decoration stripping and list
  conversion.

## Product decisions

- Selection copy should produce markdown, not a screenshot of the terminal.
- On-screen gutters and box-drawing remain a display affordance; they must not leak into the
  clipboard.
- The rendered view may stay lossy; the copy path is responsible for emitting faithful source
  markdown.

## Related Items

- `FEAT-001` Improve historical chat transcripts workflow - transcript/markdown fidelity.
- `BUG-022` `/thinking` toggle shows a line count instead of reasoning - another case where rendered
  display and underlying content diverge.
- `FEAT-028` Add a hide-tool-calls toggle that compacts runs to a tool-call count - collapsed views
  must not degrade copy output.
- `PHASE-001` V1 daily-driver hardening.

## Notes

- Relevant files: `crates/opencode-tui/src/app/app.rs` (`screen_lines`, `copy_selection`),
  `crates/opencode-tui/src/ui/selection.rs` (text extraction),
  `crates/opencode-tui/src/components/markdown/renderer.rs` (decoration + list/table/code emission),
  `crates/opencode-tui/src/components/session_message.rs` (message gutter),
  `crates/opencode-tui/src/components/prompt.rs` (prompt gutter).
- Candidate approaches to decide during refinement:
  1. Source-aware mapping: track, per rendered line, the originating message/part and source offset,
     and copy from the source markdown for that span. Most faithful, largest change.
  2. Parallel markdown projection: build a per-row plain/markdown companion to `screen_lines` during
     render and slice that instead. Medium change; requires keeping the two row sets in lockstep.
  3. Strip-and-reconstruct on copy: remove known decoration and convert `• ` back to `- `. Smallest
     change, but cannot faithfully recover code fences or tables.
- `/copy`/`build_session_transcript` already demonstrates the desired markdown shape and should be
  the reference for expected output.
- Confirm which session render path is actually used (session vs. session_message vs. message) before
  wiring the chosen approach, since decoration is added in more than one place.

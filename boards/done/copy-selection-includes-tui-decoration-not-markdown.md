---
id: "BUG-024"
title: "BUG: Mouse-selection copy captures selectable gutter decoration instead of clean text"
priority: "P2"
type: "bug"
area: "BUG"
spec: "wiki/v1.md"
status: "done"
created: "2026-09-21"
---

# BUG: Mouse-selection copy captures selectable gutter decoration instead of clean text

## Summary

Selecting text in the TUI and copying it puts visible layout decoration on the clipboard. The most
painful case is leading message chrome such as spaces plus `│`, `┃`, `▸`, or similar caret/gutter
markers. The selection highlight also makes those characters look like part of the selectable answer,
so the user expects the copied result to include UI chrome before the real text. A copied line like
`  │▸ I have enough to write the spec...` should paste as
`I have enough to write the spec...`.

The desired UI direction is to keep the TUI visually comfortable with padding/margins, but avoid
placing selectable decoration glyphs in front of message content. Clean terminal selection should be
possible without requiring the user to hand-clean leading bars, carets, or extra indentation.

## Reported behavior

- Copying a selection includes leading UI chrome such as `│`, `┃`, `▸`, and the spaces used to align
  those markers.
- The copied text often starts with indentation that exists only because a gutter was rendered.
- The selection highlight covers those glyphs, so the highlighted region does not visually match the
  clean text the user expects to paste.
- For normal prose, the result has to be hand-cleaned before it can be pasted into an editor, issue,
  or another session.
- Markdown decoration such as bullets, code-block frames, and table box drawing can still leak through
  the same rendered-buffer copy path, but this card's first implementation target is the leading
  selectable gutter/chrome problem.

## Why this exists

Copy is the primary way content leaves the TUI. If normal terminal selection faithfully reproduces
screen chrome instead of the visible answer text, the daily-driver workflow has no clean path from a
rendered answer to a reusable note, issue, or follow-up prompt. A fully source-aware markdown copy
path may still be valuable later, but the immediate fix should make ordinary mouse selection clean by
keeping decorative gutter glyphs out of selectable message rows while preserving padding/margins.

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
buffer currently includes decorative gutter glyphs in the same selectable rows as message text, so
terminal selection naturally copies those glyphs. The smallest correct fix is to stop rendering
copy-visible leading chrome in message text rows and use whitespace padding/margins for visual
separation instead.

This will not make every rendered markdown construct perfectly source-faithful. For example, if the
renderer still draws a bullet as `•` or a table with box-drawing characters, terminal selection may
still copy those rendered glyphs. That broader source-markdown projection is not required for this
first fix unless it is needed to remove leading gutter chrome.

## Scope

- Make normal mouse-selection copy for assistant and user prose omit leading UI chrome: `┃`, `│`,
  `▸`, equivalent gutter/caret markers, and indentation that exists only to align those markers.
- Preserve visual breathing room with blank padding/margins instead of selectable glyph gutters.
- Make the highlighted selection look much closer to what will be copied; the user should not see
  obvious gutter glyphs inside the selected region for ordinary message text.
- Keep common markdown selections no worse than today, and remove leading message gutters from list,
  blockquote, code-block, and table lines when those gutters are outside the content itself.
- Keep the existing selection UX (drag to select, copy on release, toast) and column hit-testing
  behavior intact.
- Keep `/copy` (full transcript) behavior as-is; this card is about selection copy.
- Prefer the minimal render/layout change over source-aware markdown mapping for this card.
- Cover the final behavior with tests where practical.

## Non-goals

- Removing all visual padding or collapsing the message layout to the terminal edge.
- Full source-aware markdown reconstruction for arbitrary partial selections.
- Changing `/copy`, session export, or `TranscriptOptions`.
- Adding a new copy keybinding or command; this is fidelity of the existing selection copy.
- Rich-text/HTML or image clipboard formats.
- Copying content that only exists as decoration (e.g. the code-block frame when no code is inside).
- Perfect markdown fidelity for tables, code fences, and renderer-emitted bullets if those require a
  larger source-offset mapping. Track that as a follow-up if needed after gutter cleanup.

## Done when

- Selecting a normal assistant or user prose line that currently copies as `  │▸ text` instead
  copies as `text` with no leading UI-only spaces, `│`, `┃`, `▸`, or equivalent gutter/caret
  glyphs.
- Message content still has comfortable visual padding/margins in the TUI; the fix must not make the
  transcript feel cramped against the terminal edge.
- Highlighting ordinary message text no longer visibly includes leading gutter glyphs before the text.
- List, blockquote, code-block, and table lines no longer include message-level gutter glyphs before
  their content when selected. Renderer-specific markdown fidelity beyond that is not required here.
- Partial selections (single line, first/last line, mid-line column bounds) still clip correctly and
  do not emit partial decoration glyphs.
- Selection UX is unchanged: drag, release-to-copy, and toast still work; `/copy` is unaffected.
- `cargo check -p opencode-tui` and `cargo test -p opencode-tui` pass, with tests asserting that
  copied selection text omits leading gutter/chrome glyphs.

## Recommended verification

- `ort-build`, then `ort`; ask for a reply containing prose plus at least one bullet list item.
- Select a line that previously looked like/copy-pasted as `  │▸ I have enough...`; paste into
  an editor and confirm it starts directly with `I have enough...`.
- Select a user message and confirm no `┃`, `│`, `▸`, or leading gutter padding is copied.
- Select a single line and a partial multi-line span; confirm clipping is correct and no stray glyphs
  appear at the boundaries.
- Run `/copy` and confirm the full-transcript markdown output is unchanged.
- Add tests around the render/capture/selection path, or the smallest reachable helper, proving
  leading gutter/chrome glyphs are absent from copied selected text.

## Product decisions

- The TUI should keep enough padding/margin that messages remain readable and visually separated.
- Do not rely on visible vertical bars, carets, or gutter glyphs as selectable message prefixes.
- The selection highlight should not suggest that UI chrome is part of the answer text.
- For this card, clean prose copy is more important than full direct-markdown reconstruction.

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
- Implementation direction:
  1. Identify the active message render path for assistant and user messages.
  2. Replace leading selectable gutter glyphs (`│`, `┃`, `▸`, and equivalents) with layout padding or
     margins that preserve visual spacing without copying visible chrome.
  3. If a marker is still needed visually, render it outside the selectable/captured text path if the
     existing architecture supports that without a large rewrite; otherwise omit the marker.
  4. Avoid source-aware markdown mapping in this card unless a minimal render/layout change cannot
     remove the copied leading chrome.
- `/copy`/`build_session_transcript` already demonstrates the desired markdown shape and should be
  left unchanged and can be used as a reference for clean prose output.
- Confirm which session render path is actually used (session vs. session_message vs. message) before
  wiring the chosen approach, since decoration is added in more than one place.

## Dev Notes

- Branch: `bug/BUG-024-clean-selection-gutters`
- PR: https://github.com/cchris-p/opencode-modded-rust/pull/59
- Change: assistant text rendering now uses two-column padding instead of the visible `▸ ` marker;
  user message rendering now uses two-column padding instead of the visible `┃ ` gutter.
- Change: mouse-selection copy runs selected screen text through a small normalizer that removes known
  leading layout chrome/padding, including the reported `  │▸ text` shape, while preserving non-layout
  indentation such as four-space code indentation.
- Decision: kept `/copy` and transcript export unchanged; this fix is limited to mouse-selection copy
  and visible selectable prefixes.
- Verification: `cargo fmt -p opencode-tui -- --check`; `cargo check -p opencode-tui`; `cargo test -p
  opencode-tui -- --test-threads=1` (49 passed). Added unit coverage for render prefixes and copy
  normalization.

## Merge Closeout - 2026-09-21

- Merged PR #59 into `development` as merge commit `ff7b5d5`.
- Remote PR branch `bug/BUG-024-clean-selection-gutters` was deleted by the GitHub merge flow and the
  stale remote-tracking ref was pruned locally.
- Card remains in `qa` for post-merge validation; no QA report has been recorded yet.

## QA Report - 2026-09-21

- User live-tested mouse-selection copy after the PR #59 merge and reported the selected text "pastes
  much [better] now": leading gutter/caret chrome is gone, so the reported `  │▸ text` case copies as
  clean prose.
- Follow-up feedback: markdown list bullets still rendered as `• `; user asked for `- ` instead so the
  copied text matches markdown source.
- Fix applied directly on `development` (no PR): assistant/user markdown list items now render `- `
  instead of `• ` (`crates/opencode-tui/src/components/markdown/renderer.rs`). Ordered list prefixes
  (`N. `) and task-list markers (`- [x] `) are unchanged.
- `cargo fmt -p opencode-tui` and `cargo check -p opencode-tui` pass.

## Closeout - 2026-09-21

- Marked `done` per user: selection copy is clean and the bullet glyph now matches markdown source.
- User will reopen a new card if further copy-fidelity work is needed.

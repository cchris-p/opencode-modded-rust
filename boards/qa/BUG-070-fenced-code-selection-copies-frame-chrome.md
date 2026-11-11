---
id: "BUG-070"
title: "Selecting a fenced code block copies frame chrome and wraps instead of raw code"
priority: "P2"
type: "bug"
area: "BUG"
spec: ""
status: "qa"
created: "2026-11-11"
updated: "2026-11-11"
---

# Selecting a fenced code block copies frame chrome and wraps instead of raw code

## Summary

Mouse-selection copy of a fenced code block does not yield the raw code a user needs to paste into
a terminal. Selecting a block such as:

```bash
sudo sh -c "zgrep -h 'pharos/add_funds' /var/log/nginx/api_external-access.log*" \
  | grep '"request": "POST' \
  | grep -o '"body_bytes_sent": "[0-9]*"' | sort | uniq -c
```

can put the rendered frame on the clipboard (`╭ bash ────`, the `│ ` gutter, `╰───`), strips
authored indentation, and inserts newlines where the TUI soft-wraps a long code line. The result is
not directly paste-runnable.

The selection highlight also covers the frame glyphs, so the user cannot tell which characters are
code and which are TUI chrome. Selecting a block should copy exactly the fenced source lines, while
the frame stays visible but is clearly not part of the selection.

## Reported behavior

- Copying a fenced snippet from a session returns frame decoration or otherwise non-source text.
- Long code lines are captured as multiple screen rows, so a soft-wrap becomes a hard newline in the
  copied command.
- Leading indentation that exists in the source is not preserved because the screen-cleanup pass
  strips two-column alignment padding.
- The highlight covers the fence so the selected region looks like it includes `╭`/`╰` chrome.

Real example session (ORT): `ses_51e463872e7a4864bf9aa857ada53227`, workspace
`/Users/cchrisleepyles/apps/ivy-notes`. The `pharos/add_funds` command above is the failing copy.

## Root cause

Selection copy scrapes the rendered screen buffer, not the markdown source.

- `crates/opencode-tui/src/components/markdown/renderer.rs:459-510` renders each fenced block as a
  header line (`╭ <lang> ────`), body lines prefixed with `│ `, and a footer (`╰───`).
- `crates/opencode-tui/src/components/session_text.rs:10-19` prepends two padding spaces to every
  markdown line; `paint_block_lines` then wraps block lines to the content width
  (`crates/opencode-tui/src/components/session.rs:1925-2032`). Wrapped continuation rows do not
  carry the `│ ` gutter.
- `crates/opencode-tui/src/app/app.rs:2198-2233` (`copy_selection`) reads `self.screen_lines` and
  `Selection::get_selected_text`. `clean_selection_text`/`clean_selection_line`
  (`app.rs:4551-4575`) only strip leading `│`/`┃`/`▸` and two-space padding; they do not understand
  fenced blocks or unwrap soft-wrapped rows.

BUG-024 fixed prose gutters and explicitly deferred source-aware fence copy; this card is that
deferred follow-up.

## Scope

- Transcript selection copy of fenced code blocks must emit the raw source lines of the block:
  no `╭`/`╰` frame, no `│ ` gutter, no wrap-induced newlines, and authored indentation preserved.
- Non-code transcript text keeps the existing selection-copy behavior and column clipping.
- Code-fence frame rows (header/footer) must not be highlighted as part of the selection, so it is
  visually clear they are not copied.
- Keep the existing selection UX: drag, release-to-copy, auto-scroll (FEAT-072), and toast.
- Keep `/copy` (full transcript), session export, and `TranscriptOptions` unchanged.
- Follow the existing content-line coordinate model used by FEAT-072 so selections spanning multiple
  viewports still work.

## Non-goals

- Source-aware reconstruction for arbitrary partial prose selections.
- Copying markdown table box drawing as source markdown.
- Adding a new keybinding, copy button, or dialog.
- Changing how code blocks look beyond de-emphasizing the frame and excluding it from highlight.
- Reworking the renderer to store per-span source offsets for every markdown construct.

## Done when

- Selecting across a fenced block in the transcript copies exactly the block's source lines, with
  authored indentation and no frame/gutter characters.
- A soft-wrapped code line copies as one source line (no inserted newline), including when the
  selection auto-scrolled across viewports.
- Selecting ordinary prose still clips by column and does not gain blank lines or chrome.
- The header/footer frame rows are not inverted by the selection highlight; code body rows are.
- Selecting a code block without including its frame (starting on a body row) still copies raw
  source for the touched lines.
- `cargo fmt -p opencode-tui -- --check`, `cargo check -p opencode-tui`, and
  `cargo test -p opencode-tui` pass, with unit tests for the raw-code copy plan.

## Recommended verification

- `ort-build`, then `ort`; ask for a reply containing the `pharos/add_funds` bash fence.
- Select the whole block and paste into a terminal/editor: output must match the source block exactly
  (including the two-space continuation indentation, no `╭`/`│`/`╰`).
- Make the terminal narrow enough to soft-wrap the long first line, repeat the selection, and confirm
  the pasted command stays one line where the source has one line.
- Select prose around the block and confirm prose copy is unchanged.
- Start a selection in the middle of a wrapped code line and confirm a raw, complete source line is
  copied rather than a truncated fragment.
- Confirm the frame rows stay visually unhighlighted while dragging over them.

## Implementation notes

- Introduce a `CopyRow` plan in `ui/selection.rs`: `Normal`, `CodeChrome`, `CodeStart(String)`, and
  `CodeContinuation`.
- `MarkdownRenderer::render_markdown` now returns `RenderedMarkdown { lines, code_blocks }`, so the
  raw fence source is captured in the same pulldown-cmark pass used for rendering (no second parse).
  `render_text_part_with_code` and `render_user_message_with_code` forward those block lines.
- In `render_message_body` (`session.rs:1336`), accumulate `code_blocks` while rendering text parts,
  then build a parallel `copy_rows` vector by scanning rendered rows for `╭`/`╰` frame rows and body
  rows and pairing body rows with the captured source lines. Wrapping therefore never affects the
  copied text.
- Thread `body.copy_rows` into the windowed transcript render in lockstep with the existing
  `line_to_message` vector, expose a per-content-line lookup on `SessionView`, and accumulate it in
  App next to `transcript_lines` (`merge_transcript_lines`).
- In `copy_selection`, when a transcript `ContentSelection` exists, compose from content lines:
  skip `CodeChrome` and `CodeContinuation`, emit `CodeStart` verbatim, and use the current
  screen-text cleanup only for `Normal` rows.
- Exclude `CodeChrome` content lines from the app selection-highlight loop.
- Dim the fence header language label to a muted color so the frame reads as chrome; keep the body
  and gutter colors as they are.

## Dev Notes

- Branch: `bug/BUG-070-raw-code-fence-copy`.
- PR: https://github.com/cchris-p/opencode-modded-rust/pull/179 (targets `development`).
- `MarkdownRenderer::render_markdown` now returns `RenderedMarkdown { lines, code_blocks }` so fence
  source is captured in the existing parse pass; existing `to_lines` callers are unchanged.
- `render_text_part_with_code` and `render_user_message_with_code` expose the captured fence lines;
  the old wrapper behavior is gone because no production caller needed the line-only form.
- `copy_rows_for_message` classifies rendered rows into `Normal`, `CodeChrome`, `CodeStart(raw)`, and
  `CodeContinuation`. Header/footer and the renderer's blank block padding are chrome; wrapped body
  rows are continuations; the first body row of each source line carries the exact source text.
- `SessionView` exposes `copy_row(content_line)` and `is_code_chrome_screen_row(row)`; App accumulates
  `transcript_copy_rows` alongside `transcript_lines` in `merge_transcript_lines`.
- Transcript copy now always composes from content coordinates (not only after auto-scroll): normal
  rows keep column clipping + `clean_selection_line`, code rows emit raw source, chrome and
  continuations are skipped, and a selection that starts inside a wrap includes the parent source
  line once. Mouse `Up` also pins the content-space endpoint to the release point.
- Selection highlight skips `CodeChrome` rows so the frame stays visible but is clearly outside the
  selection. The fence header language label now uses `text_muted` to read as chrome.
- Tests: 3 app-level `compose_transcript_selection` tests (raw code + chrome skip, continuation
  start, prose clipping), 3 session-level copy-plan tests (indentation preserved, wrap stays one
  line, prose classification), and a `TestBackend` session test proving the painted window exposes
  the right copy rows and chrome rows.
- Verification: `cargo fmt -p opencode-tui -- --check`, `cargo check -p opencode-tui`, and
  `cargo test -p opencode-tui -- --test-threads=1` (201 passed). Two pre-existing prompt tests fail
  independently of this change: `tab_autocomplete_uses_first_candidate` fails on a clean `development`
  tree, and `utf8_backspace_delete_and_cursor_are_char_safe` then fails from the poisoned env lock.

## Related Items

- `BUG-024` mouse-selection copy captures selectable gutter decoration - this card implements its
  defered code-fence follow-up.
- `FEAT-072` auto-scroll during drag selection - supplies the content-line selection model reused
  here.

## Notes

- Real session with the failing copy: `ses_51e463872e7a4864bf9aa857ada53227` (ORT), workspace
  `/Users/cchrisleepyles/apps/ivy-notes`.
- Board repo rules apply: this repository is the product repo, so implementation uses a feature
  branch and a PR into `development`.

## Merge Closeout - 2026-11-11

- Merged PR #179 into `development` as merge commit `4ba3ac2`.
- Deleted the merged remote branch `bug/BUG-070-raw-code-fence-copy` and the local branch.
- Local checkout is on `development` and current with `origin/development`.
- CI note: `fmt`, `check`, `clippy`, and `test` passed on the PR. Both `native` jobs failed in
  dependency install before compiling anything (Ubuntu: `add-apt-repository` network timeout to
  `api.launchpad.net`; macOS job also failed during native setup), so no code/test failure was
  involved.
- Card remains in `qa`; no QA report has been recorded yet.

## QA Failure - 2026-11-11

- Reported after merge: copying a fenced code block with Ctrl+C pasted only empty lines.
- Reproduced with the live ORT binary (`3bceef8...`, which includes PR #179) against session
  `ses_84126346ec064c1a9db3ce7e1074bf98` in `cass`: the assistant message has two bash fences
  (single-line `python manage.py export_appointments_csv`, then the multi-line
  `--start/--end/--output` command). Copying the second block yielded blank lines.
- Root cause: `copy_rows_for_message` classified rendered rows with `text.trim_start()`. Every
  rendered row is right-padded to the pane width, so `is_code_frame_footer` never matched `╰───`
  plus trailing padding. The code region never closed; the second `╭` header was treated as a
  continuation and its body rows were paired past the end of the first block's source, producing
  `CodeStart("")`.
- Existing copy-plan tests only covered single-block messages, so this slipped through.

## Follow-up Fix - 2026-11-11

- Branch: `bug/BUG-070-fence-footer-padding`.
- PR: https://github.com/cchris-p/opencode-modded-rust/pull/188
- Change: classify with `text.trim()` in `copy_rows_for_message` so a padded footer is recognized
  and each fence opens and closes its own region.
- Tests: `fenced_code_copy_plan_handles_multiple_blocks_in_one_message` covers two fences at 200
  and 60 columns; `prose_after_a_fenced_block_stays_normal` proves the region closes before
  following prose.
- Verification: `cargo fmt -p opencode-tui -- --check`, `cargo check -p opencode-tui`, and both new
  tests pass. Full suite: 210 passed, 2 pre-existing prompt failures
  (`tab_autocomplete_uses_first_candidate` and the poisoned-env
  `utf8_backspace_delete_and_cursor_are_char_safe`) unchanged from clean `development`.
- Card returned to `qa`; post-fix QA report recorded below.

## QA Report - 2026-11-11

- Fixed binary: branch `bug/BUG-070-fence-footer-padding` at `4eafe18`, built with
  `cargo build -p opencode-cli --no-default-features` (the native scopemux sources are not present
  in this checkout).
- Fixture: isolated server on `127.0.0.1:3299` with `OPENCODE_DATA_DIR=/tmp/qa070`, session
  `ses_924dbe3f078e4b09aa5baf84b9bf6c69`, seeded with one assistant message containing two bash
  fences.
- A/B in the real TUI via tmux, selecting the second block and pressing Ctrl+C:
  - pre-fix binary (`3bceef8...`): clipboard was exactly `\n\n\n` (the reported empty lines).
  - fixed binary: clipboard was the four raw source lines, preserving `\` continuations and the
    two-space continuation indentation.
- First block still copies correctly; `cargo fmt -p opencode-tui -- --check`, `cargo check -p
  opencode-tui`, both new regression tests, and the full suite pass (210 passed, 2 pre-existing
  prompt failures unchanged from clean `development`).
- QA passes for the reported failure; card remains in `qa` awaiting merge closeout.

---
id: "FEAT-055"
title: "Unify and improve tool / script display in the session transcript"
priority: "P3"
type: "feature"
area: "FEAT"
spec: ""
status: "qa"
created: "2026-09-23"
---

# Unify and improve tool / script display in the session transcript

## Summary

Tool calls are rendered by two parallel, partly-unused stacks, and the live session path bypasses the
block painting and wrapping that every other message part receives. As a result tool/script blocks
look different from the rest of the transcript, long output is hard-truncated, and the richer per-tool
views that already exist in the crate are never shown. Consolidate to one render path, route tool lines
through the block pipeline, and surface the per-tool views.

## Why this exists

The session transcript has an established block treatment — background, `│` gutter, width-aware
wrapping — applied to user text, assistant text, file/image parts, reasoning, and the footer. Tool
calls are the exception: they are appended raw and use their own preview/truncation rules, so they
break alignment and hide most of the output.

## Current behavior and evidence

- Duplicate render stacks. The live session path is `session_tool::render_tool_call`
  (`crates/opencode-tui/src/components/session_tool.rs:105-327`), invoked through
  `render_tool_call_part`/`append_rendered_tool_call`
  (`crates/opencode-tui/src/components/session.rs:1260-1310`). A second stack —
  `MessageView::render_tool_call` (`crates/opencode-tui/src/components/message.rs:133-270`),
  `ToolCallView`/`BashToolView`/`ReadToolView`/`WriteToolView`/`ToolResultView`
  (`crates/opencode-tui/src/components/tool_call.rs`), and `tool_views.rs` — is exported
  (`crates/opencode-tui/src/components/mod.rs:41`, `:55-59`) but does not feed the live transcript.
- Tool lines skip block painting/wrapping. `append_rendered_tool_call`
  (`crates/opencode-tui/src/components/session.rs:1282-1310`) pushes `rendered.lines` straight into the
  transcript, whereas text/file/image/footer parts are passed through `paint_block_lines`
  (`session.rs:577`, `:618`, `:671`, `:709`, `:740`, `:900`, `:921`, `:946`) which applies
  `wrap_block_line` + background + gutter (`session.rs:1312-1419`).
- Output is hard-truncated per line. `format_preview_line(line, 96)`
  (`crates/opencode-tui/src/components/session_tool.rs:484-491`) is applied to every output line
  (`session_tool.rs:205`, `:223`, `:248`), so anything past 96 columns is lost rather than wrapped.
  `tool_argument_preview` truncates the fallback preview at 84 columns (`session_tool.rs:437`).
- Inconsistent preview budgets. Bash previews 10 output lines, other block tools preview 6
  (`crates/opencode-tui/src/components/session_tool.rs:136-140`); error output collapses to 3 or 1
  (`session_tool.rs:143-151`).
- Richer, unused views already exist: `BashToolView` models command + output + exit code
  (`crates/opencode-tui/src/components/tool_call.rs:190-263`), and `tool_views.rs` defines
  edit/apply_patch/read/write/glob/grep/list/webfetch/websearch/skill/task/todowrite views, none of
  which are wired into the transcript.
- The module-level `ToolRenderMode` classification
  (`crates/opencode-tui/src/components/tool_call.rs:26-36`) that decides inline vs block also does not
  match `is_block_tool` (`session_tool.rs:78-91`), so the two stacks disagree on what is a block tool.

## Scope

- Collapse to a single tool-call render path used by the transcript; delete or re-home the unused
  stack (`message.rs` tool rendering, `tool_call.rs` view types, `tool_views.rs`) so there is one
  implementation.
- Route tool-call lines through the same block pipeline as other parts (`paint_block_lines`), so they
  get the gutter, background, and width-aware wrapping.
- Stop hard-truncating output at a fixed column; wrap to the available content width instead, and keep
  explicit expand/collapse for long output.
- Adopt the per-tool views for edit/apply_patch/read/write/glob/grep/todowrite and friends, or document
  why a given tool stays generic.
- Unify the inline-vs-block classification (`ToolRenderMode` vs `is_block_tool`) and the preview
  budgets between the two paths.

## Non-goals

- Changing what tools return or how results are stored.
- Streaming/live-updating command output or PTY/ANSI handling (see FEAT-056 for the bash terminal
  rendering).
- Tool-call compaction toggle behavior (`FEAT-028`), which stays as-is.
- Model/part transport changes.

## Done when

- Exactly one tool-call renderer serves the transcript; no exported-but-unused tool view stack remains.
- Tool blocks get the same gutter/background/wrapping as other message parts and align with them.
- Long tool output wraps to width and is not truncated at 96 columns; expand/collapse still works.
- `edit`/`apply_patch`, `read`, `write`, `glob`/`grep`, and `todowrite` show a purpose-built view (or a
  documented generic fallback).
- `cargo check -p opencode-tui` and `cargo test -p opencode-tui` pass, with tests covering the unified
  render path and width wrapping.

## Recommended verification

- `ort-build`, then `ort`; run a read, write, edit/apply_patch, glob, grep, and todowrite and confirm
  each renders a coherent block that aligns with surrounding text.
- Produce a tool result with lines longer than the terminal width and confirm it wraps instead of
  being cut at 96 columns.
- Resize the terminal narrow and wide and confirm tool blocks reflow like other parts.
- Toggle `/tool-details` and `/tool-calls` and confirm collapse/expand and compaction still behave.
- Confirm no dead tool view code remains (`grep` for `ToolCallView`, `BashToolView`, `tool_views`).

## Related Items

- `FEAT-056` Bash command display: render terminal-style block with text wrap
- `FEAT-028` Add a hide-tool-calls toggle that compacts runs to a tool-call count
- `FEAT-054` TUI color scheme consistency across all surfaces
- `FEAT-053` Make TUI display-toggle defaults configurable from opencode.json

## Notes

- Relevant files: `crates/opencode-tui/src/components/session_tool.rs`,
  `crates/opencode-tui/src/components/session.rs`,
  `crates/opencode-tui/src/components/message.rs`,
  `crates/opencode-tui/src/components/tool_call.rs`,
  `crates/opencode-tui/src/components/tool_views.rs`,
  `crates/opencode-tui/src/components/mod.rs`.
- Confirmed 2026-09-23: `MessageView`/`ToolCallView`/`BashToolView`/`tool_views` have no non-test
  caller (only definitions and `components/mod.rs` re-exports; none of the modules have tests), so the
  stack is deleted and its per-tool semantics ported into the live `session_tool.rs` renderer.
- `wrap_block_line`/`wrap_spans` (`crates/opencode-tui/src/components/session.rs:1453-1485`) is
  upgraded to word-aware wrapping with a hard-break fallback for parity with the markdown
  `Paragraph::wrap`.

## Dev Notes - 2026-09-23

- Tool render output now flows through `paint_block_lines` in `append_rendered_tool_call`
  (`session.rs`), so tool blocks get the same gutter, background, padding, and wrapping as text/
  file/image/footer parts. Toggle hit line indices are computed from the painted line count.
- Removed the fixed 96-column truncation (`format_preview_line(line, 96)`) from output and inline
  summaries; rendering wraps to the content width instead.
- Upgraded `wrap_spans` to word-aware wrapping with a hard-break fallback for over-width tokens.
- Folded the dead `ToolRenderMode` classification into `is_block_tool` as the single inline-vs-block
  authority; write/edit/apply_patch/task/todowrite/question/glob/grep/list now always render blocks.
- Ported per-tool semantics into the live renderer: read/write/edit/apply_patch/todowrite detail
  lines, richer argument previews, and glob/grep/list match counts.
- Deleted the unused `components/message.rs`, `components/tool_call.rs`, and
  `components/tool_views.rs` stacks and their `mod.rs` re-exports.
- Tests: `wrap_spans_moves_whole_words_to_the_next_line`,
  `wrap_spans_hard_breaks_tokens_wider_than_the_line`, `wrap_spans_preserves_explicit_newlines`,
  `todowrite_renders_status_icons_and_content`, `edit_renders_old_and_new_lines`,
  `glob_reports_match_count_and_pattern`, `long_output_lines_are_not_truncated_at_96_columns`.
- Verification: `cargo test -p opencode-tui` green; `grep` confirms no `ToolCallView`/`BashToolView`/
  `tool_views` references remain.
- Branch `feature/tui-display-cluster`; awaiting local QA on the open PR.

### Merge Closeout - 2026-09-23

- Merged into `development` via PR #103 (merge commit `21d7d55`).
- Feature branch `feature/tui-display-cluster` deleted (remote and local); local checkout returned to `development`.
- Remains in `qa` pending a QA report or explicit completion.

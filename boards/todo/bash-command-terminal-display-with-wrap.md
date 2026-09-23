---
id: "FEAT-056"
title: "Bash command display: render a terminal block with text wrap"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "todo"
predecessors: "FEAT-055"
created: "2026-09-23"
updated: "2026-09-23"
---

# Bash command display: render a terminal block with text wrap

## Summary

Bash tool calls should read like a terminal: a `$` prompt line with the command that soft-wraps to the
available width, followed by the command output in a framed region that also wraps, plus the exit
status. Today the command is a single unwrapped span, long commands and output are clipped or
hard-truncated, and the transcript paragraph does not wrap, so wide commands are cut off at the edge.

## Why this exists

`bash`/`shell` is the highest-volume tool for the daily-driver workflow, and its output is the primary
way users inspect what happened. The current rendering loses information (fixed 96-column truncation),
misaligns with the rest of the transcript, and gives no terminal affordance, so users cannot reliably
read long commands or their output.

## Current behavior and evidence

- The command is rendered as one unwrapped span: `session_tool.rs` pushes `$ ` then the full
  `shell_command_text(arguments)` onto a single `Line`
  (`crates/opencode-tui/src/components/session_tool.rs:164-169`), with no wrapping or truncation guard.
- That line is not block-wrapped. `append_rendered_tool_call`
  (`crates/opencode-tui/src/components/session.rs:1282-1310`) appends tool lines directly, unlike
  text/file/image/footer parts which go through `paint_block_lines`/`wrap_block_line`
  (`session.rs:1312-1419`, callers at `:900-951`).
- The transcript `Paragraph` has no `.wrap(...)`
  (`crates/opencode-tui/src/components/session.rs:987-995`), so anything wider than the viewport is
  simply clipped at the right edge rather than wrapping.
- Output lines are hard-truncated at 96 columns via `format_preview_line(line, 96)`
  (`crates/opencode-tui/src/components/session_tool.rs:484-491`, applied at `:205`, `:223`, `:248`).
- Only the first 10 output lines are previewed (`session_tool.rs:136-140`), with collapse to 3/1 for
  errors (`:143-151`); there is no terminal-style header, prompt gutter, or exit-code display.
- The wrapping helper that exists is greedy per-character, not word-aware, so a long unbroken token
  still cannot wrap gracefully even if routed through it
  (`crates/opencode-tui/src/components/session.rs:1453-1485`).
- A richer `BashToolView` already models command + output + exit code
  (`crates/opencode-tui/src/components/tool_call.rs:190-263`) but is not used by the live transcript.

## Scope

- Render bash/shell tool calls as a distinct terminal-style block: a `$` prompt line, the command
  wrapped to width, then output in a framed/inset region with a consistent gutter.
- Wrap both the command and the output to the content width instead of truncating at a fixed 96
  columns; use word-aware wrapping with a hard-break fallback for long unbroken tokens.
- Show the exit status (and running state) for the command, using the existing `BashToolView` semantics
  as the reference or wiring it in.
- Keep expand/collapse for long output and keep the preview-on-collapse behavior, but make the
  collapsed preview itself wrap.
- Ensure the block inherits the active theme tokens (gutter/border/background) rather than fixed
  colors.

## Non-goals

- PTY allocation, live streaming output, or ANSI/SGR parsing.
- Changing how bash commands are executed, sandboxed, or permissioned.
- Rendering a real scrollable terminal emulator or supporting interactive programs.
- Other non-bash tool layouts (tracked under FEAT-055).

## Done when

- A bash command longer than the terminal width wraps onto multiple `$`-prefixed lines instead of
  being clipped.
- Output lines longer than the width wrap instead of being truncated at 96 columns.
- The exit code / failed state is visible on the block.
- Collapsed and expanded states both wrap correctly, and collapse still summarizes hidden lines.
- The block uses theme tokens and aligns with the surrounding transcript.
- `cargo check -p opencode-tui` and `cargo test -p opencode-tui` pass, with a test asserting a long
  command and a long output line wrap to the provided width.

## Recommended verification

- `ort-build`, then `ort`; run a bash command well over the terminal width (e.g. a long `echo` or
  `grep` with a long pattern) and confirm the command wraps under the `$` prompt.
- Produce output with very long lines and confirm they wrap rather than stop at 96 characters.
- Resize the terminal from narrow to wide and confirm the block reflows.
- Run a failing command and confirm the exit/failed state renders.
- Collapse and expand a long output and confirm the preview wraps and totals stay correct.
- Add unit tests for command wrapping, output wrapping, and long-token hard breaks.

## Related Items

- `FEAT-055` Unify and improve tool / script display in the session transcript
- `FEAT-054` TUI color scheme consistency across all surfaces
- `FEAT-028` Add a hide-tool-calls toggle that compacts runs to a tool-call count
- `BUG-018` Prompt input cursor renders out of place (same greedy-vs-word wrap mismatch class)

## Notes

- Relevant files: `crates/opencode-tui/src/components/session_tool.rs`,
  `crates/opencode-tui/src/components/session.rs`,
  `crates/opencode-tui/src/components/tool_call.rs` (`BashToolView`).
- `wrap_spans` (`session.rs:1453-1485`) is upgraded to word-aware wrapping with a hard-break fallback
  so paths and flags do not split mid-token when they fit on the next line.
- Coordinate with FEAT-055 so the bash terminal block is the concrete instance of the unified tool
  render path rather than yet another special case. `BashToolView` is reference semantics only: the
  unused `tool_call.rs` widget is deleted under FEAT-055. Implement after the FEAT-055 core.

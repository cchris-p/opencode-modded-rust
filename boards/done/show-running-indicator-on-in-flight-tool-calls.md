---
id: "FEAT-075"
title: "Show an animated running indicator on in-flight tool calls"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "done"
created: "2026-11-11"
---

# Show an animated running indicator on in-flight tool calls

## Summary

While a tool call is actually executing, its transcript row should carry an obvious animated loading
indicator (and a `running…` label for non-bash tools) so the operator can tell that a `grep`, `glob`,
read, or polling command is still in flight instead of assuming the session has stalled. Today the
only live signal is a static state glyph on the tool row and the separate prompt spinner at the
bottom of the screen.

## Why this exists

Operator report 2026-11-11: it is unclear whether a tool call such as a polling command or a `grep`
is still running. The tool row can read `◐ grep …` or `◯ grep …` with no motion, and for the common
tool-call path the row does not even reach the `Running` state, so the only animation is the prompt
spinner far from the call.

## Current behavior and evidence

- `styles_for_state` gives `Pending` a static `◯` and `Running` a static `◐`
  (`crates/opencode-tui/src/components/session_tool.rs:676-713`). Neither animates.
- Only the bash block gets a status line; `bash_status` returns `running…` for `ToolState::Running`
  (`crates/opencode-tui/src/components/session_tool.rs:585-599`). Non-bash block tools such as
  `grep`, `glob`, `list`, and reads show no running label.
- The collapsed tool-run summary also renders a static state glyph
  (`render_tool_run_summary`, `crates/opencode-tui/src/components/session_tool.rs:20-54`).
- The `Running` state is gated on `msg.finish.is_none()`:
  `is_active_assistant = ... && msg.finish.is_none() && msg.error.is_none()`
  (`crates/opencode-tui/src/components/session.rs:1430-1444`). But a tool-call assistant message is
  finalized with `finish_reason: "tool-calls"` **before** the tools execute
  (`crates/opencode-session/src/prompt.rs:1565-1571`, and the run loop only continues when the finish
  reason is `tool-calls` at `prompt.rs:1765`). So during real tool execution the row frequently
  renders as `Pending` (`◯`), not `Running`.
- A working animated spinner already exists in the prompt (`components/spinner.rs`,
  `BRAILLE_FRAMES`, `Prompt::tick_spinner`), and the app already ticks it while the session is busy
  (`crates/opencode-tui/src/app/app.rs:990-995`). The transcript does not reuse that activity phase.

## Scope

- Detect an in-flight tool call from live session state as well as the missing result: when the last
  assistant message holds unresolved tool calls and the session status is not `Idle`, treat the first
  unresolved call as `Running` regardless of the message `finish` value (covers `tool-calls`,
  `tool_calls`, and a still-unset finish).
- Animate the state glyph on a `Running` tool row with the existing braille frame set, driven by the
  same activity phase as the prompt spinner, so the indicator moves next to the tool call.
- Add a `running…` label to non-bash block tool rows and inline tool rows while running; keep the
  existing bash `running…` status line.
- Animate the state glyph of a collapsed tool-run summary while any call in the run is in flight, and
  show a `running…` label there too.
- Respect the existing `animations_enabled` preference: when animations are disabled, fall back to
  the current static `◐` (and still show the `running…` label).
- Keep completed, failed, denied, and pending rendering unchanged; this is a render-time-only
  change and does not alter stored messages or tool execution.

## Non-goals

- Streaming live tool output or PTY/ANSI handling (FEAT-056 scope).
- Changing how tool calls and results are stored or transported.
- Changing the `/tool-calls`, `/tools`, or `show_tool_details` toggles.
- Changing the prompt spinner itself.

## Done when

- During a real tool execution (for example `grep`, `glob`, a read, or a bash polling command) the
  tool row shows an animated loading glyph next to the call, and non-bash tools also read
  `running…`.
- The running state is selected when the tool-call message has `finish_reason` set to `tool-calls`
  (or `tool_calls`) before the result arrives.
- The collapsed tool-run summary animates and reads `running…` while any call in the run is
  in flight.
- With `animations_enabled` off, the row shows the static `◐` plus `running…` and does not animate.
- Completed and failed tool calls render exactly as before.
- Unit tests cover the animated running row, the animations-disabled fallback, and the
  `tool-calls` finish-reason detection; `cargo test -p opencode-tui` and `cargo check -p
  opencode-tui` pass.

## Recommended verification

- `cargo test -p opencode-tui`.
- `ort-build`, then `ort`; run a slow `grep` or polling bash command and confirm the tool row shows
  an animated indicator and `running…` while it executes, then settles to the completed state.
- Toggle animations off and confirm the row shows a static `◐` and `running…` without motion.
- Hide tool calls (`/tool-calls`) during a run and confirm the collapsed count line animates and
  says `running…`.
- Run several tool calls in a row and confirm only the in-flight call animates.

## Related Items

- `FEAT-055` Unify and improve tool / script display in the session transcript (single tool renderer)
- `FEAT-056` Bash command display: render a terminal block with text wrap (bash running status)
- `FEAT-028` Add a hide-tool-calls toggle that compacts runs to a tool-call count (summary rendering)
- `FEAT-063` Collapse consecutive tool-call runs into one count when tool calls are hidden (archived
  follow-up to FEAT-028)

## Notes

- Likely files: `crates/opencode-tui/src/components/session_tool.rs`,
  `crates/opencode-tui/src/components/session.rs`, and
  `crates/opencode-tui/src/components/spinner.rs` (expose the braille frame set).
- The frame phase can be sourced from the existing prompt spinner accessor and passed through
  `MessageRenderCtx`; do not put it in the message layout signature or it will invalidate the
  per-message layout cache every frame.

## Dev Notes

- Exposed `spinner::braille_frame(index)` and `KnightRiderSpinner::frame_index()`; added
  `Prompt::activity_frame()` so transcript indicators advance on the same tick as the prompt spinner.
- `session_tool.rs` now replaces the static `◐` with an animated braille glyph for `ToolState::Running`
  (static `◐` when animations are disabled), and appends `running…` to non-bash block rows, inline
  rows, and collapsed tool-run summaries. Bash keeps its existing `running…` status line and gains the
  animated header glyph.
- `session.rs` threads `activity_frame` and `animations_enabled` through `MessageRenderCtx` (not the
  layout signature, so the per-message layout cache stays valid) and selects the running state for the
  last assistant message whenever the session is busy and the unresolved tool-call step carries a
  tool-call finish (`finish_allows_running_tool` accepts `tool-calls`/`tool_calls`/unset). This fixes
  the common path where the tool-call message is already finalized with `finish_reason: "tool-calls"`
  before the tools run.
- Tests added: `running_grep_row_animates_and_labels_running`,
  `running_tool_row_uses_static_glyph_when_animations_disabled`,
  `completed_tool_row_is_not_labelled_running`,
  `running_tool_run_summary_animates_and_labels_running`,
  `tool_call_finish_reasons_keep_running_state_live`, and the end-to-end render test
  `in_flight_tool_call_with_tool_calls_finish_renders_running_indicator` (drives a `TestBackend`
  session render and asserts the animated glyph, `running…`, and the visible `grep` row).
- Verification: `cargo test -p opencode-tui -- --test-threads=1` (209 passed),
  `cargo check -p opencode-tui`, and `cargo build -p opencode-cli` all pass.

## QA Report - 2026-11-11

- Commit: `6903064` (`feat(tui): animate in-flight tool call running indicator (FEAT-075)`).
- Binary: `target/debug/opencode-modded-rust`, built on the feature branch with
  `cargo build -p opencode-cli` (finished in 1m34s).
- `cargo test -p opencode-tui -- --test-threads=1`: **209 passed, 0 failed**. The parallel run flaked
  on two pre-existing prompt tests that share process env/cwd; both pass serially and are unrelated to
  this card.
- `cargo check -p opencode-tui`: pass. `cargo clippy -p opencode-tui --all-targets`: no new warnings in
  the touched files (existing warnings remain in other modules).
- Deterministic product-path render QA: `in_flight_tool_call_with_tool_calls_finish_renders_running_indicator`
  drives a `TestBackend` session render with `SessionStatus::Running`, an assistant message whose
  `finish` is `Some("tool-calls")`, and an unresolved `grep` call. The rendered buffer contains the
  animated glyph (`⠋`), `running…`, and the `grep` row, confirming the old `finish.is_none()` gate no
  longer hides the in-flight state.
- Animation/fallback/collapsed-summary QA is covered by
  `running_grep_row_animates_and_labels_running`,
  `running_tool_row_uses_static_glyph_when_animations_disabled`,
  `completed_tool_row_is_not_labelled_running`, and
  `running_tool_run_summary_animates_and_labels_running`.
- Result: **PASS**.

### Merge Closeout - 2026-11-11

- Merged into `development` via PR #184 (merge commit `17b18cf`).
- QA report above (209 TUI tests + deterministic `TestBackend` render evidence) is recorded, so the
  card moves to `done` rather than staying in `qa`.
- Remote branch `feat/FEAT-075-tool-call-running-indicator` deleted.
- The local feature branch was left in the shared checkout because another session has unrelated
  uncommitted work in the same checkout (`session.rs` and other files); switching branches would
  overwrite that work. It can be deleted once that checkout is free.

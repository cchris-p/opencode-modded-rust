---
id: "FEAT-072"
title: "Auto-scroll the session transcript while dragging a text selection to a pane edge"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "done"
created: "2026-11-11"
updated: "2026-11-11"
---

# Auto-scroll the session transcript while dragging a text selection to a pane edge

## Summary

When copying text out of the TUI, holding a mouse drag against the top or bottom edge of the tmux
pane should scroll the session transcript in that direction so the selection can extend past the
current viewport. This is the vanilla OpenCode/opentui selection behavior: drag near an edge, the
transcript keeps scrolling, and the selection keeps growing until the mouse is released or the
transcript reaches its limit.

## Why this matters

Today a selection is limited to content already visible on screen. Selecting a long assistant
answer or tool output requires scrolling first, which loses the original starting point. That breaks
the primary copy/paste workflow this TUI exists for. Vanilla handles it by auto-scrolling the
scrollable container while a selection drag is active; the Rust TUI should match that behavior.

## Current behavior and code evidence

- `crates/opencode-tui/src/ui/selection.rs` stores the anchor and cursor as screen-space
  `(row, col)` coordinates and extracts text from a viewport snapshot.
- `crates/opencode-tui/src/app/app.rs:872-876`: `MouseEventKind::Drag` only updates the endpoint;
  it never scrolls. `:852-870` handles wheel scroll independently of an active selection, and
  `:883-885` finalizes on `Up`.
- The event reader drops `MouseEventKind::Moved` (`app.rs:202`), so during a held drag only `Drag`
  events carry pointer motion. Auto-scroll therefore needs to be driven from `Drag` plus the
  16 ms `Event::Tick` (`TICK_RATE_MS`, `app.rs:43`) rather than from `Moved`.
- `copy_selection` (`app.rs:2154`) copies from `self.screen_lines`, refreshed each draw
  (`app.rs:4546`), which contains only the current viewport.
- `SessionView` already exposes what is needed: `last_messages_area` (`components/session.rs:98`,
  set at `:699`), `scroll_offset`, `scroll_up_by`/`scroll_down_by` (`:1042`, `:1047`), and
  `max_scroll_offset()` (`:1102`).
- The terminal enables mouse capture (`crates/opencode-tui/src/app/terminal.rs:11`), so the app
  receives the drag instead of tmux handling it as terminal selection.

## Vanilla parity reference

Vanilla selection/copy entry points live in opencode `packages/tui/src/util/selection.ts`, but the
edge auto-scroll itself is implemented by the opentui renderer:

- `anomalyco/opentui` `packages/core/src/renderables/ScrollBox.ts` (`AutoScroll`):
  - on `drag` with the button held, `updateAutoScroll(x, y)` is called;
  - the edge band is `3` cells from the top/bottom (and left/right);
  - speed scales with the minimum distance to an edge: `6` lines/s at `3`, `36` at `2`, `72` at
    `1`;
  - scrolling is accumulated fractionally per frame and applied as integer lines;
  - direction becomes `0` when the box is already at that limit;
  - after each scroll it calls `ctx.requestSelectionUpdate()`;
  - it stops on mouse up, when the pointer leaves the edge band, at the scroll limit, or when a
    selection event ends the drag.
- `anomalyco/opentui` `packages/core/src/renderer.ts` `requestSelectionUpdate()` re-applies the
  drag selection at the latest pointer after content scrolls under it.
- The Rust opentui port `packages/rust/crates/opentui/src/app/selection.rs` stores the anchor
  relative to its widget (`anchor_offset`, "so it moves with the widget when it scrolls") and
  `refresh_selection()` re-applies the selection at the pointer after scroll. That content
  anchoring is what lets a selection span more than one viewport.

Compare against vanilla `dev` under the pin recorded in `AGENTS.md` ("Reference Repo Convention").
The paths above were inspected on 2026-11-11; the Rust product does not vendor opentui, so this is
behavioral parity, not a code port.

## Scope and implementation notes

- Add auto-scroll state driven by `Event::Tick` while `selection.is_selecting()` and the pointer is
  held within 3 cells of the top or bottom of `session_view.last_messages_area`.
- Use a lines-per-second accumulator with the vanilla speed bands (slow 6 / medium 36 / fast 72)
  so the rate is timing-based rather than one line per event.
- Direction is vertical only; clamp at `scroll_offset == 0` and
  `scroll_offset == max_scroll_offset()`.
- Start on the first `Drag` that enters an edge band; update on every subsequent `Drag`; stop on
  `Up`, when the pointer leaves the band, or when the limit is reached.
- Scroll the session transcript only. Do not auto-scroll while dragging in dialogs, the sidebar,
  the prompt, or the home view, and do not change wheel-scroll behavior.
- `last_messages_area`, `scroll_offset`, and `max_scroll_offset()` are currently private to
  `SessionView`; expose read-only accessors (for example `messages_area()` and
  `scroll_limits()`) instead of widening fields.
- Drive the scroll step from the existing `Event::Tick` handler (`app.rs:946`) and mark the tick as
  changed whenever a line is scrolled so the frame redraws; otherwise the highlight and content
  would not update while the pointer is held still.
- Re-anchor the selection to content after each scroll step (the analogue of
  `requestSelectionUpdate`/`refresh_selection`) so the endpoint follows the pointer while the
  anchor stays on the original line.
- Selected content beyond the current viewport must survive to copy. A screen-row-only `Selection`
  plus the current `screen_lines` snapshot is insufficient: it cannot represent more than one
  viewport, and the snapshot is replaced every draw. Retain the selected rendered lines by absolute
  transcript line index while dragging (merging each viewport draw using `scroll_offset`), or expose
  a content-line buffer from `SessionView`; either is acceptable, but copy must read the
  content-relative range.
- Keep existing column semantics and trailing-whitespace/chrome cleanup.

## Non-goals

- No horizontal auto-scroll (the transcript has no horizontal scroll offset).
- No change to copy-on-release, right-click copy, Ctrl+C handling, or the clipboard contents for
  selections that never scroll.
- No double/triple-click word or line selection parity; this card adds edge auto-scroll to the
  existing cell selection only.
- No terminal or tmux-level scrollback changes.

## Acceptance criteria

- Starting a selection inside the session transcript and dragging within 3 rows of the transcript
  top scrolls up at the slow rate, faster within 2 rows, and fastest within 1 row; the bottom edge
  is symmetric.
- Scrolling continues while the pointer is held still in the edge band at a steady rate and stops
  immediately when the pointer leaves the band, the mouse is released, or the transcript reaches
  its limit.
- The selection highlight grows with the newly scrolled content while the anchor stays on the
  original line; the pointer remains at the edge.
- Copying after scrolling includes every line between the anchor and the pointer across all scrolled
  viewports, with no duplicated, skipped, or stale lines, and matches the highlighted text.
- No auto-scroll occurs outside the transcript, without an active selection, or on wheel scroll;
  existing wheel and sidebar scrolling are unchanged.
- Selections that never scroll copy exactly as before.

## Verification

- `cargo check -p opencode-tui`
- `cargo test -p opencode-tui`
- Unit tests: a selection spanning more than one synthetic viewport (anchor stays fixed as content
  scrolls, copy includes all lines); auto-scroll direction/speed/clamping with stop conditions for
  top limit, bottom limit, pointer leaving the band, and release.
- Manual: `ort-build` then `ort` in a session taller than one screen. Start a drag mid-transcript,
  hold at the top edge, confirm scrolling, growing highlight, and that the copied text includes the
  earlier lines; repeat at the bottom; release at the edge. Verify wheel scroll still works outside
  a drag and copy is unchanged when no scrolling occurs.
- Manual under tmux with mouse mode enabled: confirm the drag reaches the app rather than tmux copy
  mode, and that both edge directions behave as above.

## Related Items

- `BUG-024` Mouse-selection copy captures selectable gutter decoration instead of clean text — same
  copy path; keep the existing chrome cleanup intact.
- `FEAT-030` Make terminal tool output expandable on click in the session transcript — same
  transcript mouse hit-testing and interaction area.
- `BUG-049` Fix long-session lag with a bounded render window that still allows timeline jumps to
  old messages — the windowed renderer and content-line indexing this feature must build on.

## Dev Notes

Implemented on top of PR #176 (`origin/development`).

- Added `AutoScroll` and `ContentSelection` to `crates/opencode-tui/src/ui/selection.rs`:
  - `AutoScroll` uses the vanilla opentui 3-row edge band and 6/36/72 lines-per-second bands with a
    fractional accumulator driven by `Event::Tick`.
  - `next_scroll_offset` clamps at the top/bottom transcript limits.
  - `ContentSelection` stores the anchor/cursor by absolute transcript content line so the anchor
    stays on the original line while the transcript scrolls, and extracts copy text from a
    content-line map.
- `SessionView` now exposes read-only `messages_area()`, `scroll_offset()`, `scroll_limits()`,
  `is_point_in_messages()`, and `set_follow_suspended()`. Follow-tail snapping is suspended while a
  transcript selection drag is active so 1-line auto-scroll is not immediately undone.
- `App` tracks a content-anchored transcript selection, a content-line buffer merged from each
  painted viewport, the held pointer, and the `AutoScroll` state. `Drag` updates the content cursor
  and edge band, `Tick` advances the scroll, and `Up` stops it. `copy_selection` reads the content
  buffer when the selection auto-scrolled and otherwise preserves the existing screen-snapshot path.
- The highlight overlay maps screen rows to content lines while the selection is scrolled, so the
  highlight grows with newly revealed content and matches the copied range.

Verification run in a worktree branched from `origin/development` (PR #176 merged):

- `cargo check -p opencode-tui`
- `cargo test -p opencode-tui` (195 passed)
- New unit tests: multi-viewport content selection, anchor stability while scrolling up, edge
  speed bands and interior/outside stop conditions, fractional accumulation, and top/bottom limit
  clamping.
- Manual `ort` verification under tmux is still pending; the card is in `qa` for that pass.

## QA Verification

- PR #178 merged into `development` (`8a5d05d`).
- `cargo check -p opencode-tui`: clean.
- `cargo test -p opencode-tui`: 195 passed, 0 failed.
- CI: `fmt`, `clippy`, `check`, and `test` pass. The two `native` jobs failed on runner
  infrastructure only (macOS runner label `macos-14-arm64-us-default` unsupported; Ubuntu
  `add-apt-repository` DNS failure to `api.launchpad.net`), not on this change.
- Manual mouse-drag QA (holding the pointer at each transcript edge under tmux) was not run in
  this agent session; the card was completed on explicit user direction.

## Merge Closeout

- Merged PR #178 into `development` (`8a5d05d`); remote and local branch
  `feat/FEAT-072-auto-scroll-selection` deleted.
- Temporary worktree `opencode-modded-rust-feat072` removed and the parked duplicate stash dropped.
---
id: "BUG-049"
title: "Fix long-session lag with a bounded render window that still allows timeline jumps to old messages"
priority: "P1"
type: "bug"
area: "BUG"
spec: "wiki/v1.md"
status: "todo"
created: "2026-09-26"
---

# Fix long-session lag with a bounded render window that still allows timeline jumps to old messages

## Summary

Long sessions (hundreds to thousands of messages) make the TUI slow to open, scroll, and redraw,
because the session view rebuilds and lays out the entire transcript on every frame. The fix is to
render only a bounded window (a message/line cutoff) around the viewport instead of the whole
session, so per-frame cost no longer scales with total session length.

The hard constraint: the cutoff must not cost the user access to old content. `/timeline` must keep
listing every user prompt (including prompts older than the cutoff), and selecting any of them must
navigate to that message and actually display it. In other words, the cutoff bounds *what is laid out
per frame*, not *what the user can reach*.

## Why this exists

The product targets a responsive personal daily-driver TUI. `docs/opencode-tui.md` states that UI
changes should preserve scroll stability and low CPU usage. Today, responsiveness degrades as a
session grows, so the surface is worst on exactly the long, high-value sessions a daily driver
accumulates. This is a UI/runtime responsiveness defect, not a missing feature.

It is also the natural next step after `BUG-027` (which bounded streaming refetch cost) and
`BUG-041` (which guarantees the timeline lists every prompt): those fixed *how updates and entries
are produced*, while this card fixes *how much transcript is laid out per frame*.

## Reported behavior

- Sessions with a long history become laggy: opening, scrolling, and redrawing slow down as the
  message count grows.
- The degradation is proportional to total session size, not to what is on screen, which points at
  full-transcript re-layout per frame rather than network or storage.
- Exact thresholds and whether streaming (assistant deltas) makes it worse still need a measured
  baseline; record the observed sizes and timings when reproducing.

## Code evidence

Render cost is O(total messages) for every frame:

- `SessionView::render_messages` iterates over the entire message vector and builds one flat
  `lines` buffer for the whole session:
  `crates/opencode-tui/src/components/session.rs:591` (fn), loop at `:687`.
- The whole buffer is handed to one `Paragraph` and only then scrolled, so every message is laid
  out every draw regardless of viewport:
  `session.rs:1116-1123` (`Paragraph::new(lines).scroll((self.scroll_offset as u16, 0))`).
- `rendered_line_count` is the full transcript length, not a window:
  `session.rs:1107`; `max_scroll_offset` is derived from it: `session.rs:1273`.
- Layout height also tracks the full rendered line count, not a viewport bound:
  `session.rs:255-265`.

Navigation currently depends on the full transcript being rendered:

- `message_first_lines` maps every message id to an absolute line index and is only populated for
  messages that were laid out: `session.rs:1104-1105`, populated at `session.rs:696-698`.
- `scroll_to_message` looks the id up in that map and sets `scroll_offset` from it:
  `session.rs:1254-1258`. If old messages stop being rendered, this map no longer contains them,
  so a timeline jump to an old prompt would silently fail or land wrong. This is the core tension
  this card must resolve.

History is available in full (the fix must stay a render concern, not a data concern):

- Storage returns all messages with no `LIMIT` (`crates/opencode-storage/src/repository.rs`,
  `list_for_session`) and the server endpoint serialises every message with no pagination
  (`crates/opencode-server/src/routes.rs`, `list_messages`); traced in `BUG-041`.
- The TUI store holds the full message list for the session
  (`crates/opencode-tui/src/components/session.rs:634-638`).
- Timeline entries are built from the full list: `timeline_entries_from_messages`
  (`crates/opencode-tui/src/app/app.rs:5412`), called by `handle_open_timeline`
  (`crates/opencode-tui/src/app/app.rs:2516`), and selecting an entry calls
  `SessionView::scroll_to_message`.

## Required solution direction

- Introduce a bounded render window so per-frame work is proportional to the viewport, not the
  session: only the messages/lines needed for the visible region (plus a small overscan) should be
  built and laid out each frame.
- Keep `/timeline` built from the full message list (unchanged), so every user prompt remains
  listed, preserving `BUG-041`.
- When a timeline jump targets a message that is not in the current render window, re-anchor the
  window so the target is included (for example, window ending at the target), then scroll to it.
  Jumping to an old prompt must display that prompt, not just move an abstract offset.
- Decide and document how the user reaches content just above the window (for example, scrolling to
  the window top pages in older messages) so the cutoff is not a dead end. The timeline jump path is
  required; scroll paging above the window is the expected companion behavior.
- Keep follow/tail behavior intact: when the user is near the bottom, new/streamed content must stay
  pinned and the window must advance; when scrolled up, the view must stay stable.
- Make scrollbar/position semantics coherent under windowing (represent either the whole session
  with a virtual total or the window clearly, but not a mix that jumps).

## Scope

- Bound per-frame transcript layout in `crates/opencode-tui/src/components/session.rs` (the flat
  `lines` build, `rendered_line_count`, `max_scroll_offset`, and the paragraph construction).
- Preserve full-history navigation: keep the timeline entry list complete and make
  `scroll_to_message` re-anchor the window when the target is outside it.
- Preserve follow/tail, scrollbar, and click hit-testing behavior for the new windowed coordinates
  (click hit-testing at `session.rs:1152-1155` also assumes a full-session line index).
- Add regression tests that (a) prove per-frame layout is bounded by the window rather than message
  count and (b) prove a timeline jump to a pre-cutoff message renders and scrolls to it.
- Record measured before/after behavior on a real long session.

## Non-goals

- Adding pagination or limits to the server/storage message API, or changing how sessions are saved.
- Dropping messages from memory or from the loaded session; this is a render/layout bound only.
- Changing timeline entry construction, ordering, or the user-only filter (`BUG-020`); the timeline
  must remain complete (`BUG-041`).
- Re-doing `BUG-027`'s streaming refetch coalescing, or the incremental-part streaming follow-up.
- Scrollback-to-file, transcript export redesign (`FEAT-001`), or cross-session inspection
  (`FEAT-026`).
- Redesigning message rendering itself (wrapping, reasoning/tool collapse); those stay as they are,
  just windowed.

## Done when

- Per-frame transcript layout cost is bounded by the render window, not total message count, and a
  session with a very large history stays responsive to input, scroll, and redraw.
- A message cutoff is in effect: only the window (plus overscan) is laid out per frame, and this is
  covered by a test that grows the history without growing per-frame layout work.
- `/timeline` still lists every user prompt, and selecting a prompt older than the cutoff navigates
  to that message and renders it (test plus live check).
- Follow/tail behavior still pins to the bottom while new content arrives.
- `cargo fmt --all -- --check`, `cargo clippy -p opencode-tui --all-targets`, and
  `cargo test -p opencode-tui` pass.

## Recommended verification

- Reproduce first: `ort-build`, then `ort`; open a session with a large history (record message
  count) and note open/scroll/redraw responsiveness before the change.
- After the change, confirm the same session scrolls smoothly and input stays responsive, including
  during a streaming turn.
- Run `/timeline`: confirm the count matches every user prompt in the session, then use `Home` +
  `Enter` to jump to the oldest prompt and confirm it is rendered (pre-cutoff jump), and `End` +
  `Enter` to jump to the newest.
- Confirm scrolling to the top of the window pages in older content, and that returning to the
  bottom re-pins follow behavior.
- `cargo test -p opencode-tui -- --test-threads=1`.

## Open questions

- Window sizing policy: by message count, by rendered line count, or a multiple of the viewport
  height? Resolve during implementation and record the choice.
- Whether the window is recomputed every frame from `scroll_offset` or cached and only shifted at
  boundaries; pick the one with the lowest per-frame cost and document it.
- Scroll paging above the window: auto-extend silently vs. an explicit "load older" affordance.
- Scrollbar semantics under windowing: virtual whole-session total vs. window-only extent. Settle it
  so the thumb does not jump unexpectedly on re-anchor.
- Interaction with `was_near_bottom`/follow and with queued/pending assistant boundaries computed
  from the full message list (`session.rs:644-659`) when only a window is rendered.

## Related Items

- `BUG-041` Guarantee the session timeline lists every user prompt and can jump to the top (qa) -
  established complete timeline entries and `Home`/`End`/paging; this card must not regress it.
- `BUG-027` Session keeps freezing during thinking mode (done) - bounded streaming refetch; this
  card bounds per-frame layout, the other half of long-session responsiveness.
- `BUG-020` Timeline shows only user-sent prompts (done) - user-only timeline filter preserved here.
- `BUG-022` `/thinking` toggle shows a line count instead of actual reasoning (qa) - reasoning render
  path this windowing must keep working.
- `FEAT-028` Add a hide-tool-calls toggle - reduces rendered content; complementary, not a
  substitute for windowing.
- `FEAT-001` Improve historical chat transcripts workflow (done) - same "reach earlier turns" theme.
- `FEAT-026` Add cross-session transcript inspection (todo) - depends on full-history navigation.

## Notes

- 2026-09-26: Created from a user request to fix long-session lag with a message cutoff that still
  allows timeline navigation to old messages. Chosen as a `BUG` (responsiveness defect) rather than
  a feature. The recurrence of `BUG-027` (updates) and `BUG-041` (timeline completeness) makes the
  render window the remaining long-session bottleneck to address.
- 2026-09-26: Re-IDed from a duplicate `BUG-043` to `BUG-049`. The cluster card `BUG-043`
  (`session-run-can-end-without-terminal-state`, qa) already owned that ID; this render-performance
  card is unrelated to the run terminal-state cluster.
- Current code evidence to revisit at implementation time: `render_messages` builds a flat
  whole-session `lines` buffer (`session.rs:591-1098`), `rendered_line_count`/`max_scroll_offset`
  are full-session (`session.rs:1107`, `:1273`), and `scroll_to_message` depends on
  `message_first_lines` for every rendered message (`session.rs:1254-1258`), which windowing must
  re-anchor rather than break.
- If an implementation adds a window but leaves `message_first_lines` only covering the window, a
  timeline jump to an old prompt will fail by design; that is the specific regression to test for.
- Keep the server/storage unlimited. If a payload limit is ever introduced for other reasons,
  timeline navigation and the window re-anchor must still fetch the older target explicitly.

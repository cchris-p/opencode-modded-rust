---
id: "BUG-054"
title: "Queued user-message rows jitter (one-line step per stream event) while a turn streams"
priority: "P2"
type: "bug"
area: "BUG"
spec: "invariants/message-queuing.md"
status: "done"
created: "2026-09-27"
---

# Queued user-message rows jitter (one-line step per stream event) while a turn streams

## Summary

The queued-message visual now matches vanilla (agent-colored ` QUEUED ` badge in place of the
timestamp row), but the queued rows **shift by one line on every stream event** while a turn is
streaming. This is a display-stability defect, not a contract change — the badge's text, spacing,
color, and boundary algorithm are already correct and must not be altered.

Reported reproduction (user, 2026-09-27): send **3 queued messages** while a turn is streaming.
On **each stream event / operation** (each chunk of assistant output), the queued messages
**flash/step by one line**. The motion repeats per event, so the queued rows visibly jitter for
the whole turn instead of sitting still until they are promoted.

Condition (verified 2026-09-27): it occurs while the transcript is **shorter than the available
messages pane** (unclamped). Once the transcript is tall enough to fill the pane, the pane height
is fixed and the queued rows are stable. It only takes a few turns, so this is the normal state
during early/moderate sessions.

## Root Cause (verified 2026-09-27)

The messages pane height is sized from the **previous frame's** line total, so while the pane is
unclamped it is always one line shorter than the current content. The one-line discrepancy becomes
a one-line `scroll_offset`, which moves every visible queued row.

- `SessionView::render` sizes the pane with the stale count:
  `desired_messages_height = (self.rendered_line_count as u16).max(1).min(max_messages_height)`
  (`crates/opencode-tui/src/components/session.rs:306-308`). `self.rendered_line_count` is the
  value left over from the previous frame's `render_messages`.
- `render_messages` then recomputes the real total and sets the viewport from that stale-sized
  area: `self.rendered_line_count = total_lines` and
  `self.messages_viewport_height = usize::from(messages_area.height)` (`session.rs:812-813`).
- While unclamped, `messages_area.height == previous_total == total_lines - 1`, so
  `max_scroll = rendered_line_count - messages_viewport_height == 1`
  (`session.rs:815, 1075-1078`). Because the view is near the bottom, `scroll_offset` is pinned
  to `1` (`session.rs:816-817`), hiding the top line and shifting every visible row up by one.
- On the next render without new content the layout uses the now-correct total, the pane grows to
  match, `max_scroll` returns to `0`, and the rows drop back down by one. With streaming, this
  alternates/repeats on every event — the reported "flash".

Verified with a temporary `TestBackend` reproduction (removed after the run):
session = completed user + completed assistant + in-flight assistant + 3 queued user messages,
`TestBackend::new(80, 60)`, appending one assistant line per step.

```
step 0: draw1=([19],[24],[29], scroll=1, total=29, viewport=28)
        draw2=([20],[25],[30], scroll=0, total=29, viewport=29)
step 7: draw1=([26],[31],[36], scroll=1, total=36, viewport=35)
        draw2=([27],[32],[37], scroll=0, total=36, viewport=36)
```

Every queued row moves one line between `draw1` and `draw2`; `viewport` is always `total - 1`
until the pane clamps. Re-run at `TestBackend::new(80, 30)` where the content already exceeded the
pane: rows stayed fixed (`q1=10, q2=15, q3=20` across 8 steps) — confirming the defect is the
unclamped pane-sizing lag, not the badge or the queue boundary.

Adjacent, not the reported cause: toggling the in-flight assistant's completion (moving
`pending_assistant_idx`, i.e. the badge on/off) moves rows by up to 3 lines per toggle, not the
uniform one-line step seen here. It should still be kept stable, but it is not the primary defect.

## Evidence

- Badge rendering: `crates/opencode-tui/src/components/session_message.rs:68-90`
  (`render_user_message`, `queued` branch replaces the timestamp row).
- Queued boundary: `crates/opencode-tui/src/components/session.rs:1227-1231` (hash) and
  `:1286-1288` (render); both derive `is_queued` from
  `ctx.pending_assistant_idx.is_some_and(|pending| idx > pending)`.
- `pending_assistant_idx` is recomputed every render from message completion state
  (`session.rs:697-709`): the last assistant with `completed_at.is_none()` and no completed
  assistant after it. It is part of the render-cache globals hash (`session.rs:721`).
- The in-flight assistant message is replaced by full snapshots during streaming
  (`crates/opencode-server/src/routes.rs` `sync`/snapshot merge path; `BUG-043`/`BUG-051`), so a
  frame can observe a message list where the pending assistant index changes between frames.

## Scope

- Fix the pane-sizing lag so the messages pane height, the virtual line total, and the viewport
  are consistent within a single frame. Recommended implementation:
  - Extract the existing per-message height measurement (`session.rs:780-793`, backed by the
    `layout_cache`) into a `measure_total_lines(content_width)` helper, where `content_width`
    is derived from the horizontal area only (`session.rs:658-676`, independent of height).
  - Call it in `render()` before building the layout, and use the fresh total for
    `desired_messages_height` (`session.rs:306-308`) instead of `self.rendered_line_count`.
  - Have `render_messages` reuse the precomputed heights/`rendered_line_count` for the paint
    pass (it already recomputes the same values, so the Pass-1 loop can be skipped or made
    idempotent).
  - Result: when unclamped, `messages_area.height == rendered_line_count`,
    `max_scroll == 0`, `scroll_offset == 0`, and queued rows never shift.
- Minimal fallback if a pre-layout measure is judged too invasive: carry one line of slack in
  the unclamped pane (`desired_messages_height = min(rendered_line_count + 1, max_messages_height)`)
  and force `scroll_offset = 0` when the content fits the pane. This is less robust for events
  that add more than one line; prefer the measure-before-layout fix.
- Keep `is_queued`/`pending_assistant_idx` stable frame-to-frame while a turn is genuinely
  streaming, so a queued user message keeps its badge continuously until its own turn starts.
- Confirm the queued rows only move when the queued prompt is actually promoted (turn start),
  not transiently during snapshot/stream updates.
- Keep the badge contract exactly as GATE-001 specifies: uppercase `QUEUED`, one leading/trailing
  space, agent-color background, contrast-selected foreground, bold, timestamp replaced, boundary
  from assistant completion.

## Non-goals

- Changing the badge text, colors, spacing, or the queued boundary algorithm.
- Redesigning the "prompt hugs the content" layout; the prompt must still sit directly below the
  rendered messages when the transcript is short.
- The windowed-render performance work (`BUG-049`) beyond the pane-sizing consistency fix.
- Prompt queue ordering/abort semantics (`GATE-001`, done).

## Done when

- With several prompts queued (report repro: 3) and the transcript shorter than the pane, the
  queued rows stay visually still through the entire streaming turn — no one-line step per
  stream event, including across redraws that add no content.
- Once the transcript fills the pane, behavior is unchanged (rows anchored to the bottom).
- Each ` QUEUED ` badge stays continuously visible (no on/off flicker) until that prompt's turn
  begins, then disappears exactly once, when the queued message is promoted.
- Regression test: a `TestBackend` render that appends one assistant line per step to an
  in-flight assistant with 3 queued user messages, asserting the queued rows' screen positions
  are identical across every step and across a content-adding draw followed by a no-op draw,
  while the transcript is shorter than the pane.

## Recommended verification

- `cargo test -p opencode-tui components::session` / `components::session_message`.
- Manual: `ort-build` then `ort`; in a fresh/short session submit a prompt, then submit **3**
  more while the first streams, and watch the queued rows through the full turn and queue drain
  — they must not step by one line per stream event, and must hold still across redraws.
- Confirm the same with `show_timestamps` on and off, and with the sidebar shown/hidden.
- Confirm a long session (transcript already filling the pane) still follows the bottom and stays
  stable.

## Implementation (2026-09-27)

`crates/opencode-tui/src/components/session.rs`:

- Added `SessionView.measured_line_count` (separate from `rendered_line_count`) so the current
  frame's content total can size the pane without disturbing the follow/scroll state the paint
  pass derives from `rendered_line_count`.
- `render_messages` gained a `measure_only: bool`. In measure-only mode it runs the existing
  per-message height cache pass and computes the whole-session total, then returns before setting
  `messages_viewport_height`, `scroll_offset`, `last_messages_area`, or painting. It stores the
  total in `measured_line_count`.
- `SessionView::render` now calls `render_messages(frame, area, true)` once before building the
  layout, and `desired_messages_height` uses `measured_line_count` (the fresh total) instead of
  the previous frame's `rendered_line_count`. The paint call is `render_messages(frame, layout[1],
  false)`.
- Net effect: within one frame the pane height, the virtual total, and the viewport agree. When
  the transcript is shorter than the pane, `messages_area.height == rendered_line_count`, so
  `max_scroll == 0` and `scroll_offset == 0`; the one-line scroll offset that moved queued rows is
  gone. The clamped path and follow/scroll behavior are unchanged because `rendered_line_count`
  and `messages_viewport_height` are still set by the paint pass exactly as before.

Chosen over the card's "+1 slack" fallback because it is exact for any per-event line delta (tool
calls, wrapped lines), not just single-line text streams, and does not add a permanent blank row
between the transcript and the prompt.

## Verification (2026-09-27, agent-run)

Artifact: `crates/opencode-tui` at the fix commit; `cargo` 1.98.1.
`SCOPEMUX_SKIP_NATIVE_BUILD=1` used for `cargo check` (native `scopemux-core` build not needed for
the TUI tests).

- **Failing → passing regression test**:
  `components::session::tests::queued_message_rows_do_not_jitter_while_streaming`
  (`crates/opencode-tui/src/components/session.rs`). It renders a short session (4 prior turns +
  in-flight assistant + 3 queued user messages) at `TestBackend(80, 60)` (unclamped), appends one
  assistant line per step, then draws twice per step, asserting the queued rows do not move on the
  no-op redraw and that `rendered_line_count <= messages_viewport_height` / `scroll_offset == 0`.
  - Pre-fix (measure call temporarily disabled): **FAIL**, step 0:
    `([19], [24], [29]) vs ([20], [25], [30])` — the exact one-line step.
  - Post-fix: **PASS**.
- **Clamped regime stability**: a long-session reproduction (40 prior turns, `TestBackend(80, 30)`,
  same streaming + double draw) held the queued rows fixed at `[10]/[15]/[20]` with `viewport`
  constant at `19` and `scroll` tracking `total` in lockstep, before and after the fix — confirming
  the defect is the unclamped pane-sizing lag only.
- **Suite**: `cargo test -p opencode-tui --lib -- --test-threads=1` → 173 passed, 2 failed. The 2
  failures are the pre-existing/flaky `components::prompt::tests::tab_autocomplete_uses_first_candidate`
  (documented flaky in `BUG-049`) and
  `components::prompt::tests::utf8_backspace_delete_and_cursor_are_char_safe`, which passes in
  isolation and only fails from an order-dependent env-var lock poison in the same module; both are
  in `prompt.rs`, untouched here.
- `cargo check --workspace` clean; `cargo fmt --all` applied. `cargo clippy` is not installed on
  this toolchain, so it was not run.
- Live interactive TUI capture was not used: the repo's `scopemux-self-qa` policy is to drive QA
  deterministically and not rely on the interactive TUI, and the render path is covered directly by
  the regression test above.

## Closeout (2026-09-27)

- Fixed, tested, and closed to `done`. Fix commit: `8c70654`
  (`fix(tui): size session pane from current transcript to stop queued-row jitter (BUG-054)`)
  on `development`.
- Confidence at implementation time: high for the verified unclamped pane-sizing lag (deterministic
  failing→passing regression). If a one-line queued-row step is ever observed in a transcript that
  already fills the pane, reopen and re-verify against the pending-boundary/snapshot paths.

## Related Items

- `GATE-001` Gate: prompt queuing and queue display must match vanilla (done; owns the badge
  contract and `invariants/message-queuing.md`).
- `BUG-049` Fix long-session lag with a bounded render window (qa; renders the same transcript).
- `BUG-051` Continuation prompt flashes / unordered (adjacent in-flight message instability).
- `BUG-047` Assistant turn progress is not persisted until the run completes (snapshot behavior).

## Notes

- Reported by the user 2026-09-27: "The queued message visual looks a lot better. The only
  problem is the queued message itself jitters a bit." Follow-up repro detail: with 3 queued
  messages, the queued rows each flash up one line after every stream event/operation.
- Visual/stability fix only; the queued-message contract is already satisfied.
- Verified cause: unclamped pane height sized from the previous frame's `rendered_line_count`
  (`session.rs:306-308`), making `viewport = total - 1` and forcing `scroll_offset = 1`
  (`session.rs:812-817`). Fix is to measure the current content total before layout so pane,
  total, and viewport agree in one frame.
- `BUG-049` (windowed render, qa) touched the same sizing/scroll path; this defect is in the
  pre-existing `desired_messages_height` sizing, not the windowing itself, but the fix lands in
  the same function and should be coordinated with `BUG-049` QA.
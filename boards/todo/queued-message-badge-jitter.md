---
id: "BUG-054"
title: "Queued user-message rows jitter (one-line step per stream event) while a turn streams"
priority: "P2"
type: "bug"
area: "BUG"
spec: "invariants/message-queuing.md"
status: "todo"
created: "2026-09-27"
---

# Queued user-message badge jitters/flickers while a turn is streaming

## Summary

The queued-message visual now matches vanilla (agent-colored ` QUEUED ` badge in place of the
timestamp row), but the queued rows **jitter** during use. This is a display-stability defect,
not a contract change — the badge's text, spacing, color, and boundary algorithm are already
correct and must not be altered.

Reported reproduction (user, 2026-09-27): send **3 queued messages** while a turn is streaming.
On **each stream event / operation** (each chunk of assistant output), the queued messages
**flash up by one line**. The motion repeats per event, so the queued rows visibly step/flicker
for the whole turn instead of sitting still until they are promoted.

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

## Hypothesis (to confirm during fix)

The one-line-per-event flash points at the **windowed transcript layout/scroll being recomputed
and re-pinned on every stream event**, not at the badge's own drawing. The bottom-anchored
queued rows are downstream of the in-flight assistant, so any transient off-by-one in the
whole-session line total or the scroll offset moves them:

- `scroll_offset` is re-pinned to `max_scroll` every render when near bottom
  (`session.rs:815-818`), and `max_scroll = rendered_line_count - viewport_height`
  (`session.rs:1075-1078`). `rendered_line_count` is recomputed from per-message heights each
  frame (`session.rs:802-812`), so it tracks the growing in-flight assistant.
- Per-message heights come from a cache keyed by a per-message signature
  (`session.rs:780-793`). If the in-flight assistant's height and the computed
  `rendered_line_count` disagree by one line for a frame (stale/partial cache entry, or an
  extra transient row such as a spacing/footer/streaming line that appears then disappears),
  the re-pin scrolls the viewport one line and the queued rows jump.
- The window is laid out from whole-session line offsets (`session.rs:820-897`), so a one-line
  swing in `scroll_offset` shifts every visible queued row by exactly one line — matching the
  report.

Confirm which of these is the transient: an unstable per-frame height for the in-flight
assistant, a spacing line, or the follow/scroll re-pin racing the layout update.

## Scope

- Make the transcript layout/scroll stable across stream events while a turn is streaming, so
  the queued rows do not step by one line on each event.
- Make `is_queued`/`pending_assistant_idx` stable frame-to-frame while a turn is genuinely
  streaming, so a queued user message keeps its badge continuously until its own turn starts.
- Confirm the queued rows only move when the queued prompt is actually promoted (turn start),
  not transiently during snapshot/stream updates.
- Keep the badge contract exactly as GATE-001 specifies: uppercase `QUEUED`, one leading/trailing
  space, agent-color background, contrast-selected foreground, bold, timestamp replaced, boundary
  from assistant completion.

## Non-goals

- Changing the badge text, colors, spacing, or the queued boundary algorithm.
- The windowed render work (`BUG-049`) beyond any interaction with badge stability.
- Prompt queue ordering/abort semantics (`GATE-001`, done).

## Done when

- With several prompts queued (report repro: 3), the queued rows stay visually still through the
  entire streaming turn — no one-line step/flash per stream event.
- Each ` QUEUED ` badge stays continuously visible (no on/off flicker) until that prompt's turn
  begins, then disappears exactly once, when the queued message is promoted.
- A regression test asserts the whole-session line total / scroll offset and
  `is_queued`/`pending_assistant_idx` are stable across successive renders of the same logical
  state.

## Recommended verification

- `cargo test -p opencode-tui components::session` / `components::session_message`.
- Manual: `ort-build` then `ort`; submit a prompt, then submit **3** more while the first streams,
  and watch the queued rows through the full turn and queue drain — they must not step up one
  line per stream event.
- Confirm the same with `show_timestamps` on and off, and with the sidebar shown/hidden.

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
- The motion is a layout/scroll artifact (per-event re-pin of the bottom-follow window), not a
  change in the badge text or boundary.

---
id: "BUG-041"
title: "Guarantee the session timeline lists every user prompt and can jump to the top"
priority: "P2"
type: "bug"
area: "BUG"
spec: ""
status: "qa"
created: "2026-09-23"
---

# Guarantee the session timeline lists every user prompt and can jump to the top

## Summary

The windowed session timeline should always list every prompt the user actually sent, so the
user can jump straight to the top of a long session. The reported symptom ("only the latest
prompts appear") is real but originates in **vanilla opencode**, which windows each session to
the last 100 messages before the timeline reads them. The Rust fork does **not** have that
window and already lists every user prompt, verified live. This card turns that finding into a
hard guarantee plus the navigation needed to actually use it: regression tests that lock out the
vanilla window, and first-class "jump to top/bottom" controls for long timelines.

## Root cause (confirmed)

Vanilla opencode's TUI sync store loads and keeps only the last 100 messages per session, so any
older user prompt falls out of the timeline.

- `sync()` fetches with a hard cap: `sdk.client.session.messages({ sessionID, limit: 100 })`
  (`packages/tui/src/context/sync.tsx:603`).
- It then keeps only the last 100 regardless of the fetch result:
  `const removed = infos.slice(0, -100)` and `const visible = infos.slice(-100)`; the removed
  messages' parts are deleted (`sync.tsx:624-656`).
- The timeline is built from that windowed store:
  `sync.data.message[props.sessionID] ?? []` (`packages/tui/src/routes/session/dialog-timeline.tsx:23`).
- Amplifier: the timeline skips any user message without a loaded, non-synthetic/non-ignored text
  part (`dialog-timeline.tsx:27-30`), and removed messages have their parts deleted, so some
  surviving old prompts are still dropped.

The cap is message-count based, not turn-based. Each user turn produces many
assistant/reasoning/tool messages, so a session crosses 100 messages after only a handful of
turns and starts losing the earliest prompts.

### Real-data confirmation

Against the Rust product database, vanilla's last-100 window drops real user prompts:

- Session `ses_84f2e28ef2574fb8b21fbae447d3df5b`: 260 messages, 13 user prompts -> last 100
  keeps 11 -> **2 dropped**.
- Session `ses_9b58e7d4a1e146dfb040b6eb733ea94f`: 320 messages, 11 user prompts -> last 100
  keeps 5 -> **6 dropped**.

## Rust current state (live-verified — already complete)

Every layer between storage and the dialog preserves the full history, and the live TUI timeline
on both sessions above shows **all** user prompts (11/11 and 13/13), not just the latest:

- Storage returns all messages for a session with no `LIMIT`:
  `MessageRepository::list_for_session` (`crates/opencode-storage/src/repository.rs:654-707`,
  query at `:667-670`).
- Server load assigns the entire message list: `load_sessions_from_storage`
  (`crates/opencode-server/src/server.rs:170-188`, assign at `:180-181`).
- The message endpoint serialises every message with no pagination:
  `list_messages` (`crates/opencode-server/src/routes.rs:1823-1838`).
- The TUI client fetches the full body and the store replaces it wholesale:
  `ApiClient::get_messages` (`crates/opencode-tui/src/api.rs:1026-1039`) and
  `sync_session_from_server` / `SessionContext::set_messages`
  (`crates/opencode-tui/src/app/app.rs:3952-3974`,
  `crates/opencode-tui/src/context/session_context.rs:184-186`).
- The timeline builder emits one entry per user message, in order, with no cap:
  `timeline_entries_from_messages` (`crates/opencode-tui/src/app/app.rs:5085-5103`), called by
  `handle_open_timeline` over the whole stored vector (`app.rs:2329-2344`).

Residual Rust gaps (the actual work on this card):

- No regression test protects completeness; a future "parity" change could add a
  vanilla-style window without anyone noticing.
- The dialog is a fixed 20-row list (`crates/opencode-tui/src/components/dialogs/timeline.rs:81`)
  with only `Up`/`Down`/`Enter`/`Esc` handled (`crates/opencode-tui/src/app/app.rs:1753-1770`).
  There is no `Home`/`End`/`PageUp`/`PageDown` and no position indicator, so a long list is
  navigable only by holding an arrow key.
- The render passes a cloned `ListState` (`timeline.rs:129`), so the scroll offset is never
  written back to the live state. It happens to still keep the selection visible because ratatui
  recomputes bounds from `selected`, but the offset is not a durable source of truth.

## Implementation plan

### 1. Lock in completeness with tests (no behavior change expected)

- Add a unit test next to the existing timeline tests
  (`crates/opencode-tui/src/app/app.rs:5202-5222`) that builds a synthetic history with >100
  mixed messages (for example 150 assistant/system plus ~30 user) and asserts
  `timeline_entries_from_messages` returns exactly one entry per `MessageRole::User`, in
  oldest-to-newest order, and none for assistant/system.
- Add a test asserting entry construction does not depend on a text part being present (a user
  turn with empty content still produces an entry), so the vanilla `!part` skip is not
  inherited.
- Optional but preferred: a server-level test that `GET /session/{id}/message` returns every
  stored message for a session with more than 100 messages (guards the API against a future
  default limit).

### 2. Make long timelines navigable to the top/bottom

- In `crates/opencode-tui/src/components/dialogs/timeline.rs`:
  - Add `select_first`, `select_last`, and `move_by(delta)` (for paging) that update
    `state.select(...)` across the whole entry vector.
  - Derive the page step from the rendered viewport height so paging matches what is visible.
  - Show a position indicator in the title (for example `Timeline  3/57`) so the user can see
    the list is complete; a scrollbar is optional.
  - Keep the repo-wide `&mut self.state.clone()` render call
    (`crates/opencode-tui/src/components/dialogs/timeline.rs`, formerly line 129): ratatui
    recomputes the visible window from the selected index each frame, so the selected entry stays
    visible. Changing only this dialog to take `&mut self` would diverge from every other dialog
    and fight the existing render borrow structure, so it was intentionally left as-is.
- In `crates/opencode-tui/src/app/app.rs` timeline key handler (`app.rs:1753-1770`), wire:
  - `Home` (and optionally `g`) -> `select_first`
  - `End` (and optionally `G`) -> `select_last`
  - `PageUp` -> `move_by(-page)`, `PageDown` -> `move_by(+page)`
  - Keep `Up`/`Down`/`Enter`/`Esc` behavior unchanged.
- Decide and document the initial selection in `TimelineDialog::open`
  (`timeline.rs:34-42`): keep first entry selected (oldest, so `Enter` immediately jumps to the
  top), or select the entry nearest the current session scroll position. Record the choice in
  the card.

### 3. Verify jump-to-top end to end

- Confirm `Enter` on the first entry scrolls the session view to the absolute top
  (`SessionView::scroll_to_message`, `crates/opencode-tui/src/components/session.rs:1125-1142`;
  the first message's `message_first_lines` entry is 0).
- Confirm `Enter` on the last entry scrolls to the newest prompt.

## Files to touch

- `crates/opencode-tui/src/components/dialogs/timeline.rs` — navigation methods, real state,
  title indicator.
- `crates/opencode-tui/src/app/app.rs` — key wiring, completeness/regression tests.
- `crates/opencode-server/src/routes.rs` (test only) — optional no-default-limit guard.

## Non-goals

- Adding assistant/system entries or role filters (`BUG-020` settled user-only).
- Matching vanilla's newest-first ordering or its `!part` skip; Rust intentionally lists every
  user prompt.
- Adding pagination/limits to the server message API, or a vanilla-style 100-message window.
- Redesigning session storage or the session view.
- Changing the fork-from-timeline path.

## Done when

- Tests prove `timeline_entries_from_messages` yields one entry per user message for a >100
  message history, oldest-to-newest, with no assistant/system and no text-part dependency.
- The timeline dialog supports first/last/paging navigation; the selected entry is always
  visible; a position indicator shows the list is complete.
- `Enter` on the first entry scrolls the session view to the very top, and on the last entry to
  the newest prompt, verified in a live TUI run.
- The Rust path is documented as intentionally free of vanilla's 100-message window.

## Recommended verification

- `cargo test -p opencode-tui --lib timeline` (or the specific new test names).
- `cargo fmt --all -- --check` and `cargo clippy -p opencode-tui --all-targets`.
- `ort-build`, then `ort`; open a session with >100 messages and multiple user prompts; run
  `/timeline`:
  - confirm every user prompt appears (count matches the session),
  - confirm `End`/`PageDown` reach the newest and `Home`/`PageUp` reach the oldest,
  - confirm `Enter` on the oldest entry lands at the top of the session view,
  - confirm the position indicator reads `<current>/<total>`.
- Compare with vanilla: vanilla's `/timeline` on the same session shows only the last 100
  messages' prompts.

## Related Items

- `BUG-020` Timeline shows only user-sent prompts (done) - established the user-only filter this
  card preserves.
- `BUG-030` Down at the newest prompt-history entry clears the input box (qa) - adjacent
  navigation behavior.
- `FEAT-037` Gate prompt history navigation to cursor boundaries (done) - prompt-history recall
  UX.
- `FEAT-001` Improve historical chat transcripts workflow - same "return to earlier turns" theme.
- `FEAT-026` Add cross-session transcript inspection (todo) - also depends on complete message
  history.

## Notes

- 2026-09-23: Root cause confirmed in vanilla: `sync.tsx:603` (`limit: 100`) plus
  `sync.tsx:624-656` (`slice(-100)` and part deletion) feed a windowed store into
  `dialog-timeline.tsx:23-30`. Rust was traced end to end (storage -> server load -> API -> TUI
  store -> entry builder) and has no message cap; the live TUI timeline was driven over a PTY on
  two real sessions and showed all 11/13 user prompts. The card is therefore scoped to
  (a) regression tests that forbid the vanilla window and (b) making long timelines easy to
  navigate to the top/bottom.
- Decision to confirm during implementation: initial timeline selection (oldest-first, so
  `Enter` jumps to top) vs nearest-to-current-scroll.
- Keep the server API unlimited; if a limit is ever added for payload reasons it must be
  turn-aware or the timeline must fetch beyond it.

## Dev Notes

- Branch: `bug/BUG-041-timeline-lists-every-user-prompt` (from `origin/development`).
- Renumbered from the originally created `BUG-040` to `BUG-041`: another agent's card
  `boards/qa/grep-tool-blocks-async-runtime.md` already owns `BUG-040` on the local
  `development` branch. The file was renamed to `ensure-timeline-lists-every-user-prompt.md`.
- Change (`crates/opencode-tui/src/components/dialogs/timeline.rs`): added `select_first`,
  `select_last`, `move_by(delta)`, `page_up`, `page_down`, `selected_index`, `len`, and `is_empty`;
  added a `page_step` cell derived from the rendered inner height (default 10 before first render);
  the title now renders `Timeline  <current>/<total>` and omits the counter when empty.
- Change (`crates/opencode-tui/src/app/app.rs`): the timeline key handler now maps `Home` ->
  `select_first`, `End` -> `select_last`, `PageUp` -> `page_up`, `PageDown` -> `page_down`,
  keeping `Up`/`Down`/`Enter`/`Esc` unchanged.
- Decision: `open()` keeps the oldest prompt selected (index 0) so `Enter` immediately jumps to
  the top of the session; `End`/`PageDown` reach the newest. This was the card's open question and
  is now settled oldest-first.
- Decision: did not change the repo-wide `&mut self.state.clone()` render pattern; ratatui
  recomputes the visible window from the selected index each frame, so the selection stays
  visible without a persisted offset, and diverging from the other dialogs was not worth it.
- Tests added: `timeline_entries_include_every_user_prompt_in_large_history` (40 user prompts
  interleaved with 120 assistant messages, asserts one entry per user prompt in order),
  `timeline_entries_do_not_require_text_content` (empty-content user turn still yields an entry),
  and dialog tests `open_selects_oldest_prompt`, `select_first_and_last_clamp_to_bounds`,
  `move_by_clamps_within_the_list`, `page_moves_use_measured_step`,
  `navigation_on_empty_timeline_is_safe`, `title_shows_position_out_of_total`,
  `empty_timeline_title_has_no_position`.
- Deferred (optional in the plan): a server-level test that `GET /session/{id}/message` has no
  default limit; the client/server path was already verified empirically against two real
  sessions (260 and 320 messages).
- Verification: `cargo test -p opencode-tui --lib -- --test-threads=1` -> 111 passed, 0 failed;
  `cargo fmt --all -- --check` clean; `cargo clippy -p opencode-tui --all-targets` produced no
  warnings referencing the changed files. Live TUI check (rebuilt `target/debug/opencode`, driven
  over a PTY attached to a local server): `/timeline` opened at `Timeline  1/11` on the
  320-message session and `Enter` on the first entry scrolled the session view to the first
  prompt.
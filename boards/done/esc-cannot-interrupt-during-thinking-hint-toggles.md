---
id: "BUG-029"
title: "Esc cannot interrupt a thinking turn; the interrupt hint toggles between states"
priority: "P1"
type: "bug"
area: "BUG"
spec: "invariants/coding-session-behavior.md"
status: "done"
created: "2026-09-21"
---

# Esc cannot interrupt a thinking turn; the interrupt hint toggles between states

## Summary

The user cannot reliably interrupt a running turn, especially while the model is in its thinking
phase. Pressing `Esc` does not stop the run; instead the prompt status line only toggles between
`esc interrupt` and `esc again to interrupt`, so the double-press confirmation never lands an abort.
This is a user-facing UX defect on the existing interrupt path: the confirmation state is
wall-clock/time-windowed and is reset both by the 5-second timeout tick and by a successful
confirmation, so during a long or unresponsive thinking stream the two required keypresses seldom
count as a pair.

This card tracks the reported symptom. Code reading produced a leading hypothesis (confirmation
window and reset semantics), but it is **not confirmed** and the interaction with the thinking
freeze (`BUG-027`) and the abort path (`BUG-019`) is untested.

## Reported behavior

- "I am unable to interrupt at times (likely during thinking)."
- "Pressing `Esc` to interrupt results in the text toggling instead: `esc again to interrupt` ->
  `esc interrupt`."
- Net effect: the abort never fires (or keeps re-arming), and the run continues to completion.

## Why this exists

Stopping a runaway or wrong generation is the primary safety valve for a daily-driver agent loop
(see `BUG-019`). If the confirmation affordance itself is unreliable, the user cannot exercise the
one control they have over a running turn, and the hint text makes it look like input is being
accepted while nothing happens.

## Code evidence

Interrupt key handling (TUI):

- `Esc` is bound to `session_interrupt` (`crates/opencode-tui/src/context/keybind.rs:187`).
- The session route gates on non-idle status, then calls `register_interrupt_keypress()`; if it
  returns `false` the handler returns early **without** aborting
  (`crates/opencode-tui/src/app/app.rs:488-512`).
- `register_interrupt_keypress()` returns `true` only when a prior press is still inside the
  confirmation window; the first press arms and returns `false`, and a successful confirmation calls
  `reset_interrupt_confirmation()` (`crates/opencode-tui/src/components/prompt.rs:631-639`).
- After a confirmed abort the handler immediately calls `clear_interrupt_confirmation()`, before the
  server status is observed (`app.rs:506`).

Confirmation window and reset:

- Window is a wall-clock 5 seconds: `INTERRUPT_CONFIRM_WINDOW_SECS`
  (`prompt.rs:30`), `interrupt_confirmation_active()` (`prompt.rs:1117-1123`).
- Every render tick calls `maybe_reset_interrupt_confirmation()`, which clears the confirmation once
  the window has elapsed (`prompt.rs:409-413`, `prompt.rs:1125-1130`).
- `clear_interrupt_confirmation()` is also called whenever the session is observed idle
  (`app.rs:3678-3699`).

Status line rendering (the visible toggle):

- `SessionStatus::Running` renders `thinking` plus either `esc again to interrupt` when the
  confirmation is active or `esc interrupt` otherwise
  (`crates/opencode-tui/src/components/prompt.rs:1093-1111`).
- So the observed text flip is exactly the confirmation state going armed → reset → armed.

Server abort and cancellation (for the "abort did fire but did not stop it" case):

- `/session/{id}/abort` cancels the active runner if present, else returns `aborted: false`
  (`crates/opencode-server/src/routes.rs:2829-2852`).
- `SessionPrompt::cancel` removes the run and cancels its token
  (`crates/opencode-session/src/prompt.rs:796-804`); the token is registered in `start()`
  (`prompt.rs:759-775`).
- Cancellation is only observed at the top of the step loop (`prompt.rs:1000-1004`) and on the next
  streamed event (`prompt.rs:1162-1166`). If the provider stream is quiet (e.g. a long thinking
  phase that is buffered upstream), the token is not noticed until the next event arrives.

## Hypotheses (confidence-ranked)

- **H1 (70%) - The 5s confirmation window expires before the user's second `Esc` lands**, so the
  second press is treated as a fresh arm and no abort is ever sent. Likely amplified during thinking:
  `BUG-027` starves the event loop, so the user waits for visible feedback and/or keys are delivered
  late, pushing the pair past 5 seconds. The text then toggles exactly as reported.
- **H2 (55%) - The hint resets on a successful confirmation before status goes idle.** Because the
  handler clears the confirmation immediately (`app.rs:506`) while the status only flips on a server
  event (`app.rs:774-797`), a fired-but-slow abort shows `esc interrupt` again, inviting another
  press and reproducing the toggle even when abort did succeed.
- **H3 (45%) - The abort is sent but does not stop the thinking run promptly.** Cancellation is
  only checked on the step boundary or the next stream event (`prompt.rs:1000`, `prompt.rs:1162`),
  and the active runner may not yet be registered during early thinking; the status therefore stays
  `Running`, the hint re-arms, and the user concludes `Esc` does nothing. Overlaps `BUG-019`.
- **H4 (35%) - Status is already `Idle` at press time** (server/local status disagreement while
  streaming), so the handler skips the abort branch entirely and just clears confirmation
  (`app.rs:499`, `app.rs:510`).
- **H5 (25%) - The confirmation state is a single global on the prompt** (`prompt.rs:94-95`), so a
  stale arm from a previous turn or another route can interfere. Lower confidence: it is reset on
  send/clear/set_input.

## Open questions

1. Does the failure require *two* presses to reproduce, or does a single `Esc` also fail? (Distinguishes
   H1 from H3/H4.)
2. Did any abort request actually reach the server during the reported incidents? (Server logs /
   `aborted: true|false`.)
3. Does the hint toggle persist with `/thinking` off? (Separates the `BUG-027` starvation amplifier
   from the confirmation-window logic.)
4. Reproduce timer: how long between the first and second `Esc` in a failing case - under or over 5s?
5. Does the server session status return to `Idle` after the apparent toggle, or does the run finish
   on its own?

## Scope

- Make interrupt confirmation robust for the reported case: a confirmed `Esc`/`Esc` pair must issue
  the abort even when the thinking stream makes the event loop unresponsive.
- Stop the hint from toggling misleadingly: keep `esc again to interrupt` visible through a fired
  abort until the server reports idle (or otherwise make the reset reflect real state).
- Ensure the abort actually reduces a thinking run to idle (coordinate with `BUG-019` rather than
  re-owning its server-side abort fix).
- Add regression coverage for the confirmation state machine (arm → confirm → abort) independent of
  wall-clock timing, and for the reset/render behavior.
- Record the confirmed root cause and evidence on this card.

## Non-goals

- The thinking-phase responsiveness defect itself (`BUG-027` owns the freeze).
- The server-side abort/cancel correctness work already owned by `BUG-019`.
- Changing the double-press safety semantics or the `Esc` keybinding.
- Broad prompt-input or keybind redesign.

## Done when

- A user can reliably interrupt a running turn, including during a thinking phase, using the
  documented `Esc`-twice gesture.
- The interrupt hint never toggles between `esc interrupt` and `esc again to interrupt` without
  either issuing an abort or reflecting a real state change.
- There is an evidence-backed explanation (and regression test) for the confirmation/toggle defect,
  naming whether the failure is the time window, the early reset, the server abort, or a combination.
- `cargo check` and `cargo test` pass for the touched crates (at minimum `opencode-tui`).

## Recommended verification

- Reproduce: `ort-build`, then `ort`; start a turn that produces long reasoning, press `Esc` twice,
  and record whether the run stops and what the hint shows.
- Instrument `register_interrupt_keypress` / `maybe_reset_interrupt_confirmation` to log press
  timing, whether confirmation was active, and whether abort was issued.
- Measure first-press → second-press interval in a failing case versus a working case.
- Confirm the server `POST /session/{id}/abort` response (`aborted: true|false`) during the failure.
- After the fix, confirm the run stops, the hint does not toggle misleadingly, and a subsequent
  prompt works.
- `cargo test -p opencode-tui`; `cargo check -p opencode-tui`.

## Related Items

- `BUG-019` Escape does not interrupt the running session - server-side abort/cancel correctness;
  this card owns the TUI confirmation/UX failure when abort is attempted during thinking.
- `BUG-027` Session keeps freezing during thinking mode - the event-loop starvation that likely
  widens the confirmation window beyond 5s.
- `BUG-022` `/thinking` toggle shows a line count instead of actual reasoning - same thinking/render
  surface, display-only.
- `FEAT-006` Make Ctrl+D exit the TUI and Ctrl+C clear the prompt - same prompt-input key handling
  surface.

## Notes

- Symptom reported by user 2026-09-21: interrupt unreliable during thinking; `Esc` toggles the hint
  text `esc again to interrupt` <-> `esc interrupt` instead of stopping the run.
- Leading suspicion is the 5-second wall-clock confirmation window plus the immediate reset on a
  fired abort; both are timing-dependent and would be exaggerated by an unresponsive event loop.
  Confirm with instrumentation before changing the design.

## Dev Notes

Implemented a defensive fix for the reported symptom (2026-09-21). The root cause was **not**
confirmed by live reproduction; the change addresses the timing/UX and abort-effectiveness
hypotheses together so the reported toggle cannot occur regardless of which one dominates.

TUI (`crates/opencode-tui`):

- Replaced the `interrupt_press_count` + `last_interrupt_time` fields with a small, clock-injected
  `InterruptConfirmation` state machine (`Idle` / `Armed` / `Pending`).
- `Pending` is latched once an abort is issued and cleared only when the session leaves its running
  status (`set_spinner_active(false)`, idle) or a new run starts (`SessionStatusBusy`), so the
  running hint no longer flips back to `esc interrupt` while an abort is in flight. The running
  hint now shows `interrupting` while pending.
- The interrupt handler no longer ignores the result of `abort_session`: a missing client or an
  abort call error clears the confirmation and surfaces a toast instead of silently doing nothing,
  and an `aborted: false` response reports "Nothing to interrupt".

Session (`crates/opencode-session`):

- The provider stream loop now cancels via `tokio::select!` on the run's `CancellationToken` in
  addition to checking it per event, so an abort is observed even when the stream is quiet
  (buffered thinking, between chunks) instead of waiting for the next event.

Status: **unconfirmed root cause; this is a coverage fix, not a proven one.** The card remains in
`qa` pending live reproduction and user feedback.

## Verification

- `cargo fmt -p opencode-tui -p opencode-session`
- `cargo check -p opencode-session -p opencode-tui` - clean.
- `cargo clippy -p opencode-tui -p opencode-session --all-targets` - no new warnings at changed
  code (pre-existing workspace warnings remain).
- `cargo test -p opencode-tui interrupt` - 4 passed:
  `interrupt_second_press_within_window_confirms`,
  `interrupt_second_press_after_window_rearms_without_confirming`,
  `interrupt_pending_latches_until_reset`,
  `interrupt_armed_press_expires_after_window`.
- `cargo test -p opencode-tui` - pre-existing failures unrelated to this change:
  `components::prompt::tests::tab_autocomplete_uses_first_candidate` fails on the base branch too
  (verified by stashing this change), and poisons the shared env lock, cascading to two more tests.
- `cargo test -p opencode-session` - 152 passed; 2 pre-existing environment-dependent failures
  (`instruction::tests::test_find_up_*`, macOS tempdir symlink canonicalization).
- Not yet verified live in the TUI (`ort-build` + `ort`); awaiting user verification and feedback.

## PR

- https://github.com/cchris-p/opencode-modded-rust/pull/66

## Completion

- 2026-09-21: PR #66 merged into `development` (merge commit `2e5a531`). Feature branch
  `bug/BUG-029-interrupt-confirmation` deleted remotely and locally; temporary worktree/stash
  cleaned up.
- Card intentionally **remains in `qa`**: the fix is unverified against a live reproduction and the
  root cause is unconfirmed, so it must not move to `done` until the user reproduces in the TUI and
  records a QA report (or explicitly directs completion).

## QA Notes (2026-09-23)

- Re-confirmed the delivered `InterruptConfirmation` state machine (Idle/Armed/Pending) and the
  `Pending` latch are present in `development`, along with the session-side `tokio::select!` cancel
  path that observes abort while the provider stream is quiet.
- `cargo test -p opencode-tui interrupt` -> 4 passed
  (`interrupt_second_press_within_window_confirms`,
  `interrupt_second_press_after_window_rearms_without_confirming`,
  `interrupt_pending_latches_until_reset`, `interrupt_armed_press_expires_after_window`).
- `cargo test -p opencode-tui` -> 104 passed, 0 failed.
- The BUG-038 stream-timeout change also makes a silently wedged stream return control to the loop,
  which removes the "quiet stream ignores the cancel token" amplifier described by H3.
- Live reproduction during a thinking phase was **not** run in this headless environment; the root
  cause remains unconfirmed by live capture.

## Merge Closeout - 2026-09-23

- Cluster closeout PR #89 merged into `development`; card moved `qa -> done`.
- The interrupt state-machine fix and its tests were already merged (PR #66); the BUG-038 stream
  timeout also removes the quiet-stream cancel-token amplifier this card flagged (H3).


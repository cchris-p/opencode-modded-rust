---
id: "BUG-021"
title: "BUG: TUI exits on its own while typing the first prompt in a new terminal session"
priority: "P1"
type: "bug"
area: "BUG"
spec: "wiki/v1.md"
status: "qa"
created: "2026-09-21"
---

# BUG: TUI exits on its own while typing the first prompt in a new terminal session

## Summary

The TUI can shut down by itself while the user is still typing the very first prompt. There is no
intentional exit action being taken, yet the process leaves the UI and returns to the shell. The user
currently only observes this in a fresh terminal session.

## Reported behavior

- The TUI exits on its own mid-typing, before the first prompt is submitted.
- The user is only able to reproduce it when opening a new terminal session.
- It is not yet clear whether the whole process exits, the TUI detaches, or the input loop simply
  stops receiving events and the run loop ends.
- No error or message has been captured yet; the failure is observed as the TUI disappearing.

## Why this exists

Typing the first prompt is the single most common action in the product. If the app can exit while
the user is still composing that prompt, the core daily-driver workflow is unreliable and the user
cannot trust the TUI to stay open. This also risks losing unsubmitted input.

## Candidate exit paths (code evidence)

These are the known ways the TUI currently leaves its run loop. Each should be ruled in or out.

- Plain `q` exits with no route or focus gate:
  `crates/opencode-tui/src/app/app.rs:470-473` sets `AppState::Exiting` on
  `KeyCode::Char('q')` with empty modifiers, **before** prompt input is handled at
  `app.rs:613-627`. Any first prompt containing the letter `q` (for example "question", "quick",
  "query", "quote") would therefore exit the app mid-typing. **Confirmed root cause** (see
  Investigation below).
- Event-channel disconnect exits the loop:
  `app.rs:277` breaks on `RecvTimeoutError::Disconnected`, and `app.rs:289-291` sets
  `AppState::Exiting` on `TryRecvError::Disconnected`. This triggers if the event sender side is
  dropped (input reader thread or server event listener) while the app is running.
- Input reader thread liveness: `app.rs:171` uses
  `crossterm::event::poll(timeout).unwrap_or(false)` and `app.rs:172-184` treats read errors as
  `None`, so a terminal/input error would silently stop producing key events without breaking the
  thread; confirm whether a fresh TTY path can leave the reader unable to deliver keys.
- Registered but unwired exit keybind: `app_exit_alt` is bound to `Esc` in
  `crates/opencode-tui/src/context/keybind.rs:158`, but `Esc` is not handled as an exit in
  `handle_event`; verify it is not being delivered as an exit path on a new terminal.

## Investigation - 2026-09-21 (ROOT CAUSE CONFIRMED)

### Root cause

Plain `q` was hardcoded as an exit key in the TUI key handler, with no route or prompt-focus gate:

```rust
// crates/opencode-tui/src/app/app.rs (before fix)
if key.code == KeyCode::Char('q') && key.modifiers.is_empty() {
    self.state = AppState::Exiting;
    return Ok(());
}
```

This ran in `handle_event` before prompt input is routed (`app.rs:613-627`), so typing the letter `q`
anywhere in the prompt exited the app mid-typing. That matches the report exactly: the user could not
type `q` at all in the input.

This is also a parity defect. The reference default is
`packages/tui/src/config/keybind.ts:48`: `app_exit: keybind("ctrl+c,ctrl+d,<leader>q", ...)`. Plain `q`
is not an exit key in the reference; `q` under the leader key (`<leader>q`) is. The Rust TUI already
implements the leader path at `app.rs:418` (`Ctrl+X` then `q`), so the hardcoded plain-`q` exit was
redundant and wrong.

### Evidence (live before/after reproduction)

Both binaries were run in a real PTY (Python `pty.fork`) with a fresh workspace and isolated state
dir, launched as `opencode tui`, then sent `q` followed by `Ctrl+D`.

- Before fix (plain `q` handler present): process **exits on `q`**. PTY log shows the TUI enter the
  alternate screen and then immediately restore the terminal and exit:
  `alive_after_q=False`.
- After fix (plain `q` handler removed): process **stays alive on `q`** and only exits on `Ctrl+D`:
  `alive_after_q=True`, `alive_after_ctrld=False`.

The other candidate exit paths (event-channel disconnect, input reader death, focus/paste events,
`Esc` keybind) were not needed to explain the report and are left as non-goals unless they reproduce.

### Fix

- Removed the hardcoded plain-`q` exit block from `crates/opencode-tui/src/app/app.rs`.
- `q` is now routed to the prompt input on `Home`/`Session` routes, as any other printable character.
- Intentional exits are unchanged: `Ctrl+D`, the `Ctrl+X` leader then `q`, and `/exit` (`/quit`, `/q`).

### Regression test

- Added `crates/opencode-tui/src/components/prompt.rs` test `plain_q_types_into_prompt`, asserting
  `Prompt::handle_key` inserts `q` into the input instead of quitting.
- `cargo test -p opencode-tui` passes (32 tests).
- `cargo check -p opencode-tui` clean.

### Verification performed

- `cargo check -p opencode-tui` succeeded.
- `cargo test -p opencode-tui` succeeded (32 passed).
- Live PTY reproduction above confirmed the before/after behavior on freshly built `opencode` binaries.

### Status / next step

- Fix is applied in the working tree only. No branch, commit, or PR was created; the working tree also
  contains unrelated in-progress changes, so do not commit this as part of a broader change set
  without the user's direction.
- Awaiting the user's own `ort-build` + `ort` verification in a new terminal session.

## Hypotheses to test

- `q`-while-typing: a first prompt containing `q` is misread as the quit key because the handler
  runs before prompt input is routed. Test by typing `q` alone and by typing a prompt containing `q`.
- New-terminal-only signal: a fresh terminal emits input that maps to an exit-triggering event, such
  as an initial focus/paste/escape sequence or a terminal-capability response that crossterm reports
  as `Esc`.
- Reader/loop shutdown: on a fresh start the input reader or server event listener thread exits or
  drops its sender, so the run loop sees a disconnect and stops.
- Focus/paste events: `FocusGained`/`FocusLost`/`Paste` are produced at `app.rs:180-182`; confirm
  none of them can lead to an exit or an unhandled state in `handle_event`.

## Investigation goals

- Reproduce the spontaneous exit from a new terminal session and capture the exact key sequence that
  triggers it.
- Determine whether the process exits, detaches, or just stops updating, and whether a session was
  created or persisted before the exit.
- Identify the exact code path responsible: quit key, event-channel disconnect, input reader death,
  or a terminal input event on fresh launch.
- Determine whether the `q` exit gate (route/focus unaware) is the actual cause and whether it also
  affects resumed sessions or only fresh sessions.
- Confirm whether unsubmitted prompt text is lost on the spontaneous exit.

## Scope

- Investigate the TUI event loop and key handling end to end:
  `crates/opencode-tui/src/app/app.rs`, `crates/opencode-tui/src/context/keybind.rs`, and
  `crates/opencode-tui/src/components/prompt.rs`.
- Reproduce on a fresh terminal session, and separately attempt reproduction in an already-running
  session for comparison.
- Confirm whether the exit is intentional-looking (`AppState::Exiting`) or an accidental loop break.
- Keep the first pass focused on the smallest correct fix once the cause is confirmed.

## Non-goals

- Redesigning the prompt input or keybinding system.
- Changing `Ctrl+D` exit or `Ctrl+C` clear semantics (tracked by `FEAT-006`).
- Changing `Esc` interrupt semantics for a running session (tracked by `BUG-019`).
- Adding new keybinding configuration beyond what the fix requires.

## Done when

- The root cause of the spontaneous exit is identified and explained.
- The implemented fix removes the unintended exit while typing a prompt.
- Typing any prompt text, including text containing `q`, does not exit the TUI.
- A fresh terminal session can open the TUI, type a first prompt, and submit it without the app
  exiting on its own.
- Unsubmitted prompt input is not lost to an unintended exit.

## Recommended verification

- Reproduce before the fix from a new terminal: launch the TUI, type a prompt containing `q`, and
  capture the exit.
- Unit-test the key handler so that plain `q` while the prompt is focused inserts text instead of
  exiting.
- `cargo check -p opencode-tui` and `cargo test -p opencode-tui`.
- Live: run `ort-build`, then `ort` in a new terminal; type and submit a first prompt containing `q`
  and confirm the TUI stays open and the prompt is submitted.
- Confirm intentional exits (`Ctrl+D`, `/exit`) still work.

## Open questions for user

- Is this the Rust product launched with `ort`, or the vanilla `opencode` CLI? The item currently
  assumes the Rust TUI in this repo.
- What terminal and shell are you using for the new session?
- Does the exit also happen if you open a new terminal but reuse an existing session, or only on a
  brand-new session/workspace?
- What are you typing around the moment it exits? Does the prompt text contain the letter `q`?
- Does the whole terminal return to the shell prompt, or does it look like the TUI detached?
- Have you ever seen an error, stack trace, or shell message on exit?

## Related Items

- `BUG-019` Escape does not interrupt the running session - same `handle_event` key path; keep the
  interrupt semantics separate from any exit fix.
- `FEAT-006` Make Ctrl+D exit the TUI and Ctrl+C clear the prompt - established the intentional exit
  and clear behavior this bug must not violate.
- `FEAT-014` Enforce a single local TUI server per workspace and increment the server per additional
  ort run - fresh-session server lifecycle context for reproducing on a new terminal.
- `FEAT-016` Remove local TUI server reuse so every ort run starts a fresh server for the activated
  workspace - relevant because the report is specific to new terminal sessions.

## Notes

- Do not close this as a user error until the key path is proven; the `q` handler currently exits
  regardless of route or prompt focus, which is a plausible product defect on its own.
- Capture real runs and key sequences rather than assuming only one cause; if more than one exit path
  is real, fix the primary cause and split the remainder into separate items.

## Merge closeout - 2026-09-21

- PR: https://github.com/cchris-p/opencode-modded-rust/pull/49 merged into `development`
  (merge commit `9c9c89c`).
- Branch `bug/BUG-021-tui-plain-q-exit` deleted remotely and locally.
- Local `development` synced and contains the fix (`app.rs` plain-`q` exit removed, regression test
  present).
- No QA report recorded yet; item remains in `qa` until the user records verification or explicitly
  completes it.

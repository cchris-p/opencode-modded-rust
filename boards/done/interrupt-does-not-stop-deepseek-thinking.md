---
id: "BUG-057"
title: "Interrupt still does not stop a running session; deepseek keeps thinking after Esc"
priority: "P1"
type: "bug"
area: "BUG"
spec: "invariants/coding-session-behavior.md"
status: "done"
created: "2026-09-30"
---

# Interrupt still does not stop a running session; deepseek keeps thinking after Esc

## Summary

Recurrence of the long-running interrupt defect. On the default `deepseek/deepseek-flash` model,
pressing `Esc` (the documented interrupt gesture) does not stop the turn: the model keeps producing
reasoning / the session keeps running, and the user cannot regain control. This is the same class of
user-facing control failure tracked and previously "fixed" by `BUG-019`, `BUG-029`, and `BUG-043`,
which together claimed the server abort path, the TUI confirmation state machine, and the guaranteed
terminal-state/interrupt contract.

The code path that is *supposed* to satisfy `invariants/coding-session-behavior.md` ("Abort must
interrupt the in-flight run even when it is parked on an await that does not observe the prompt
token; the run is cancelled and finalized rather than left wedged") is present in `development`
(`6898e0b`). Since the symptom still reproduces, this card exists to force a live, evidence-backed
answer to *which stage of the interrupt path actually fails*, rather than a fourth code-only guess.

## Reported behavior (user, 2026-09-30)

- "interrupt still doesn't interrupt the session. deepseek keeps thinking."
- The turn does not stop after the interrupt gesture; the model continues `thinking` / streaming.
- Reported as a **major bug** and a persistent source of frustration on the primary daily-driver
  surface.

## Why this matters

Stopping a runaway or wrong generation is the one safety valve for an agent loop that spends tokens
and mutates the workspace. `BUG-019` already argued this. If the interrupt still does nothing on the
product's default model, the product cannot be trusted for daily driving, and three prior fixes
have not closed it.

## Associated board items (pulled in)

Interrupt lineage (the fixes this card may be a regression of, or a gap between):

- `BUG-019` Escape does not interrupt the running session (done) - server abort was a stub; added
  the active-prompt registry, real cancellation, idle broadcast, and provider-valid aborted turns
  (PRs #48, #54). This is the direct ancestor of the current symptom.
- `BUG-029` Esc cannot interrupt a thinking turn; the interrupt hint toggles between states (done) -
  TUI `InterruptConfirmation` state machine (Idle/Armed/Pending) plus a session-side
  `tokio::select!` on the cancel token while the stream is quiet (PR #66). Explicitly left
  unconfirmed against live reproduction.
- `BUG-043` A session run can end without a terminal state, leaving it stuck busy and uninterruptible
  (done) - `run_cancel` token selected in `drain_session_queue`, run future dropped on abort,
  `finalize_run_without_terminal`, plus the monotonic `merge_session_snapshot` fix and durable
  terminal metadata (PRs #118, #127). This is the strongest current abort guarantee.
- `BUG-051` Interrupt-then-continue continuation prompt flashes / never settles (done) - confirmed
  the abort + continuation runs were progressing and fixed the stale-snapshot display clobber
  (PR #127); established that observed `deepseek` runs were slow, not wedged.

Thinking / stream liveliness (candidate amplifiers of "keeps thinking"):

- `BUG-027` Session keeps freezing during thinking mode (done) - full-session synchronous refetch
  storm during reasoning; bounded to ~5 refetches/s. A still-starved event loop can delay or swallow
  the interrupt keypress.
- `BUG-038` DeepSeek reasoning turn stalls mid-turn and never completes (done) - added the 90s
  provider stream idle timeout.
- `BUG-046` A provider stream without idle gaps is unbounded (done) - added the per-step stream
  budget (`OPENCODE_STREAM_BUDGET_MS`, default 15 min). A long reasoning stream is bounded only by
  this budget, not by abort if the cancel token is not observed.
- `BUG-040` Grep tool blocks the async runtime and wedges the server and TUI (done) - async
  starvation can freeze the whole server, including the abort handler.
- `BUG-045` Server runtime errors hidden by /dev/null stdio (done) - captured the server log now used
  to classify live failures.

Continuation / state counterparts:

- `BUG-044` Continuing a session resumes from an earlier user prompt (done).
- `BUG-047` Assistant turn progress not persisted until the run completes (done).
- `FEAT-035` Resume an interrupted session from where it left off (done) - depends on a clean
  interrupted state.
- `BUG-052` Local servers share DB lock contention (done) - multiple servers over one DB can make
  the abort target ambiguous if the TUI is not talking to the server running the turn.

## Current code path (development `6898e0b`) - appears correct on paper

- TUI interrupt: `crates/opencode-tui/src/app/app.rs:571-628`. Gated on non-`Idle` **local**
  session status; second press within the confirmation window calls `client.abort_session`.
- Abort route: `crates/opencode-server/src/routes.rs:4681` -> `abort_active_session_prompt`
  (`:4583`). Cancels the active `SessionPrompt` token (`:4596`), cancels the run-scoped
  `run_cancel` (`:4605-4610`), rejects pending questions (`:4615`).
- Drain/run guarantee: `drain_session_queue` (`:2790`) selects the run future against
  `run_cancel.cancelled()` (`:2833`, `:2841`), drops the run future, and calls
  `finalize_run_without_terminal` (`:2861`).
- Session stream loop: `crates/opencode-session/src/prompt.rs:1334` selects
  `token.cancelled()` against each stream event; the whole-step budget wrapper is at `:1296`.

Because every one of these is present, a fourth code-only fix is not justified. The failure must be
localized with live evidence.

## Candidate failure points (must be narrowed by live capture)

- **C1 - The abort request is never sent.** TUI `Esc` is swallowed or gated: local status already
  `Idle` (server/local desync), confirmation state expired, shell-mode branch, or event-loop
  starvation delaying the second keypress past the confirmation window. The user then sees the
  model "keep thinking" while the client did nothing.
- **C2 - Abort is sent but the server returns `aborted:false`.** `ACTIVE_PROMPTS` does not hold the
  session (different server instance for this port/workspace, run handled by a path other than
  `run_prompt_turn`, or a registration race). The TUI would show "Nothing to interrupt".
- **C3 - Abort fires but the provider stream is not actually dropped.** The run is parked before
  the select (e.g. `provider.chat_stream(request).await`, `ensure_title`) and neither token is
  observed; or the reasoning stream is being drained by a detached task. The model keeps generating
  server-side and the transcript keeps growing.
- **C4 - The run stops but the session never reports idle.** The TUI keeps showing a running /
  thinking state, so the user concludes interrupt failed even though generation stopped. Candidate:
  `update_task` / snapshot merge re-asserting a running snapshot, or the status broadcast being
  missed.

These are not mutually exclusive; the live capture must name which one (or combination) applies.

## Investigation - 2026-09-30 (headless abort probes)

Tested the real binary (`target/debug/opencode`, built 2026-09-30 20:24, from `development` at
`6898e0b`) against `deepseek/deepseek-flash`, using an isolated HOME/DB (never the operator's DB) and
driving the HTTP API directly. Three probes:

1. **Abort during provider connect / pre-token phase.** `prompt_async`, then abort ~4s in while the
   session was `busy` and no content had been emitted. `POST /session/{id}/prompt/abort` returned
   `{"aborted": true}`; `GET /session/status` returned `busy=false`/`idle=true` within ~1s; no
   further content growth.
2. **Abort during the quiet thinking phase.** `deepseek-flash` emitted nothing for 45s (status stayed
   `busy` — the "keeps thinking" experience). Aborting at 45s again returned `{"aborted": true}` and
   the status went idle; content never grew after abort.
3. **Abort during active streaming.** With a reliable streaming prompt, waited until 203 chars of
   assistant text were streaming, then aborted. Peak content before abort `203`, after abort `203`
   (`grew_after_abort=false`), idle within ~1s.

**Result: the server-side abort path works in every phase tested (connect, quiet thinking, active
streaming): the request cancels the run, stops content growth, and returns the session to idle.** This
directly clears candidate **C2** (server returns `aborted:false`) and **C3** (stream not dropped) for
these phases, and rules out a simple server-side regression of `BUG-019`/`BUG-043`.

Consequences for the remaining candidates:

- **C1 (abort never sent) is now the leading candidate.** If the TUI does not issue the request, the
  server never gets a chance to stop the turn, matching "keeps thinking". The static read of the TUI
  path did not reveal an obvious gate bug, so this needs a live capture from the TUI process
  (whether `POST /session/{id}/abort` is received and what it returns).
- **C4 (stops but never shows idle)** remains possible but is not supported by the server probes,
  which show idle broadcasts promptly.
- **Stale binary** must be excluded: several long-lived `opencode serve` processes were observed on
  ports 3187-3192 started between 2026-09-29 17:03 and 2026-09-30 21:24. A TUI attached to a server
  started before the current build would not have the latest interrupt fixes. Record which server the
  failing session used.

### How to complete the localization (needs the live TUI)

The remaining step is a focused capture on the TUI side, not more server probing:

1. Confirm the failing run's server was started from the current `target/debug/opencode` (rebuilt via
   `ort-build`), e.g. `opencode debug paths` / process start time.
2. During a `deepseek` thinking turn, press `Esc` twice and capture whether the TUI actually issues
   `POST /session/{id}/abort` and what the response is (`aborted:true|false`, or a toast like
   "Cannot interrupt" / "Nothing to interrupt").
3. Read `…/traces/server.log` (and the isolated trace if enabled) around the abort.

## Evidence needed (evidence-first)

1. Reproduce on the actual binary the user was running (`ort-build`, then `ort`), on
   `deepseek/deepseek-flash`, during a thinking turn.
2. Confirm whether the TUI issued `POST /session/{id}/abort` and what it returned
   (`aborted:true|false`), plus the timing of the two `Esc` presses relative to the confirmation
   window.
3. Read `…/traces/server.log` (BUG-045) around the abort: cancellation logged
   (`Stream cancelled` / `Prompt loop cancelled`), `finalize_run_without_terminal`, panics, or
   nothing at all.
4. Poll `GET /session/{id}/status` before/after abort and inspect the persisted assistant message
   for a terminal record.
5. Identify the exact process/port/workspace (`lsof`, `pgrep`) so the abort target is unambiguous
   (guards C2 and the shared-DB ambiguity from `BUG-052`).

## Scope

- Localize the failing stage of the interrupt path with live evidence (TUI -> route -> cancel ->
  terminal state -> status broadcast).
- Fix the confirmed stage so `Esc` reliably stops a running `deepseek` thinking turn.
- Add a regression test at the confirmed failure point (not only at an existing guard).
- Ensure the interrupt gesture leaves a provider-valid, resumable session (`BUG-019` contract).
- Update this card with the confirmed root cause and evidence.

## Non-goals

- Changing the `Esc` double-press semantics or keybinding unless evidence shows it is the defect.
- Re-fixing `BUG-027`/`BUG-038`/`BUG-046` stream-response behavior unless evidence shows it blocks
  cancellation.
- Provider transport rewrites beyond the confirmed cause.
- Broad TUI event-loop redesign.

## Done when

- There is an evidence-backed explanation of which stage fails and why, naming the artifacts
  (server log lines, request/response, status transitions) that show it.
- Pressing `Esc` twice during a `deepseek` thinking turn stops generation and returns the session
  to `idle` with a durable terminal record.
- A regression test covers the confirmed failure mode and fails before the fix.
- `cargo check` and `cargo test` pass for the touched crates.

## Recommended verification

- `ort-build`, then `ort`; start a long reasoning turn on `deepseek/deepseek-flash`; press `Esc`
  twice; confirm streaming stops and status returns to idle within a bounded time.
- Confirm a subsequent prompt in the same session works.
- `cargo test -p opencode-tui interrupt`; `cargo test -p opencode-server`;
  `cargo test -p opencode-session`.
- Live HTTP probe: `POST /session/{id}/abort` during a streaming turn and assert
  `GET /session/{id}/status` -> `busy=false`.

## Notes

- Created 2026-09-30 from the user's report; treated as the recurrence of the interrupt defect
  after `BUG-019`/`BUG-029`/`BUG-043` were all closed.
- The three prior cards shipped code-only fixes; `BUG-029` was explicitly merged **without** live
  reproduction and its root cause was never confirmed. This card deliberately starts from live
  evidence before changing code.
- Relevant files: `crates/opencode-tui/src/app/app.rs`,
  `crates/opencode-tui/src/components/prompt.rs`, `crates/opencode-server/src/routes.rs`,
  `crates/opencode-session/src/prompt.rs`, `crates/opencode-provider/src/stream.rs`.
- Runtime bounds and server-log locations are documented in `README.md` and `docs/`.

## Dev Notes - 2026-09-30 (TUI gate hardening)

Probes (above) cleared the server side, so the TUI client path was hardened as the leading candidate
C1.

- `crates/opencode-tui/src/app/app.rs` (`session_interrupt` handler): removed the gate that skipped
  the abort whenever the TUI's cached local `SessionStatus` was `Idle`. The handler now always
  registers the confirmation and issues `POST /session/{id}/abort`; the server is authoritative and
  answers `aborted:false` when nothing ran, which the TUI surfaces as "Nothing to interrupt".
- Added `interrupt_ignores_cached_status` as a named policy seam so a local-status gate cannot be
  silently re-introduced, and `interpret_abort_response` to classify the server response (missing
  field latches, explicit `false` reports nothing to interrupt).
- Added a `trace::line` diagnostic on every issued abort (session id + cached status) so future live
  captures show whether the TUI sent the request.
- Behavior kept: the confirmation stays latched after a fired abort until the session status changes,
  and the "no server connection" / request-error toasts are unchanged.

This is a robustness fix, not a proven root-cause fix: it removes a real way the interrupt can be
silently dropped (cached status desync) but the live TUI reproduction of C1 was not captured. The
remaining live check (does the request reach the server during a stuck/thinking turn?) still applies.

## Verification - 2026-09-30

- `cargo check -p opencode-tui` clean.
- `cargo test -p opencode-tui interrupt` -> new
  `interrupt_is_not_gated_by_cached_local_status`, `abort_response_missing_field_latches_confirmation`,
  `abort_response_explicit_false_reports_nothing_to_interrupt` pass (6 passed).
- `cargo test -p opencode-tui` -> 179 passed, 0 failed.
- Not run: live `ort` reproduction (needs the user's TUI; see the live-localization steps above).

## Related Items

- `BUG-019`, `BUG-029`, `BUG-043`, `BUG-051` - the interrupt/terminal-state fixes this card may be
  a regression of; see the "Associated board items" section above.
- `BUG-027`, `BUG-038`, `BUG-040`, `BUG-046` - thinking/stream/runtime-starvation amplifiers.
- `BUG-044`, `BUG-047`, `FEAT-035` - continuation and interrupted-state counterparts.
- `BUG-052` - shared-DB / multi-server ambiguity that can make the abort target wrong.

## Closeout - 2026-10-07

- Landed the TUI gate hardening (removed the cached-status gate in the `session_interrupt` handler,
  added `interrupt_ignores_cached_status` / `interpret_abort_response` and the abort trace line)
  directly on `development` (no PR), per user direction.
- Card marked `done` at user request. Residual: still a robustness fix for candidate C1, not a
  proven root cause; the one open live check is pressing `Esc` twice during a `deepseek` thinking
  turn and confirming `POST /session/{id}/abort` is issued and returns `aborted:true`.
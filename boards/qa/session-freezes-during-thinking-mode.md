---
id: "BUG-027"
title: "Session keeps freezing during thinking mode"
priority: "P1"
type: "bug"
area: "BUG"
spec: "invariants/coding-session-behavior.md"
status: "qa"
created: "2026-09-21"
---

# Session keeps freezing during thinking mode

## Summary

While a model is producing reasoning (thinking), the TUI session **freezes for the duration of the
thinking phase**: input is not accepted and no progress is visible, and the UI only catches up and
finishes the task once reasoning completes. It reproduces on every thinking turn with `/thinking` on,
on the default `deepseek/deepseek-v4-flash` model. The likely cause is the reasoning-streaming update
path, not the model itself: every `session.updated` event triggers a synchronous, full-session HTTP
refetch inside the TUI event loop, and reasoning deltas drive those events at the maximum throttle
rate (every ~50ms) for the whole thinking phase, saturating the loop until the stream ends.

This card tracks investigation and fix. A few verification details (server-side responsiveness,
transcript capture, `/thinking`-off comparison) remain open.

**Symptom correction (2026-09-21).** The original report described a brief stutter that recovered
mid-turn. The user corrected this: the session freezes for the whole reasoning phase and the UI is
unresponsive (no input, no visible progress), then catches up and the task completes normally. This
is a sustained event-loop starvation, not a discrete repeatable stall, which shifts weight toward the
blocking refetch path (H1/H2) and away from render cost (H4) as the primary cause.

## Reported behavior

- The session **freezes while the model is in thinking mode**: the UI is unresponsive (input is not
  accepted, progress is not visible) for the duration of the thinking phase, then catches up and the
  task completes normally. It is not a permanent hang requiring restart, but it is also not a brief
  stutter that recovers mid-turn.
- The freeze ends when reasoning finishes (the task still completes), so it is bounded by the length
  of the reasoning phase rather than by a crash or deadlock.
- It happens on **every** thinking turn, not only long/deep reasoning, so it is systematic rather
  than a one-off provider hiccup; longer reasoning makes the freeze longer.
- Reproduced with `/thinking` **on** (reasoning rendered while streaming) on
  `deepseek/deepseek-v4-flash` (the product default).
- The freeze aligns with reasoning output rather than tool execution or normal assistant text.

User-supplied on 2026-09-21 (corrected): freezes for the whole thinking phase, UI unresponsive with
no visible progress, task completes once reasoning ends / every thinking turn / `/thinking` on /
`deepseek/deepseek-v4-flash`.

## Why this exists

Reasoning is the longest high-frequency stream in a normal agent turn, and V1 targets a responsive
personal daily-driver TUI (`docs/opencode-tui.md`: "UI changes should preserve scroll stability and
low CPU usage"). If the TUI cannot stay responsive while the model thinks, the primary surface of
the product fails exactly when the user is waiting on the model. This is a UI/runtime responsiveness
defect, not a missing feature.

## Code evidence

Update cadence during reasoning (server side):

- Reasoning deltas mutate the assistant part and then call the throttled session-update emitter:
  `crates/opencode-session/src/prompt.rs:1157-1168` (ReasoningDelta), appending via
  `append_delta_part` (`prompt.rs:1443-1458`).
- The emitter only rate-limits to **50ms**; it does not coalesce or send incremental parts:
  `maybe_emit_session_update` (`prompt.rs:1430-1441`).
- Each emit **clones the entire session** and pushes it through an unbounded channel:
  server update hook at `crates/opencode-server/src/routes.rs:1812-1814`.
- A separate task applies the snapshot under a lock and broadcasts a `session.updated` event with no
  payload: `crates/opencode-server/src/routes.rs:1795-1811`.

Update handling (TUI side):

- The TUI reacts to `session.updated` by refetching the whole session, throttled to **50ms**, with a
  pending-sync fallback processed on the next 16ms tick:
  `crates/opencode-tui/src/app/app.rs:746-758` and `app.rs:797-811`.
- `sync_session_from_server` performs **two blocking HTTP GETs** (`get_session` then `get_messages`)
  and replaces the entire message list, synchronously inside the event loop:
  `crates/opencode-tui/src/app/app.rs:3633-3664`.
- The API client is a `reqwest::blocking::Client` with a 30s timeout:
  `crates/opencode-tui/src/api.rs:384-385`; `get_session` `api.rs:413-426`; `get_messages`
  `api.rs:979-992`.
- The main loop processes input then optionally draws with a 16ms tick and a 256-event per-frame cap:
  `crates/opencode-tui/src/app/app.rs:42-43`, `app.rs:271-325`.

Render path (potential compounding factor):

- Reasoning parts are rendered per draw, and when shown may be expanded while streaming:
  `crates/opencode-tui/src/components/session.rs:687-733`. Note `BUG-022` already tracks the
  collapsed-to-count rendering of reasoning, which affects how much is drawn here.

Net: during thinking, the server emits `session.updated` at up to ~20/s; the TUI responds with up to
~40 blocking HTTP round trips per second, each carrying the full session and a reasoning body that
grows for the length of the turn. Because those calls are synchronous in the event loop, input and
redraw are starved → the session appears frozen.

## Hypotheses (confidence-ranked)

- **H1 (93%) - Full-session synchronous refetch on every update blocks the TUI event loop.** Evidence:
  `app.rs:746-758`, `app.rs:3633-3664`, blocking client `api.rs:384-385`. The loop cannot process
  input or draw while the two GETs are in flight. Because reasoning emits continuously at the 50ms
  throttle and each refetch slows as the transcript grows, the loop stays saturated for the whole
  thinking phase rather than recovering between updates, which matches the reported sustained freeze
  (raised from 90% after the 2026-09-21 symptom correction).
- **H2 (80%) - Cost grows with reasoning length, so longer thinking degrades further.** Each
  snapshot/response carries the whole growing reasoning text; up to ~20 refetches/second make total
  transfer O(n²) over the turn. Raised from 70%: it explains both the systematic occurrence and the
  fact that the freeze duration tracks the reasoning length.
- **H3 (55%) - Redundant server work per emit.** Full-session clone plus unbounded channel push per
  50ms emit (`routes.rs:1812-1814`), even though only a small event is broadcast to the TUI.
- **H4 (40%) - Rendering amplifies it.** Re-laying-out the expanded/streaming reasoning block every
  draw (`session.rs:687-733`) adds cost on top of the network path. Lowered from 60% after the
  correction: render cost alone would still let input be read and the spinner animate, so it is a
  compounding term, not the cause of a total input/progress freeze.
- **H5 (15%) - Reasoning arrives in large bursts** from the provider, producing a long single
  application+emit and a correspondingly long blocking refetch.
- **H6 (30%) - Lock contention** on `context.session` (TUI) or the server `sessions` mutex while the
  update task holds it during streaming. Raised from 15%: sustained (not discrete) stalling is also
  consistent with server-side serialization of `GET /session/{id}/message` behind the streaming
  update task's lock (`routes.rs:1794-1804`).

## Open questions

Answered by user on 2026-09-21:

1. ~~Brief stutter vs. full hang?~~ **Sustained freeze for the whole thinking phase; task still
   completes once reasoning ends.**
2. ~~Every thinking turn or only long reasoning?~~ **Every thinking turn.**
3. ~~Is `/thinking` shown?~~ **On.**
4. ~~Which model/provider?~~ **`deepseek/deepseek-v4-flash`.**

Still open:

5. Is the server also unresponsive during the stutter, or only the TUI (does another session stall)?
6. Can a transcript export of a freezing session be captured to `docs/transcripts/` for timing
   evidence, and does the stutter persist with `/thinking` off (to separate H1 from H4)?

## Fix confidence and measurement plan

Added 2026-09-21. These are **pre-measurement** estimates from code reading only; nothing has been
instrumented or reproduced under measurement yet. They are deliberately separated into "is this the
cause" vs. "will the first fix remove the symptom."

- **~90%** that the synchronous full-session refetch is *a* real contributor. The path is
  unambiguous: every `session.updated` (up to ~20/s during reasoning) triggers two blocking HTTP GETs
  inside the event loop (`app.rs:746-758`, `app.rs:3633-3664`), which is sufficient to starve the loop
  for the whole reasoning phase.
- **~75%** that it is the *dominant* cause of the freeze every thinking turn (raised from ~60% after
  the 2026-09-21 symptom correction). With `/thinking` on the expanded reasoning block keeps growing
  and is re-laid-out on redraws (`session.rs:687-733`), but render cost alone would still let input be
  read and the spinner animate; a total input/progress freeze fits the blocking I/O path better. The
  two have not been fully separated.
- **~65%** that stopping the per-event full refetch *alone* removes the freeze. If rendering is also
  a major factor, the render/coalescing work is required too; combined, that lifts confidence to
  **~80-85%**.
- **~90%** that the *investigation direction* is correct — i.e. measuring this path will identify the
  real cause. Confidence in a correct diagnosis is much higher than confidence in the first fix being
  complete.

Measurement plan to raise confidence above ~85% (in rough order of value):

1. Instrument `sync_session_from_server` (`app.rs:3633`) to log duration, and count `session.updated`
   events during one thinking turn. Confirm whether per-sync latency spikes coincide with the
   stutters.
2. Run the same prompt with `/thinking` off and compare. If the stutter disappears, rendering shares
   the cause (H4 rises); if it stays, the refetch path is implicated (H1/H2 rise).
3. Time a raw `GET /session/{id}/message` against the server while it is streaming, and correlate
   response size with latency to test the O(n²) claim in H2.
4. Separate the two GETs (`get_session` `api.rs:413` vs `get_messages` `api.rs:979`) to see which one
   dominates the stall.

Decision rule: if refetch latency correlates with the stutters and `/thinking` off does not remove
them, the fix confidence jumps and the incremental-update approach is the right first move. If
`/thinking` off also removes the stutter, treat H1 and H4 as joint causes and scope both.

## Scope

- Establish whether the freeze is caused by the synchronous full-session refetch path above, and
  measure the per-update cost during reasoning.
- Make the TUI stay responsive while reasoning streams, ideally by consuming incremental updates
  instead of refetching the full session per event.
- Remove or bound redundant full-session clones/refetches on the streaming update path.
- Add a regression test at the confirmed failure point (event loop not blocked by sync refetch;
  reasoning streaming does not degrade responsiveness).
- Record the confirmed root cause and evidence on this card.

## Non-goals

- Re-fixing reasoning display semantics; `BUG-022` owns collapsed/count reasoning rendering.
- Provider transport rewrites beyond what the confirmed cause requires.
- Broad TUI architecture redesign or a full SSE/eventing overhaul in this card.
- Changing the model default or disabling thinking.

## Done when

- There is an evidence-backed explanation of why the session freezes during thinking mode, naming
  the loop stage and the cost that grows with reasoning.
- The TUI remains responsive (accepts input and redraws) throughout a long reasoning stream.
- A test or measurement reproduces the freeze before the fix and shows it resolved after.
- `cargo check` and `cargo test` pass for the touched crates (at minimum `opencode-tui`,
  `opencode-server`, `opencode-session`).

## Recommended verification

- Reproduce: `ort-build`, then `ort`, and send a prompt that produces long reasoning; observe
  responsiveness and input handling throughout the thinking phase.
- Instrument timing around `sync_session_from_server` (`app.rs:3633`) and count `session.updated`
  events during reasoning to confirm the refetch storm.
- Verify the reasoning payload size grows across the turn and correlates with per-sync latency.
- After the fix, confirm input is accepted and the UI redraws during thinking, and that CPU/transfer
  no longer scale with reasoning length.
- `cargo test -p opencode-tui`; `cargo check -p opencode-tui -p opencode-server -p opencode-session`.

## Dev Notes

- 2026-09-21: Corrected the recorded symptom from "brief stutter that recovers" to a sustained
  freeze for the whole thinking phase (no input, no visible progress; task completes when reasoning
  ends), and re-weighted the hypotheses (H1 93%, H2 80%, H6 30%, H4 40%).
- 2026-09-21: Added env-gated tracing to measure the sync-refetch duty cycle:
  - New `crates/opencode-tui/src/trace.rs`: a background sampler thread writes one line per second
    with counters for loop iterations, `session.updated` deliveries, syncs and cumulative sync time,
    draws and cumulative draw time, key events, and starvation gaps. Running on its own thread keeps
    sampling while the main event loop is blocked.
  - `crates/opencode-tui/src/lib.rs`: declares `trace` and calls `trace::init()` in `run_tui`.
  - `crates/opencode-tui/src/app/app.rs`: instruments the run-loop iteration/gap, `draw`, key events,
    `SessionUpdated`, and splits `get_session` vs `get_messages` timing in `sync_session_from_server`.
  - Enabled only when `OPENCODE_TUI_TRACE=<path>` is set; behavior is unchanged otherwise.
- Verification: `cargo check -p opencode-tui -p opencode-cli` and `cargo build -p opencode-cli`
  passed; `cargo fmt -p opencode-tui -- --check` clean; `cargo clippy -p opencode-tui --all-targets`
  reports no new warnings at the changed code (pre-existing workspace warnings remain).
- 2026-09-21: Change delivered in PR #60 (base `development`): `feat(tui): add env-gated stall
  tracing for BUG-027`. Card moved to `qa` for local verification of the tracing.
- Status: **root cause not yet confirmed; no fix implemented.** This change delivers the measurement
  tooling and the corrected symptom only. Next step is the run protocol: capture
  `OPENCODE_TUI_TRACE` logs with `/thinking` on and off, then read the per-second `SAMPLE` duty cycle
  (`draws`/`keys` vs `sync_ms`) to decide H1/H2 versus H4.
- 2026-09-21: PR #60 merged into `development` (merge commit `217fd3f`); branch
  `bug-027-thinking-freeze-tracing` deleted remotely and locally. Card intentionally **remains in
  `qa`** pending a QA report: the merge delivered the tracing tooling only, so the bug is not fixed
  and the card must not move to `done`. Promote to `done` only after the trace run confirms the cause
  and the responsiveness fix lands with before/after evidence.

## Related Items

- `BUG-022` `/thinking` toggle shows a line count instead of actual reasoning - same streaming/render
  path; related but display-only, not responsiveness.
- `BUG-023` Root-cause why the plan-mode session stalled after tool calls - adjacent
  prompt-loop/update-path investigation.
- `BUG-025` Concurrent server sync deletes sessions and messages - full-snapshot session sync
  concerns; related data-freshness risk on the same path.
- `BUG-019` Escape does not interrupt the running session - abort path intersects with how streaming
  updates are emitted.
- `FEAT-028` Add a hide-tool-calls toggle - same TUI streaming/rendering performance surface.
- `PHASE-001` V1 daily-driver hardening.

## Notes

- Relevant files: `crates/opencode-tui/src/app/app.rs`,
  `crates/opencode-tui/src/api.rs`,
  `crates/opencode-tui/src/components/session.rs`,
  `crates/opencode-session/src/prompt.rs`,
  `crates/opencode-server/src/routes.rs`.
- The 50ms throttle exists on both sides (emit at `prompt.rs:1437`, TUI sync at `app.rs:749`), so the
  two subtly reinforce each other: the TUI refetches almost exactly as often as the server emits.
- A likely minimal fix direction is to stop refetching the full session on every `session.updated`
  and instead apply incremental part updates from the event (or debounce/coalesce refetches), then
  render from the already-synced state. Confirm with measurement before committing to a design; the
  confidence estimates and measurement plan are in **Fix confidence and measurement plan** above.
- Report characteristics (2026-09-21, corrected): the session freezes for the whole thinking phase
  (no input, no visible progress) and the task completes once reasoning ends; every thinking turn;
  `/thinking` on; `deepseek/deepseek-v4-flash`. Because the freeze blocks input and redraw rather
  than merely lowering frame rate, the blocking refetch path (H1/H2) is the prime suspect and
  rendering (H4) is treated as a compounding term; measuring the sync-refetch duty cycle is the
  fastest confirmation.
- Instrumentation added 2026-09-21 behind `OPENCODE_TUI_TRACE=<path>`: a background sampler logs
  per-second counters (event-loop iterations, `session.updated` events, syncs, sync time, draws,
  draw time, keys) so the duty cycle is captured even while the main loop is frozen. Per-sync lines
  split `get_session` vs `get_messages` duration and record message count.

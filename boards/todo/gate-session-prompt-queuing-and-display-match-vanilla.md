---
id: "GATE-001"
title: "Gate: prompt queuing and queue display must match vanilla OpenCode exactly"
priority: "P0"
type: "gate"
area: "GATE"
spec: "invariants/message-queuing.md"
status: "todo"
predecessors: ""
created: "2026-09-21"
---

# Gate: prompt queuing and queue display must match vanilla OpenCode exactly

## Summary

Hard gate. No queue-related story may be completed until the session prompt queuing system and every
queue visual surface match vanilla OpenCode exactly.

Today the Rust product has **no prompt queue**: concurrent prompts to one session spawn concurrent
runs, race each other, and can drop or reorder session history; no client or TUI surface represents
queued work; and `prompt_async` reports `queued` without executing. See
`invariants/message-queuing.md` for the current-behavior evidence and target invariants.

This card documents the vanilla behavior to target (section below), the current Rust gaps, and the
concrete acceptance criteria. It is the refinement artifact for the blocked stories.

## Gate requirement

> Queuing system and visual display of queue must match exactly opencode vanilla.

Parity is judged against the frozen reference source (below), not a paraphrase. Where the Rust runner
has no equivalent mechanism (for example mid-turn steer injection), the observable behavior and
display must still match; internals that cannot be reused are reference-only and called out under
"Parity decisions (resolved)".

## Vanilla behavior to target (frozen reference)

### 0. Source of truth

`$HOME/repos/opencode-modded` at commit `e62912b5d18b73316c7bfd6e894b040698f6c880` (per `AGENTS.md`).
All line references below were verified at that commit.

### 1. Follow-up delivery model (core)

- The delivery mode is a two-value enum: `Delivery = "steer" | "queue"`
  (`packages/schema/src/session-delivery.ts:5`).
- `session.prompt` admits a prompt with an explicit or default delivery, is idempotent by message ID,
  and wakes the runner unless `resume === false` (`packages/core/src/session.ts:359-380`).
- Admitted prompts are stored as session inputs carrying an `admitted_seq`; `promoted_seq` stays null
  until the runner promotes them (`packages/core/src/session/input.ts:118-167`).
- `hasPending(db, sessionID, delivery)` reports whether un-promoted inputs of a delivery exist
  (`input.ts:170-186`).
- `promoteNextQueued` promotes **exactly one** queued input: the un-promoted `queue` row with the
  lowest `admitted_seq` (FIFO) (`input.ts:268-285`).
- `promoteSteers` promotes **all** un-promoted `steer` rows with `admitted_seq <= cutoff`, ordered by
  `admitted_seq` (`input.ts:245-266`).
- Runner precedence and draining (`packages/core/src/session/runner/llm.ts:190-193,394-415`):
  - At turn start it checks pending steers first, then queues: `hasSteer ? "steer" : hasQueue ?
    "queue" : undefined`.
  - A `queue` promotion promotes one queued input and then all eligible steers up to the current
    sequence cutoff; both count toward starting a turn.
  - After each outer loop it re-checks the queue and continues until nothing is pending.
- Shipped-client reality at this commit: the app **normalizes `followup: "queue"` back to `"steer"`**
  (`packages/app/src/context/settings.tsx:354-355,373-374,376-378`), and the TUI submits prompts
  without an explicit delivery, so the effective default is `steer`. The FIFO `queue` path exists in
  core but is not selected by the shipped clients.

### 2. TUI queued-message display (the "visual display of queue")

- Queued boundary algorithm (`packages/tui/src/routes/session/index.tsx:244-250`):
  - `completed` = index of the last assistant message that has `time.completed`.
  - `pending` = index of the last assistant message after `completed` that has **no** `time.completed`.
  - A user message is queued when `pending !== undefined && messageIndex > pending`
    (`index.tsx:1388`).
- Rendering (`index.tsx:1388-1391,1438-1453`):
  - When queued, the user message's timestamp row is replaced by a badge rendered as
    `<span style={{ bg: color(), fg: queuedFg(), bold: true }}> QUEUED </span>`.
  - `color = agent.color(message.agent)`; `queuedFg = selectedForeground(theme, color)`.
  - The badge text is the uppercase literal `QUEUED` with exactly one leading and one trailing space.
  - `metadataVisible = queued || showTimestamps`, so the badge row is present even when timestamps are
    hidden.
  - Required properties: agent-colored background, contrast-selected foreground, bold, replaces (not
    augments) the timestamp, and appears on every user message after the in-flight assistant.
- Keybind: `session_queued_prompts` is bound to `<leader>q` / "Manage queued prompts"
  (`packages/tui/src/config/keybind.ts:102,309`). In the frozen tree this keybind has **no** handler in
  `packages/tui/src`; the working "manage queued prompts" surface is in the CLI run footer (below).

### 3. CLI run queue and footer (serial queue with management)

- `packages/opencode/src/cli/cmd/run/runtime.queue.ts` implements a serial prompt queue that drains one
  turn at a time; ordinary prompts submitted while an ordinary turn is active are appended to both the
  working queue and a `queued` list that is exposed for edit/removal until their turn begins
  (`runtime.queue.ts:16-24,280-310,323-330`).
- Queue events drive the footer: `{ type: "queue", queue }` (count) and
  `{ type: "queued.prompts", prompts }` (list) (`runtime.queue.ts:73-84`).
- Footer state exposes `queue: number` (`packages/opencode/src/cli/cmd/run/types.ts:83-92`) and entries
  `FooterQueuedPrompt { messageID, partID, prompt }` (`types.ts:45-49`).
- Management UI appears only while `queued.length > 0`: a "Manage queued prompts" command with footer
  `` `${count} queued` `` that opens the `queued-menu` route
  (`packages/opencode/src/cli/cmd/run/footer.command.tsx:414-431`,
  `packages/opencode/src/cli/cmd/run/footer.view.tsx:552-561,576-590`).
- Removal: `onQueuedRemove(messageID)` drops the waiting prompt from both the queue and the queued list
  (`runtime.queue.ts:323-330`, `footer.ts:371-377`).
- Queue empties and the menu auto-closes when no queued prompts remain
  (`footer.view.tsx:576-590`, `runtime.queue.ts:280-310`).

## Current Rust implementation and gaps

- **No queue.** `POST /session/{id}/prompt` validates then `tokio::spawn`s a run per request and returns
  `started` immediately; there is no per-session serialization (`crates/opencode-server/src/routes.rs:1747`,
  `routes.rs:2069-2073`).
- **`ACTIVE_PROMPTS` is last-writer-wins.** A single map slot per session, overwritten by each run; it
  exists for abort, not ordering (`routes.rs:282` region / `routes.rs:1983-1986`, `routes.rs:2043-2050`).
- **The busy guard is ineffective across requests.** `SessionPrompt` has `assert_not_busy` / `start` /
  "Session already running" (`crates/opencode-session/src/prompt.rs:178,734,817`) but its state map is
  created per instance (`prompt.rs:137-149`) and a new `SessionPrompt` is built per request
  (`routes.rs:1976-1982`), so concurrent requests each see an empty state.
- **`prompt_async` is a stub.** It appends a user message and an empty assistant message, persists, and
  returns `{"status":"queued"}` without executing (`routes.rs:2280-2313`).
- **No queued run status.** `SessionRunStatus` is only `{ Idle, Busy, Retry }` (`routes.rs:282-286`).
- **No TUI queue display.** `render_user_message` emits message text then a timestamp when
  `show_timestamps` is on; there is no badge (`crates/opencode-tui/src/components/session_message.rs:15-74`).
  The message loop has the index available (`crates/opencode-tui/src/components/session.rs:560-589`) and
  already computes an active assistant via `finish.is_none()` (`session.rs:606-608`), so the vanilla
  boundary is computable from `finish` / `completed_at` on `Message`
  (`crates/opencode-tui/src/context/session_context.rs:21-35`).
- **No `session_queued_prompts` command.** The leader key is `Ctrl+X` (`crates/opencode-tui/src/app/app.rs:435-437`,
  `crates/opencode-tui/src/context/keybind.rs:7-45`); no queued-prompts binding or dialog exists.
- **No CLI run queue footer.** The CLI has a `run` command but no interactive serial-queue/footer surface
  (`crates/opencode-cli/src/main.rs:81-82` and chat runner at `:1650`); the vanilla footer parity applies
  only once/where that surface exists.

## Target behavior for the Rust product (derived, implementable)

1. Accept a prompt while a session is running: do not spawn a second run and do not reject it.
2. Process one turn at a time per session; accepted prompts are ordered by admission sequence (FIFO).
3. Materialize the accepted user message in the session transcript immediately so every client can
   render it, and mark it pending until the runner consumes it.
4. TUI renders the vanilla `QUEUED` badge exactly as specified in section 2, using the message's agent
   color, on every user message after the in-flight assistant.
5. Provide the "Manage queued prompts" surface in the CLI run footer matching section 3 (command shown
   only when queued prompts exist, count in the footer, list, removal), where that surface exists.
6. Never report `queued` without executing; `prompt_async` must execute or be removed/aliased.
7. Abort/cancel reaches both the active turn and the waiting prompts.

## Parity decisions (resolved)

- **Delivery modes are reference-only; FIFO after the current turn is the required observable.** The
  frozen clients effectively use `steer` and normalize `queue` away, but the Rust runner has no mid-turn
  injection mechanism. This gate therefore requires the observable contract: one turn at a time,
  admitted order preserved, queued user messages displayed and processed after the current turn. Core
  `delivery` internals are documented for fidelity only; strict mid-turn steer parity would be a
  separate card.
- **The TUI `QUEUED` badge and the CLI run-footer manage surface are both in scope.** The TUI keybind
  `<leader>q` is a no-op in the frozen tree, so parity is the badge plus the CLI footer management
  surface, not a TUI dialog that vanilla never implemented.
- **The CLI footer half is conditional.** It applies where the Rust CLI has an interactive run surface;
  if none exists at implementation time, record that as a documented partial and keep the badge/server
  halves in scope.

## Scope

- Server-side per-session prompt queue: one active turn per session, FIFO admission order, no dropped or
  replaced prompts.
- Queue-aware acceptance response (`started` vs `queued`), a queued run status, and immediate
  materialization of accepted user messages.
- TUI `QUEUED` badge parity per section 2, including boundary computation from assistant completion.
- CLI run-footer queue count/list/manage parity per section 3 where that surface exists.
- Shared visibility of queued prompts and results for TUI and CLI.
- Complete abort/cancel for the active turn and waiting prompts.

## Non-goals

- Mid-turn steer injection into a running turn (reference-only; see Parity decisions).
- General upstream sync or broad OpenCode parity beyond queuing and its display.
- Non-queue TUI layout/theming work.
- Concurrent multi-server database sync correctness (`BUG-025`), which is adjacent, not gated here.
- Attach/detach or server-reuse lifecycle changes.

## Done when

- [ ] The server enforces one active prompt turn per session; subsequent accepted sends queue in
      admission order and are never dropped, replaced, or interleaved.
- [ ] A prompt endpoint reports `started` vs `queued` accurately, and no endpoint reports `queued`
      without executing.
- [ ] Run status can express queued work and is readable by clients.
- [ ] The TUI renders the `QUEUED` badge exactly as vanilla: on every user message after the in-flight
      assistant boundary, uppercase `QUEUED` with one leading and one trailing space, agent-color
      background, contrast-selected foreground, bold, replacing the timestamp row.
- [ ] The queued boundary is derived from assistant completion (`finish`/`completed_at`), matching
      vanilla's `time.completed` logic.
- [ ] Abort/cancel reaches both the active turn and queued-but-unstarted prompts.
- [ ] Queued prompts and their results are visible to both TUI and CLI through the shared session state.
- [ ] Where the CLI run surface exists, its footer shows the queue count/list and a manage-queued-prompts
      surface matching vanilla (shown only when queued prompts exist, supports removal).
- [ ] Side-by-side parity evidence against the pinned reference commit is recorded.
- [ ] The concurrent-send regression test described in `invariants/message-queuing.md` passes.

## Recommended verification

- Run the Rust TUI against a session, submit several prompts while one turn is running, and compare the
  queued-message display side-by-side with vanilla at the pinned commit: badge text, spacing, colors,
  position (after the in-flight assistant), and timestamp replacement.
- Confirm submit order is preserved and no message is lost or reordered after completion and reload.
- Confirm queued user messages appear immediately with the badge while the current turn streams.
- Where the CLI run surface exists, confirm the footer queue count/list and management match vanilla for
  the same scenario.
- Confirm abort reaches both the active turn and queued-but-unstarted prompts.
- `cargo test -p opencode-server` including the concurrent-send regression test; `cargo test -p
  opencode-tui` for the queued display.

## Blocked Items

- `FEAT-021` Queue CLI task sends while TUI session is open.
- `FEAT-005` Copy Cline-style CLI task send conventions.
- `FEAT-019` Add CLI status visibility for tasks and background sessions.

## Related Items

- `invariants/message-queuing.md` - current-behavior evidence and target invariants for this gate.
- `FEAT-021` Queue CLI task sends while TUI session is open - core queue implementation story.
- `FEAT-005` Copy Cline-style CLI task send conventions - submits prompts into the queue.
- `FEAT-019` Add CLI status visibility for tasks and background sessions - surfaces queued state.
- `FEAT-020` Add default task target selection for CLI sends - routing into a target; already merged,
  not blocked.
- `FEAT-007` Add advanced coding-session polling - adjacent waiting-on-state work, not queueing.
- `FEAT-036` Persist and recall typed input-box messages after send or clear - local draft recall, not a
  queue.
- `BUG-025` Concurrent servers delete each other's sessions and messages via full-snapshot DB sync -
  adjacent concurrency data loss, not prompt queueing.

## Notes

- Created on 2026-09-21; no pre-existing `GATE-*` card existed.
- `bd` does not parse dependency metadata, so the block is expressed in each blocked card's
  `## Blocked By` section plus a `predecessors` frontmatter field.
- Vanilla line references in this card are pinned to `e62912b5d18b73316c7bfd6e894b040698f6c880`; if a
  later board item changes the frozen reference line, re-evaluate this gate.

# Message Queuing Invariants

Status: **Approved / binding** (2026-09-21, via `GATE-001`). The invariants below are binding. The
pre-implementation snapshot, proposed design, and decision list are retained as historical rationale;
the current implemented behavior is summarized here and in `wiki/cli-surface.md`.

This document owns the rules for ordering and serializing prompt/message delivery to a
session. It is the authoritative queuing contract; `invariants/cli-task-targeting.md` keeps
the CLI-specific targeting rules and defers queuing semantics here.

## Scope

- Applies to any message submitted to a session that produces agent work: TUI prompts,
  CLI/`task` sends, direct HTTP/API prompt calls, and any future client.
- Covers server-side per-session acceptance, ordering, execution, status, abort, and
  persistence of queued prompts.
- Does not cover transport attach/detach, server discovery/reuse, or non-prompt operations
  (shell, command execution, fork).

Terminology:

- **Accepted**: the server has durably taken responsibility for a prompt.
- **Queued**: accepted but not yet executing.
- **Active**: the single prompt currently executing for a session.
- **Queue**: the per-session ordered set of accepted-but-not-yet-active prompts.

---

## Current Behavior (implemented by `GATE-001`)

> **Implementation status (2026-09-21, `GATE-001` / PR `feature/GATE-001-session-prompt-queue`):**
> The invariants below are implemented on the server and in the TUI: a shared per-session FIFO queue
> with a single drain loop, accept-time materialization with single ownership, a `Queued` run status,
> abort-active plus explicit queued cancel, `prompt_async` aliased to the queued path, and the vanilla
> `QUEUED` TUI badge. Beyond the invariants, the CLI task client rules stay target until `CLI-001`/
> `CLI-006` land. The one deliberate partial is the CLI run-footer "Manage queued prompts" surface,
> which is conditional on a CLI interactive run surface existing.

### Historical Pre-implementation Baseline

### Endpoints

Registered in `crates/opencode-server/src/routes.rs:143-145`:

- `POST /session/{id}/prompt` -> `session_prompt` (`routes.rs:1710`)
- `POST /session/{id}/prompt/abort` -> `abort_prompt` -> `abort_active_session_prompt` (`routes.rs:2076`, `routes.rs:2083`)
- `POST /session/{id}/prompt_async` -> `prompt_async` (`routes.rs:2280`)

### `session_prompt` (the canonical path)

1. Derives prompt text from `message`, or from `command` + `arguments`; returns
   `BadRequest` if neither is present (`routes.rs:1715-1726`).
2. Verifies the session exists, then resolves provider/model synchronously
   (`routes.rs:1728-1739`).
3. `tokio::spawn`s the actual work and immediately returns
   `{"status":"started","model":...,"variant":...}` (`routes.rs:1747`, `routes.rs:2069-2073`).
4. Inside the spawned task:
   - clones a snapshot of the session (`routes.rs:1748-1754`),
   - sets run status `Busy` (`routes.rs:1755`),
   - resolves agentic context (agent, system prompt, tools, params),
   - builds permission and question callbacks,
   - constructs a fresh `SessionPrompt` and **overwrites**
     `ACTIVE_PROMPTS[session_id]` (`routes.rs:1983-1986`),
   - runs `prompt_with_update_hook`, streaming session snapshots through
     `state.sessions` and broadcasting `session.updated` with source `prompt.stream`.
5. On completion it removes itself from `ACTIVE_PROMPTS` only if the stored `Arc` is still
   pointer-equal (`routes.rs:2043-2050`), updates the session, broadcasts `prompt.final`,
   sets status `Idle`, and persists (`routes.rs:2053-2066`).

### Key structural facts

- `ACTIVE_PROMPTS` is `HashMap<String, Arc<SessionPrompt>>` keyed by session id: a **single
  slot with last-writer-wins**, not a queue (`routes.rs:301-302`).
- There is **no serialization**: two concurrent `/prompt` calls for the same session spawn
  two independent tasks. Each clones the session at start; both stream snapshots into shared
  state and both write the session at the end, so they can interleave, overwrite, and
  reorder history.
- `SESSION_RUN_STATUS` (`routes.rs:298`) has only `Idle`, `Busy`, `Retry`; there is no
  `queued` state. `Busy` is set when execution begins, not when a request is accepted.
- `GET /session/status` merges session lifecycle status with run status and exposes
  `status`, `idle`, `busy`, `attempt`, `message`, `next` (`routes.rs:331-369`).

### `prompt_async` (misleading stub)

`prompt_async` (`routes.rs:2280`) appends a user message and an empty assistant message to
the session, persists, and returns `{"status":"queued","message_id":...,"model":...}`
(`routes.rs:2296-2313`). It never creates a `SessionPrompt`, never calls a provider, and
never executes. The result is a persisted unanswered assistant turn. This is the only code
that claims to "queue" a prompt, and it does not queue or execute anything.

### Abort

`abort_active_session_prompt` (`routes.rs:2083`) looks up `ACTIVE_PROMPTS[session_id]`,
cancels that runner, and sets `Idle`. Only the currently tracked runner can be cancelled;
orphaned concurrent runners and accepted-but-unstarted prompts cannot be found or cancelled
because they are not represented anywhere.

### Clients

- TUI submits through `ApiClient::send_prompt` -> `POST /session/{id}/prompt`
  (`crates/opencode-tui/src/api.rs:533`).
- TUI `submit_prompt` (`crates/opencode-tui/src/app/app.rs:2297`) has **no busy guard**; a
  user can press Enter while a prompt is running and fire a second `/prompt`.
- The `opencode task` CLI surface is not implemented yet; `CLI-007` only added target
  selection. The generated route docs list `sessionPromptAsync` (`crates/opencode-cli/src/main.rs:3322`),
  but no client uses it as a queue.
- The TUI prompt stash (`crates/opencode-tui/src/components/prompt.rs:659`) and the server's
  `TUI_REQUEST_QUEUE` (`routes.rs:4260`) are unrelated to prompt queuing (draft storage and
  server-to-TUI control injection respectively).

### Observed gaps

1. Concurrent prompts to one session race and can corrupt or reorder history.
2. `prompt_async` reports `queued` but never runs, leaving a dangling assistant turn.
3. Status cannot express "queued" or queue depth.
4. Abort cannot reach accepted-but-unstarted work.
5. No client can tell whether its send started or is waiting.
6. Restart loses any in-flight/queue intent because none is durably represented.

---

## Binding Invariants

Approved 2026-09-21 via `GATE-001`. Implementation status: 1-9 and 11 are implemented; 10 is partially
implemented (in-memory queue plus transcript persistence, no resume); 12 remains target for the CLI
client pending `CLI-001`/`CLI-006`; 13 holds (no lifecycle changes).

1. **One active run per session.** A session executes at most one agentic prompt at a time.
   Concurrency across different sessions is allowed and independent.

2. **Every accepted prompt is ordered FIFO per session.** Ordering is by server acceptance
   order, not client wall-clock send order. There is no cross-session ordering guarantee.

3. **Acceptance is explicit and lossless.** A prompt is either accepted (started or queued)
   or rejected with a clear error. Acceptance must never silently drop, replace, or
   overwrite another accepted prompt.

4. **One canonical path.** TUI, CLI, and API all deliver prompts through the same canonical
   server/session prompt path and the same `SessionPrompt` runtime. No parallel executor and
   no endpoint may fake queueing by appending messages without execution.

5. **Queue-aware acceptance response.** Every prompt endpoint returns whether the request is
   `started` or `queued`, with a stable request/message identifier and, when queued, its
   position or queue depth. `prompt_async` must become a real queued execution path or be
   removed/aliased; it must never remain a message-append stub.

6. **Status distinguishes queued from busy.** Run status exposes at least `idle`, `busy`, and
   `queued` (with queue depth/position). Transitions are broadcast as `session.status`
   events and readable from `GET /session/status`.

7. **Shared observable state.** Queued prompts and their results are visible to every client
   through normal session state (`session.updated` / transcript). A queued user message
   appears immediately and is marked pending until it becomes active. TUI and CLI observe
   the same ordering and results.

8. **Abort and cancel are complete.** The runtime can cancel the active run and can
   explicitly remove queued prompts. Cancelled items are represented as cancelled turns and
   are never left as unresolved pending work. Abort must not be limited to whichever runner
   happens to be tracked.

9. **Failure isolation.** A provider/tool error in one prompt fails only that prompt and
   records an explicit error turn; it does not corrupt ordering, and the next queued prompt
   proceeds.

10. **Durable ordering intent.** Accepted prompt intent is persisted such that restart cannot
    silently reorder or lose it. On startup, queued-but-unstarted work is either resumed or
    explicitly marked cancelled/unstarted; silent loss is forbidden. The exact resume policy
    is decided below.

11. **Bounded queues with backpressure.** Each session has a defined queue bound. Overflow is
    rejected explicitly (never dropped silently) and does not affect already-accepted work.

12. **Client honesty.** A client that cannot stream queued work must report queued status
    rather than implying execution. `--stream` semantics for a queued request are defined
    explicitly (follow on start, or report unavailable-while-queued).

13. **No lifecycle side effects.** Queuing must not change `ort` TUI launch/attach/detach or
    same-workspace server reuse behavior; those remain owned by their own board items.

---

## Implemented Design Shape (`GATE-001`)

This is the shape implemented by `GATE-001`; exact types are implementation detail.

- **Per-session queue state.** Replace the last-writer-wins `ACTIVE_PROMPTS` slot with:
  - an active-runner handle (single slot, guarded), and
  - a per-session `VecDeque<PendingPrompt>` (ordered), behind a per-session lock or owned by
    a dedicated per-session drain task.
- **Enqueue path.** The HTTP handler validates input, persists the user message with a
  `queued` marker, pushes a `PendingPrompt`, and returns `started` (if it took the active
  slot) or `queued` with position. A drain loop then executes prompts strictly in FIFO order,
  keeping `busy` while the queue is non-empty and transitioning to `idle` only when drained.
- **Active vs queued status.** `SESSION_RUN_STATUS` gains a `queued { position, depth }`
  variant (or equivalent), and `session_status` surfaces it.
- **Callbacks.** Permission/question callbacks are created per active prompt as today; queued
  prompts must not register callbacks until they become active.
- **Abort contract.** Define two operations: cancel-active (cancel current runner) and
  clear-queued (drop pending items as cancelled). Decide whether `abort` does one or both
  (see open decisions). Both must be able to address every accepted item.
- **Persistence.** Persist queued user messages (and a queued marker) via the existing
  session store. Do not add a separate queue store.
- **Bound.** A configurable per-session maximum (proposed default 32) rejects new sends with
  a clear "queue full" error once reached.

## Client Behavior (target)

The TUI behavior below is implemented; the CLI/API items remain target until `CLI-001`/`CLI-006`.

- **TUI.** May keep its optimistic user message; a queued message renders as pending and is
  reconciled when `session.updated`/status shows it active or complete. The TUI may also
  queue locally before the server ack, but the server queue is authoritative.
- **CLI.** `task send`/`task new` return target session plus `started`/`queued` and queue
  position. `task view`/status show queued entries. `--stream` follows the request when it
  starts; while queued it reports queued status instead of pretending to stream.
- **API.** `/session/{id}/prompt` is queue-aware. `prompt_async` is either removed, aliased
  to `/prompt` with fire-and-forget semantics, or made the explicit queued endpoint; it must
  execute for real.

---

## Decisions (resolved by `GATE-001`)

1. **Endpoint shape:** `/prompt` is queue-aware and returns `started` or `queued`; the stub is deprecated.
2. **`prompt_async` fate:** aliased to the queue-aware path; it never reports `queued` without enqueuing execution.
3. **Abort semantics:** abort cancels the active run only; a separate `POST /session/{id}/prompt/cancel` removes one waiting prompt.
4. **Cancelled-turn representation:** an explicit cancel removes the queued prompt and its materialized user message; no cancelled turn is retained.
5. **Restart policy:** the queue is in-memory; accepted-but-unstarted messages persist in the transcript and are never auto-executed or silently deleted. No resume in V1.
6. **Queue bound and overflow:** per-session default 32 with an explicit `queue is full` rejection.
7. **TUI queued-message rendering:** pending `QUEUED` badge plus explicit cancel; no reorder in V1.
8. **`--stream` while queued:** return queued immediately (session, message id, position); `--stream` follows once active. Target for `CLI-001`.
9. **Status vocabulary:** `idle | busy | queued` with position/depth; `retry` retained.

---

## Relationship To Other Artifacts

- **Board items (queuing-related):**
  - `GATE-001` Gate: prompt queuing and queue display must match vanilla OpenCode exactly
    (primary and sole owner; `done` - implemented the per-session queue and the vanilla `QUEUED` badge).
  - `CLI-007` Add default task target selection for CLI sends (routing into the queue; merged in PR #37,
    in `qa`; the target input consumed by `CLI-001`).
  - `CLI-001` Copy Cline-style CLI task send conventions (reactivated 2026-09-22; canonical CLI task send
    surface and prerequisite gate for the remaining `CLI-*` stories).
  - `CLI-006` Add CLI status visibility for tasks and background sessions (reactivated 2026-09-22; surfaces
    queued state; co-prerequisite gate).
  - `CLI-008` Queue CLI task sends while TUI session is open (archived 2026-09-21; queue delivered by `GATE-001`).
  - `FEAT-007` Add advanced coding-session polling (waiting on session state; related but distinct)
  - `FEAT-036` Persist and recall typed input-box messages (hold; local drafts, not a queue)
- **Handoffs:** `handoffs/archive/2026-09-21-session-prompt-queue-gate-handoff.md` (`H-004`, closed and
  archived 2026-09-22; satisfied by `GATE-001`; the reactivated `CLI-001`/`CLI-006` are now owned by `H-006`,
  `handoffs/archive/2026-09-22-cli-task-surface-and-status-handoff.md`). `handoffs/archive/2026-09-16-cli-task-targeting-handoff.md`
  (`H-003`) is superseded and folded into `H-004` (2026-09-22).
- **Canonical behavior doc:** `wiki/cli-surface.md` (current + target CLI task and TUI lifecycle behavior).
- **Existing invariants:** `invariants/cli-task-targeting.md` (targeting rules; its enqueue
  rule references this doc), `invariants/coding-session-behavior.md` (canonical session path),
  `invariants/runtime-lifecycle.md` (no lifecycle changes).
- **Index:** listed in `invariants/README.md`.

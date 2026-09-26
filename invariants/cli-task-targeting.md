# CLI Task Targeting Invariants

Canonical behavior reference: `wiki/cli-surface.md`.

Status: **binding**. Target selection (`CLI-007`) and the `opencode task new|send|view|status` surface
(`CLI-001`/`CLI-006`) are implemented and QA-verified as of 2026-09-23. Rules that depend on an
unresolved decision are marked **target** and must remain true if and when that work lands.

## Scope

- Covers the `opencode task` CLI surface: target `list|select|show|clear`, `new`, `send`, `view`, and
  `status`.
- Covers how CLI task sends interact with the shared per-session prompt queue.
- Does not cover the `opencode run` command, which still uses the interim `AgentExecutor` loop
  (`CLI-002`, `hold`), nor TUI launch/attach/detach lifecycle (see `invariants/runtime-lifecycle.md`).
- Sources inspected:
  - `crates/opencode-cli/src/main.rs` (`TaskCommands` at `:293`, `send_task_prompt` at `:2942`,
    `handle_task_new` at `:3131`, `handle_task_send` at `:3163`, `handle_task_view` at `:3188`,
    `handle_task_status` at `:3208`)
  - `crates/opencode-server/src/routes.rs` (`accept_prompt` at `:2478`, `SESSION_QUEUE_LIMIT = 32`
    at `:394`, `POST /{id}/prompt/cancel` at `:148`)
  - Board items `CLI-001`/`CLI-006`/`CLI-007` (`done`), `CLI-002`/`CLI-005`/`CLI-009` (`hold`)
  - `wiki/cli-surface.md`

## Invariants

- CLI task commands must send work only to an explicit target: a server/session provided on the command line or a user-selected default task target.
- Selecting a default task target is a user-directed action that may be changed or cleared at any time.
- The selected default task target is scoped to CLI task sends and views; it must not cause normal `ort` TUI launches to auto-attach to or reuse a server.
- A selected default target must identify the server URL and, when applicable, the default session ID used by follow-up sends and views.
- Target selection must present active, observable servers/sessions for user choice when an interactive selector is used; stale records may be shown only when clearly marked unavailable and must not be selected silently.
- Explicit command-line server/session options must override the selected default target.
- A stale, unreachable, or invalid selected target must fail visibly and must not silently fall back to a different server or session.
- Task sends and views must use the canonical server/session runtime path, not a parallel prompt executor.
- Creating a task from the CLI may update the selected current/default session pointer only after the server acknowledges the created session.
- Viewing a task/session from the CLI must not mutate the selected target or session state.
- Sending a CLI task prompt to a session that is currently open in the TUI must enqueue the request for that session instead of racing, interleaving, replacing, or rejecting it solely because the TUI is open.
- Queued CLI task prompts must preserve submit order per target session.
- The TUI and CLI must observe the same queued prompt results through the shared session state.
- CLI task behavior must remain separate from normal `ort` TUI launch lifecycle, attach, detach, and same-workspace reuse decisions unless those behaviors are changed by their own explicit board items.

## Implementation Evidence

- `task new`/`send` POST to `/session/{id}/prompt` (`send_task_prompt`, `main.rs:2942-2957`) and print the
  server's `started`/`queued` status with queue position/depth (`main.rs:2959-2975`).
- `task new` persists the selected session only after the create and prompt calls succeed
  (`main.rs:3146-3154`); `task send` requires a resolved session and never writes target state.
- `task view` reads `/session/{id}/message` and never writes (`main.rs:3188-3206`).
- `task status` reads `/session?roots=true&limit=100` plus `/session/status` and renders
  `idle|busy|queued` with queue position/depth (`main.rs:3208-3265`, `print_task_statuses` at `:3056`).
- Explicit `--server`/`--session` override the stored target via `selected_task_target`
  (`main.rs:3176`, `:3196`, `:3217`); an unreachable target fails via `ensure_task_target_available`
  (`main.rs:2897`).
- Queueing on an open TUI session and FIFO ordering are owned by the shared server queue
  (`invariants/message-queuing.md`; `accept_prompt`, `routes.rs:2478`).

## Known Gaps / Drift

- `opencode run` does not yet use the canonical session runtime (`CLI-002`, `hold`), so the
  "canonical path" invariant currently holds for `opencode task`, not for `opencode run`.
- Same-workspace server attach/reuse remains an open human decision (`CLI-005`, `hold`); no reuse
  behavior may be introduced until that card resolves.
- CLI question/approval parity (`CLI-009`) and subagent surface parity (`CLI-010`) are separate
  stories; `CLI-010` is in `qa`, `CLI-009` is on `hold`.

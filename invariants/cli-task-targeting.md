# CLI Task Targeting Invariants

Canonical behavior reference: `wiki/cli-surface.md`.

These are target invariants for the final system. The current CLI surface only implements
`opencode task target list|select|show|clear` (`CLI-007`); `task new`/`send`/`view` and CLI status are
target (`CLI-001`/`CLI-006`).

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

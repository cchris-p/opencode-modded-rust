# CLI Task Targeting Invariants

- CLI task commands must send work only to an explicit target: a server/session provided on the command line or a user-selected default task target.
- Selecting a default task target is a user-directed action that may be changed or cleared at any time.
- The selected default task target is scoped to CLI task sends and views; it must not cause normal `ort` TUI launches to auto-attach to or reuse a server.
- A selected default target must identify the server URL and, when applicable, the default session ID used by follow-up sends and views.
- Explicit command-line server/session options must override the selected default target.
- A stale, unreachable, or invalid selected target must fail visibly and must not silently fall back to a different server or session.
- Task sends and views must use the canonical server/session runtime path, not a parallel prompt executor.
- Sending a CLI task prompt to a session that is currently open in the TUI must enqueue the request for that session instead of racing, interleaving, replacing, or rejecting it solely because the TUI is open.
- Queued CLI task prompts must preserve submit order per target session.
- The TUI and CLI must observe the same queued prompt results through the shared session state.

# Session Durability Invariants

- Durable session and message state is owned by the persistent store, not by any individual server process.
- A server process must never delete or overwrite durable session or message state that it did not explicitly create, modify, or delete.
- Saving a server's in-memory view must be additive: it must not remove stored sessions or messages that are absent from that view.
- Deleting a session is explicit and user-initiated; only the explicit delete path removes durable sessions and their messages.
- A server must not overwrite a durable session whose stored update is newer than the server's in-memory view.
- Replacing a session's message history must be atomic, so an interrupted save cannot leave a partially written session.
- Persistent session and task state must outlive any individual server process, server restart, model turn, or TUI exit.
- Resuming a session must reconstruct it from durable state and must fail visibly when the requested session does not exist, rather than presenting an empty session.

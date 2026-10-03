# opencode-server

`opencode-server` provides HTTP/SSE/WebSocket APIs and bridges CLI, TUI, and external systems.

## Responsibilities

- Expose unified API routes
- Manage sessions, config, providers, MCP, permissions, files, and search
- Provide event stream and TUI control endpoints
- Handle OAuth callbacks, PTY, and workspace operations

## Route groups (selection)

As in `crates/opencode-server/src/routes.rs`:

- Base: `/health`, `/event`, `/path`, `/vcs`
- Session: `/session/*`
- Provider: `/provider/*`
- Config: `/config/*`
- MCP: `/mcp/*`
- File: `/file/*`
- Search: `/find/*`
- Permission: `/permission/*`
- Project: `/project/*`
- PTY: `/pty/*`
- TUI control: `/tui/*`
- Experimental: `/experimental/*`
- Plugin auth: `/plugin/*`

## Module structure

- `server.rs` – Server startup and lifecycle
- `routes.rs` – Route definitions and handlers
- `oauth.rs` / `mcp_oauth.rs` – OAuth flow
- `pty.rs` – Terminal session bridge
- `worktree.rs` – Workspace operations

## External Supervision Contract

Watchdog and other external supervisors should start ORT with `opencode serve`, not by depending on TUI-only launcher state.

- Launch command: `opencode serve --cwd <workspace> --hostname 127.0.0.1 --port <port> --startup-json`
- Startup discovery: parse the single startup JSON line emitted by the CLI for `pid`, `url`, `health_url`, and `workspace`.
- Readiness: poll `health_url` (`GET /health`) until it succeeds before exposing the server as usable.
- Stop: terminate the supervised child process or process group. ORT does not provide a separate remote shutdown route for this contract.
- Reuse: this launch path starts the requested local server process and does not silently attach to an unrelated stale TUI server.

## Development notes

- Define input/output models before adding routes and handlers
- Avoid blocking (I/O, DB, network) on high-concurrency paths
- Keep CLI/TUI call sites in sync when changing APIs

## Validation

```bash
cargo check -p opencode-server
```

# opencode-cli

`opencode-cli` provides the workspace’s unified executable entrypoint (binary name: `opencode`).

## Command scope

- Start TUI
- Start or attach to server
- Run single tasks (`run`)
- Start, continue, inspect, and list status for task sessions (`task new|send|view|status`)
- Manage explicit CLI task targets (`task target`)
- Invoke session, model, MCP, debug, and other subcommands

## Top-level subcommands

As of `opencode --help` (2026-02-23):

- `tui`
- `attach`
- `run`
- `task`
- `serve`
- `web`
- `acp`
- `models`
- `session`
- `stats`
- `db`
- `config`
- `auth`
- `agent`
- `debug`
- `mcp`
- `export`
- `import`
- `github`
- `pr`
- `upgrade`
- `uninstall`
- `generate`
- `version`

## Common options

### `opencode tui`

- `-m, --model <MODEL>`
- `-c, --continue`
- `-s, --session <SESSION>`
- `--fork`
- `--agent <AGENT>` (default: `build`)
- `--port <PORT>`, `--hostname <HOSTNAME>`

### `opencode run`

- `MESSAGE...`
- `--command <COMMAND>`
- `-f, --file <FILE>`
- `--format <default|json>`
- `--thinking`
- `--agent <AGENT>` / `--model <MODEL>`

### `opencode serve`

`opencode serve` is the supported local HTTP server entrypoint for an external supervisor such as Watchdog.

- `--cwd <PATH>` selects the workspace/root before server startup.
- `--hostname <HOSTNAME>` selects the bind host. The default is `127.0.0.1`.
- `--port <PORT>` selects the bind port. The default CLI value resolves to `3000` for compatibility with existing local usage.
- `--startup-json` prints one machine-readable JSON line before the server begins accepting requests.

With `--startup-json`, supervisors should parse a line shaped like:

```json
{"event":"opencode.server.starting","mode":"serve","pid":12345,"bind_host":"127.0.0.1","port":3000,"url":"http://127.0.0.1:3000","health_url":"http://127.0.0.1:3000/health","workspace":"/repo/path"}
```

Readiness is established by polling `health_url` until the endpoint returns success. Stopping a supervised server is process supervision: terminate the process group or child process that produced the startup JSON line. ORT does not reuse or attach to unrelated stale servers for this path.

### `opencode task new|send|view|status`

Task commands talk to an explicit server/session target. They do not discover, start, reuse, or attach to a server implicitly, and they do not change normal `opencode`/`ort` TUI launch behavior.

- `task new [--server <URL>] [--stream] <PROMPT...>` creates a new session on the selected or provided server, submits the prompt through `POST /session/{id}/prompt`, then stores that acknowledged session as the workspace-local default task session.
- `task new [--server <URL>] < prompt.md` reads prompt text from stdin when available.
- `task send [--server <URL>] [--session <SESSION_ID>] [--stream] <PROMPT...>` submits a follow-up prompt to the explicit session override or selected default session through the canonical session prompt path.
- `task view [--server <URL>] [--session <SESSION_ID>] [--json]` prints the selected session transcript from the target server without opening the TUI.
- `task status [--server <URL>] [--session <SESSION_ID>] [--json]` reads `GET /session/status` from the selected or provided server, lists server sessions by default, and narrows to one session when `--session` is provided.
- `--server` and `--session` override the stored target for the current command only.
- A stale or unreachable selected target fails visibly instead of falling back to a different server or session.

### `opencode task target`

`task target` manages the explicit default server/session pointer used by future CLI task commands. It does not change normal `opencode`/`ort` TUI launch behavior and does not implicitly discover or reuse servers.

- `list --server <URL>` live-checks explicitly provided server candidates and prints their root sessions when reachable.
- `select --server <URL> [--session <SESSION_ID>]` verifies the server, verifies the session when provided, and stores the workspace-local default target in `.opencode/task-target.json`.
- `show` prints the stored target and live-checks whether the server/session is still available.
- `clear` removes only the stored task target pointer.

Explicit task command options such as `--server` and `--session` override this selected target for `task new`, `task send`, `task view`, and `task status`.

## Source entrypoint

- `crates/opencode-cli/src/main.rs`

## Development notes

- After changing subcommand behaviour, update `--help` and then the docs
- Prefer consistent naming between CLI args and server/config fields

## Validation

```bash
cargo check -p opencode-cli
./target/debug/opencode --help
./target/debug/opencode task --help
./target/debug/opencode task target --help
```

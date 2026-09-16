# opencode-cli

`opencode-cli` provides the workspace’s unified executable entrypoint (binary name: `opencode`).

## Command scope

- Start TUI
- Start or attach to server
- Run single tasks (`run`)
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

### `opencode task target`

`task target` manages the explicit default server/session pointer used by future CLI task commands. It does not change normal `opencode`/`ort` TUI launch behavior and does not implicitly discover or reuse servers.

- `list --server <URL>` live-checks explicitly provided server candidates and prints their root sessions when reachable.
- `select --server <URL> [--session <SESSION_ID>]` verifies the server, verifies the session when provided, and stores the workspace-local default target in `.opencode/task-target.json`.
- `show` prints the stored target and live-checks whether the server/session is still available.
- `clear` removes only the stored task target pointer.

Explicit task command options such as `--server` and `--session` are reserved to override this selected target as later task send/view commands land.

## Source entrypoint

- `crates/opencode-cli/src/main.rs`

## Development notes

- After changing subcommand behaviour, update `--help` and then the docs
- Prefer consistent naming between CLI args and server/config fields

## Validation

```bash
cargo check -p opencode-cli
./target/debug/opencode --help
./target/debug/opencode task target --help
```

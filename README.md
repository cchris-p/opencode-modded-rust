# scopemux-code

**A Rust-native local coding agent product informed by OpenCode**

`scopemux-code` is the main Rust product. It is informed by OpenCode as a reference line, but it is not a long-term TypeScript customization effort. The current repository contains a broad existing implementation surface, while the active product direction is documented in `wiki/` and `invariants/`.

## Current status

- Product name: `scopemux-code`
- Repository shorthand: `opencode-modded-rust` (legacy repository identifier)
- Executable: `opencode` (legacy compatibility command name)

## Feature overview

- **Interaction modes:** TUI (default), CLI single run, HTTP server, Web/ACP mode
- **Sessions:** Create, continue, fork sessions; import/export
- **Tool system:** Built-in read/write/edit, shell, patch, and related tools
- **Model support:** Multiple providers, agent mode switching
- **Extensibility:** Plugin bridge (including TS plugins), MCP, LSP
- **Terminal:** Improved layout, collapsible sidebar, syntax highlighting, path completion

## Quick start

### 1. Requirements

- Rust stable
- Cargo
- Git (recommended)

### 2. Build

```bash
cargo build -p opencode-cli
```

### 3. Use this project's binary

To avoid running another OpenCode (e.g. npm/global) on your PATH when using `scopemux-code`, run from the **repo root**:

- **`./target/debug/opencode`** after `cargo build -p opencode-cli`
- **`cargo run -p opencode-cli --`** to always use this repo's version

### 4. Show help

```bash
./target/debug/opencode --help
```

or

```bash
cargo run -p opencode-cli -- --help
```

### 5. How to run

- Default: start TUI:

```bash
cargo run -p opencode-cli --
```

- Start TUI explicitly:

```bash
cargo run -p opencode-cli -- tui
```

- Single non-interactive run:

```bash
cargo run -p opencode-cli -- run "Check this repo for risks"
```

- Start HTTP server:

```bash
cargo run -p opencode-cli -- serve --port 3000 --hostname 127.0.0.1
```

## CLI commands overview

From the repo root use `./target/debug/opencode` or `cargo run -p opencode-cli --`. These commands match the current `./target/debug/opencode --help`:

- `tui` – Start interactive terminal UI
- `attach` – Attach to a running server
- `run` – Run a single message
- `task new|send|view|status` – Start, continue, inspect, or list status for task sessions on an explicit server/session target
- `task target` – Select, show, list, or clear the explicit default target for CLI task commands
- `serve` – Start HTTP server
- `web` – Start headless server and open web UI
- `acp` – Start ACP server
- `models` – List available models
- `session` – Session management
- `stats` – Token/cost statistics
- `db` – Database tools
- `config` – Show configuration
- `auth` – Credential management
- `agent` – Agent management
- `debug` – Debugging and troubleshooting
- `mcp` – MCP management
- `export` / `import` – Export/import sessions
- `github` / `pr` – GitHub-related features
- `upgrade` / `uninstall` – Upgrade and uninstall
- `generate` – Generate OpenAPI spec
- `version` – Show version

Subcommand help:

```bash
./target/debug/opencode tui --help
./target/debug/opencode run --help
./target/debug/opencode task --help
./target/debug/opencode task target --help
./target/debug/opencode serve --help
./target/debug/opencode session --help
```

## Configuration

Configuration is merged from the following paths in priority order (searched upward):

- `opencode.jsonc`
- `opencode.json`
- `.opencode/opencode.jsonc`
- `.opencode/opencode.json`

Global config default path:

- Linux/macOS: `~/.config/opencode/opencode.jsonc` (or `.json`)

See: `docs/opencode-config.md`

## Repository structure

- `crates/opencode-cli` – CLI entrypoint (binary: `opencode`)
- `crates/opencode-server` – HTTP/SSE/WebSocket server
- `crates/opencode-tui` – Terminal UI
- `crates/opencode-session` – Sessions and messages
- `crates/opencode-tool` – Tool registration and execution
- `crates/opencode-provider` – Model provider adapters
- `crates/opencode-plugin` – Plugin system and subprocess bridge
- `crates/opencode-mcp` – MCP client and registration
- `crates/opencode-lsp` – LSP support
- `crates/opencode-storage` – SQLite storage

## Development and validation

```bash
cargo fmt
cargo check
cargo clippy --workspace --all-targets
```

Minimal check (typical):

```bash
cargo check -p opencode-cli
cargo check -p opencode-tui
```

### Git hooks

This repo ships a version-controlled `pre-commit` hook that runs `cargo fmt --all`
when a commit stages Rust files, then re-stages the formatted result. It no-ops
when `cargo` is unavailable or when no Rust files are staged. Install it once
per clone:

```bash
./scripts/install-git-hooks.sh
```

Bypass a single commit with `git commit --no-verify`, or skip the hook for one
commit with `OPENCODE_SKIP_FMT_HOOK=1`. Disable it entirely with
`git config --unset core.hooksPath`. Files with unstaged edits are intentionally
not re-staged; the hook warns instead so unstaged work is never captured.

### TUI stall tracing

The TUI can log runtime timings to a file for diagnosing stalls and freezes
(for example the thinking-mode freeze tracked as `BUG-027`). Set
`OPENCODE_TUI_TRACE` to a file path before launching:

```bash
ort-build
OPENCODE_TUI_TRACE=/tmp/ort-thinking-on.log ort
```

While that is set, a background thread writes one `SAMPLE` line per second with
loop iterations, `session.updated` deliveries, syncs and cumulative sync time,
draws and cumulative draw time, key events, and starvation gaps. Each
full-session refetch also logs a `SYNC` line that splits `get_session` vs
`get_messages` duration, and any loop gap of 50ms or more logs `STARVATION`.
The sampler runs on its own thread, so it keeps recording while the main event
loop is blocked. Tracing is disabled unless `OPENCODE_TUI_TRACE` is set.

To separate rendering cost from the network refetch path, capture a second run
with `/thinking` off and compare:

```bash
OPENCODE_TUI_TRACE=/tmp/ort-thinking-off.log ort
```

During a stall, `draws=0` and `keys=0` with `sync_ms` close to the sample
`dt` point at the blocking refetch path; a large `draw_ms` with small `sync_ms`
points at rendering.

## Documentation

- User guide: `USER_GUIDE.md`
- Docs index: `docs/README.md`
- Planning wiki: `wiki/README.md`
- Invariants index: `invariants/README.md`
- CLI: `docs/opencode-cli.md`
- TUI: `docs/opencode-tui.md`
- Server: `docs/opencode-server.md`
- Tools: `docs/opencode-tool.md`
- Provider: `docs/opencode-provider.md`
- Config: `docs/opencode-config.md`

## Notes

- `scopemux-code` is the canonical product name for planning and product-facing documentation.
- The executable name `opencode` remains for backward compatibility.
- `opencode-*` crate names and `opencode-modded-rust` are retained as implementation and repository identifiers, not the product name.
- `opencode-modded` remains the long-term reference and planning repo.
- The TypeScript reference line for planning and later sync review is `$HOME/repos/opencode-modded`, referenced from its `dev` branch; fetch the latest `dev` before comparing vanilla behavior. The recorded pin is `f54ce313b99a6661d7758ad042f7a6e05c8e0972` (re-pinned by `GATE-004`; the prior `e62912b5` pin is unreachable).

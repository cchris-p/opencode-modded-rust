# opencode-modded-rust

**A Rust-native local coding agent product informed by OpenCode**

`opencode-modded-rust` is the main Rust product. It is informed by OpenCode as a reference line, but it is not a long-term TypeScript customization effort. The current repository contains a broad existing implementation surface, while the active product direction is documented in `wiki/` and `invariants/`.

## Current status

- Product name: `opencode-modded-rust`
- Repository: `opencode-modded-rust`
- Executable: `opencode-modded-rust`

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

To avoid running another OpenCode (e.g. npm/global) on your PATH when using `opencode-modded-rust`, run from the **repo root**:

- **`./target/debug/opencode-modded-rust`** after `cargo build -p opencode-cli`
- **`cargo run -p opencode-cli --`** to always use this repo's version

### 4. Show help

```bash
./target/debug/opencode-modded-rust --help
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

From the repo root use `./target/debug/opencode-modded-rust` or `cargo run -p opencode-cli --`. These commands match the current `./target/debug/opencode-modded-rust --help`:

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
- `upgrade` / `uninstall` – Upgrade and uninstall (`upgrade --check` reports an available
  artifact revision without installing)
- `generate` – Generate OpenAPI spec
- `version` – Show version

Subcommand help:

```bash
./target/debug/opencode-modded-rust tui --help
./target/debug/opencode-modded-rust run --help
./target/debug/opencode-modded-rust task --help
./target/debug/opencode-modded-rust task target --help
./target/debug/opencode-modded-rust serve --help
./target/debug/opencode-modded-rust session --help
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

- `crates/opencode-cli` – CLI entrypoint (binary: `opencode-modded-rust`)
- `crates/opencode-server` – HTTP/SSE/WebSocket server
- `crates/opencode-tui` – Terminal UI
- `crates/opencode-session` – Sessions and messages
- `crates/opencode-tool` – Tool registration and execution
- `crates/opencode-provider` – Model provider adapters
- `crates/opencode-plugin` – Plugin system and subprocess bridge
- `crates/opencode-mcp` – MCP client and registration
- `crates/opencode-lsp` – LSP support
- `crates/opencode-storage` – SQLite storage

## Native provider artifacts

The default binary compiles the native ScopeMux provider. `INFRA-006` builds it
once in CI (`.github/workflows/native-artifacts.yml`) and publishes platform
bundles to the `native-artifacts` release, so machines download a prebuilt bundle
instead of cloning `scopemux-core` and compiling the C provider (with its
Tree-sitter/pybind11 submodules) per machine.

- Install for a build: `scripts/ensure-native-artifacts.sh` downloads the matching
  bundle (by OS/arch + pinned revision) into
  `~/.cache/opencode-modded-rust/native/` and prints its path. `ort-build` calls
  it and exports `SCOPEMUX_PREBUILT_DIR`; set `OPENCODE_RUST_PREBUILT_NATIVE=0` to
  skip it.
- Package a bundle (local or CI): `scripts/package-native-artifacts.sh`.
- `build.rs` links the bundle from `SCOPEMUX_PREBUILT_DIR` when set (no CMake,
  Python, or Tree-sitter/pybind11 submodules), and otherwise falls back to the
  source build in `third_party/scopemux-core` (provisioned by
  `scripts/fetch-scopemux-core.sh`). Set `SCOPEMUX_CORE_DIR` to force a specific
  source checkout.

**CI access.** `scopemux-core` and the grammar/pybind11 submodules the provider
build needs are public, so CI clones them over anonymous HTTPS (the workflow sets
`SCOPEMUX_CORE_REPO` to the HTTPS URL; the script's default is SSH). The private
`scopemux-test-cases` submodules are intentionally not initialized. If
`scopemux-core` is ever made private, add token auth to the workflow.

## CLI binary artifacts

`INFRA-007` makes the CLI a downloadable artifact, so most machines run `ort`
without cloning this repo or installing a Rust toolchain.

- Publish from a builder: `scripts/publish-cli-artifact.sh` builds
  `target/release/opencode-modded-rust`, packages
  `opencode-modded-rust-<os>-<arch>.tar.gz` (+ `.sha256` and `.rev`), and uploads
  it to the rolling `native-artifacts` release. `--dry-run` packages only.
- CI also publishes on `development` pushes. The Linux asset is built on
  `ubuntu-22.04` (glibc 2.35, the oldest fleet host); a `ubuntu-24.04` build
  requires GLIBC 2.39 and fails on 22.04 machines.
- Consumers install with `~/standards/utilities/ort-install` (no checkout
  required). It verifies the checksum and installs to
  `~/.local/share/ort/bin/<rev>/`, with `current` pointing at the newest
  revision. `ort` auto-installs when no binary is present; use
  `OPENCODE_RUST_USE_CHECKOUT=1` on a builder to prefer a local
  `target/debug`/`target/release` binary.

### Release channels and fleet deployment

- **stable** (default): the latest immutable `main` release (`v<version>`).
  `ort-install` and `opencode-config` follow it by default.
- **preview** (opt-in): the rolling `native-artifacts` tag refreshed on
  `development` pushes. Select with `ort-install --channel preview` or
  `OPENCODE_RUST_CHANNEL=preview`. Preview builds never auto-deploy to the
  fleet.
- Only `main` releases deploy, and a release is complete only after every
  active registered machine is installed and verified; any failure rolls back
  the affected machines. Inactive machines are reported and skipped.
- Policy: `invariants/release-deployment.md`. Runbook:
  `docs/release-deployment.md`.

### Update notices and artifact updates

- An artifact-installed binary compares its installed revision with the
  revision published for the active channel at startup and then on a throttled
  interval. The running revision is read from the
  `~/.local/share/ort/bin/<rev>/opencode-modded-rust` layout, so a
  `cargo run`/`target/debug` build is treated as a development build and never
  shows a notice.
- When a different published revision exists, the prompt status line shows
  `Update available: <version> (<short rev>) - press ctrl+g`, and the same
  notice is shown once as a toast. It never steals focus or blocks a turn.
- `ctrl+g`, typing a bare `/update` (or `/upgrade`) in the prompt and pressing Enter, or
  `/update` in the command palette opens the Update dialog. It
  shows current vs available version/rev/channel; `Enter` installs, `r`
  re-checks, `d` dismisses the notice, and `Esc` closes. `upgrade --check`
  prints the same state from the CLI.
- Installation is delegated to `~/standards/utilities/ort-install` (checksum
  verified, channel-aware). The running process is never hot-swapped; restart
  the session to apply the new revision. A successful install suppresses the
  notice for the rest of that session.
- Checks use a short timeout, cache their result in
  `${XDG_CACHE_HOME:-~/.cache}/opencode/update-check.json`, and are silent on
  offline, rate-limited, or unknown responses. They never block startup or a
  model request.
- `OPENCODE_RUST_UPDATE_CHECK=0` disables notices and checks for this feature
  (use it to opt out); `OPENCODE_RUST_UPDATE_INTERVAL=<seconds>` sets the
  periodic interval and `0` disables periodic re-checks after the startup
  check. `OPENCODE_RUST_CHANNEL=stable|preview` chooses the channel and is
  unchanged by this feature. The launcher's silent auto-update remains a
  separate `OPENCODE_RUST_AUTO_UPDATE` control before the product starts.

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

### Runtime bounds and server logs

Runtime bounds match opencode vanilla: a normal turn is not cut off by a
wall-clock cap. A run continues until the model stops, the provider fails, or
the user aborts it. Optional opt-in backstops remain available:

- `OPENCODE_RUN_TIMEOUT_MS` - optional hard wall-clock budget for one queued
  run. Default is `0` (disabled), matching vanilla; set a positive value to opt
  into a backstop for unattended runs. On expiry the run is cancelled and the
  session is finalized with a durable error instead of staying `busy`.
- `OPENCODE_STREAM_IDLE_TIMEOUT_MS` - idle gap tolerated between provider
  stream events before the step is treated as stalled and fails loudly
  (default `300000`, 5 minutes, matching vanilla's provider chunk timeout; `0`
  disables).
- `OPENCODE_STREAM_BUDGET_MS` - optional wall-clock budget for a single
  provider step, used to bound an endless reasoning loop that keeps emitting
  events. Default is `0` (disabled), matching vanilla; set a positive value to
  opt in.

Neither `OPENCODE_RUN_TIMEOUT_MS` nor `OPENCODE_STREAM_BUDGET_MS` is set by
default, so long agentic runs are allowed to finish just like vanilla.

Aborting a run (`Esc`) cancels it even when it is parked on an await that does
not observe the prompt token, so the session returns to idle instead of wedging.

The local server's stdout/stderr is written to
`dirs::data_local_dir()/opencode/traces/server.log` (macOS:
`~/Library/Application Support/opencode/traces/server.log`) instead of being
discarded, and a panic hook appends the panic message, source location, and
backtrace there. Set `OPENCODE_SERVER_LOG` to override the path, or to
`0`/`false`/`off` to disable the durable sink. Find it with
`opencode-modded-rust debug paths` (`server-log`) or `opencode-modded-rust session inspect <id>`
(`Server log:`). Turn progress is persisted incrementally (throttled to ~1s) so
exiting, stalling, or aborting mid-turn keeps the latest chunk for resume.

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
- Release and fleet deployment: `docs/release-deployment.md`

## Notes

- `opencode-modded-rust` is the canonical product name for planning and product-facing documentation.
- The executable name is `opencode-modded-rust` (launched via `omr`/`ort`).
- `opencode-*` crate names are retained as implementation identifiers; `scopemux-code` is a retired/legacy name.
- `opencode-modded` remains the long-term reference and planning repo.
- The TypeScript reference line for planning and later sync review is `$HOME/repos/opencode-modded`, referenced from its `dev` branch; fetch the latest `dev` before comparing vanilla behavior. The recorded pin is `f54ce313b99a6661d7758ad042f7a6e05c8e0972` (re-pinned by `GATE-004`; the prior `e62912b5` pin is unreachable).

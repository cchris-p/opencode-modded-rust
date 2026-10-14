---
id: "CLI-013"
title: "Reach the Rust CLI subcommands through the launcher"
priority: "P2"
type: "feature"
area: "CLI"
spec: "wiki/cli-surface.md"
status: "done"
predecessors: "START-034"
created: "2026-10-14"
updated: "2026-10-14"
---

# Reach the Rust CLI subcommands through the launcher

## Summary

Make the Rust product's non-TUI subcommands reachable through a launcher instead of only via the raw
binary path. Today the user-facing launchers force the `tui` subcommand, so `task`, `serve`,
`session`, `attach`, and the rest of the headless surface (built by `CLI-001`/`CLI-006`/`CLI-012`)
have no convenient entry point.

## Why this exists

- `_opencode_rust_exec "$@"` (`~/standards/opencode-config:950`) is already a general passthrough, but
  every user-facing launcher hardcodes a subcommand:
  - `opencode-rust-tui` (`:1307`) always runs `_opencode_rust_exec tui ...`.
  - `ort` (`:1327`) and `omr` (`:1331`) both call `opencode-rust-tui`.
  - `opencode-rust-run` (`:1536`) only runs `run`.
- So there is no shorthand for `task`/`serve`/`session`/`attach`. The whole headless Cline-like
  workflow is only runnable as `$OPENCODE_RUST_REPO/target/debug/opencode task ...`.
- `ort --help` shows only the `tui` subcommand help, hiding the rest of the CLI surface, which is
  what made the feature look missing.

## Dependency

- Depends on `START-033` (command/binary name) and `START-034` (name scrub); sequenced after both.
  The launcher cannot be cleanly renamed or dispatched until the product identity is fixed and the
  stale `scopemux-code` name is gone. Do not start until they land.

## Proposed design

- Give the product launcher subcommand dispatch: a recognized non-TUI subcommand
  (`task`, `serve`, `session`, `attach`, `run`, …) passes through to `_opencode_rust_exec "$@"`;
  bare launcher invocation and TUI-only flags (`--model`, `--session`, `--fork`, `--continue`,
  `--attach`, `--port`, …) still launch the TUI.
- Make help/discoverability honest: the launcher help (or a documented `--help` path) must show the
  product's full top-level command list, not just `tui`.
- Preserve `ort` as the TUI launcher for back-compat; the passthrough name follows the `START-033`
  decision.

## Resolved Decisions (locked 2026-10-14)

- **Passthrough launcher:** `omr` becomes the general launcher — TUI by default, subcommand
  passthrough for known subcommands. `ort` stays TUI-only for back-compat.
- **Dispatch rule:** explicit allowlist of the product's top-level subcommands; any first token not
  in the allowlist (including TUI flags such as `--model`/`--session`/`--fork`/`--attach`/`--port`)
  launches the TUI.
- **Help:** `omr --help` delegates to the product's top-level `--help` (full command list).

## Scope

- Launcher subcommand passthrough and TUI-flag disambiguation (standards `opencode-config`).
- Discoverable help that lists the full command surface.
- Docs update so the headless CLI path is documented alongside the TUI path.

## Non-goals

- Renaming the product/binary/clap command (that is `START-033`).
- Changing server lifecycle, attach/detach policy, or `opencode` (vanilla) behavior.
- `opencode run` routing (`CLI-002`, `hold`).

## Done when

- `ort` (or the agreed launcher) runs `task`/`serve`/`session`/`attach` without the raw binary path.
- Bare launcher invocation and TUI flags still launch the TUI with no behavior change.
- The launcher help surfaces the full command list.
- `opencode` remains vanilla and unchanged.

## Recommended verification

- `ort serve`, `ort task new "hi"`, `ort task view`, `ort session list`, `ort attach <url>` all reach
  the product.
- `ort`, `ort --model <m>`, `ort --continue` still launch the TUI.
- `ort --help` (or the documented path) lists the product's commands.

## Related Items

- `START-033` Decouple the product command identity from opencode — sets the command name.
- `START-034` Scrub scopemux-code product-name references — predecessor; removes the stale name.
- `CLI-001` Cline-style CLI task send — built the `task`/`session`/`attach` surface this makes reachable.
- `CLI-006` CLI status visibility; `CLI-012` CLI task file attachments — headless surface made reachable.
- `CLI-003`/`CLI-005` — launcher/TUI lifecycle decisions this must not disturb.
- `wiki/cli-surface.md` — canonical CLI-surface reference.

## Notes

- Created 2026-10-14; follow-up to `START-033` per user direction ("name decoupling first, then the
  CLI improvement").

## Implementation Notes

- 2026-10-14: implemented in standards `~/standards/opencode-config` (committed on its `development`
  branch; H-015/CLI-013), after `START-033`/`START-034` landed on product `development`.
  - `omr` is now the general launcher: the first non-flag token is checked against the explicit
    allowlist `task serve session attach run import export mcp auth models pr debug stats db acp web`.
    A match passes straight through to `_opencode_rust_exec "$@"`; anything else (bare `omr` or TUI
    flags `--model`/`--session`/`--fork`/`--attach`/`--port`) launches the TUI.
  - `omr --help`/`-h`/`help` delegates to the product's top-level `--help` (full command list).
  - `ort` remains TUI-only for back-compat; `opencode` on `PATH` is untouched vanilla.
  - `_opencode_rust_exec` and `ort-doctor` binary path updated to
    `$OPENCODE_RUST_REPO/target/debug/opencode-modded-rust`.
- Verification (stubbed `_opencode_rust_exec`): `omr serve`, `omr task new hi`, `omr task view`,
  `omr session list`, `omr attach <url>`, `omr models`, `omr stats` → passthrough; `omr --model m`,
  `omr --continue`, bare `omr`, `ort` → TUI; `omr --help` → top-level help.
- Docs: `wiki/cli-surface.md` documents the `omr` launcher reachability and uses `omr` in the target
  workflow (product `development`).
- Merge: product PRs #142 (`START-033`) and #143 (`START-034`) merged into `development` in order.

## QA / Verification (2026-10-14, on `development`)

- `omr --help` lists the full top-level command surface (25 commands).
- Passthrough: `omr version`/`--version` → `opencode-modded-rust 0.1.0`; `omr debug paths`,
  `omr session list`, `omr attach --help`, `omr config`, `omr agent --help`, `omr github` reach the
  product.
- Real headless smoke: `omr serve --port 4319` → `GET /health` = `{"status":"ok","version":"0.1.0"}`.
- TUI routing: bare `omr`, `omr --model <m>`, `omr --continue`, `omr tui`, and `ort` launch the TUI.
- `opencode` remains vanilla and unchanged.
- **QA PASS.**


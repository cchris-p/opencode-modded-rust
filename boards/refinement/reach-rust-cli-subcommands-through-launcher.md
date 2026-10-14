---
id: "CLI-013"
title: "Reach the Rust CLI subcommands through the launcher"
priority: "P2"
type: "feature"
area: "CLI"
spec: "wiki/cli-surface.md"
status: "refinement"
predecessors: "START-033"
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

- Depends on `START-033`, which decides the product command/binary name. The launcher cannot be
  cleanly renamed or dispatched until that identity is fixed. Do not start until `START-033` lands.

## Proposed design

- Give the product launcher subcommand dispatch: a recognized non-TUI subcommand
  (`task`, `serve`, `session`, `attach`, `run`, …) passes through to `_opencode_rust_exec "$@"`;
  bare launcher invocation and TUI-only flags (`--model`, `--session`, `--fork`, `--continue`,
  `--attach`, `--port`, …) still launch the TUI.
- Make help/discoverability honest: the launcher help (or a documented `--help` path) must show the
  product's full top-level command list, not just `tui`.
- Preserve `ort` as the TUI launcher for back-compat; the passthrough name follows the `START-033`
  decision.

## Open Decisions

- Which launcher becomes the passthrough: extend `ort`/`omr`, or add a dedicated CLI launcher (e.g.
  `<name>-cli`)?
- Which first-argument tokens count as subcommands vs TUI flags (allowlist vs "anything not a known
  TUI flag").
- Whether the launcher help should delegate to the product's top-level `--help`.

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

- `START-033` Adopt the scopemux-code command identity — predecessor; decides the launcher name.
- `CLI-001` Cline-style CLI task send — built the `task`/`session`/`attach` surface this makes reachable.
- `CLI-006` CLI status visibility; `CLI-012` CLI task file attachments — headless surface made reachable.
- `CLI-003`/`CLI-005` — launcher/TUI lifecycle decisions this must not disturb.
- `wiki/cli-surface.md` — canonical CLI-surface reference.

## Notes

- Created 2026-10-14; follow-up to `START-033` per user direction ("name decoupling first, then the
  CLI improvement").

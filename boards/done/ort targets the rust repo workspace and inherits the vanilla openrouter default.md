---
id: "BUG-011"
title: "ort targets the rust repo workspace and inherits the vanilla openrouter default"
priority: "P1"
type: "bug"
area: "BUG"
spec: "AGENTS.md"
status: "done"
created: "2026-09-15"
---

# ort targets the rust repo workspace and inherits the vanilla openrouter default

## Summary

Launching `ort` from a directory other than this repo does not operate on the directory
it was activated from, and the default model/provider is wrong:

- The TUI shows the workspace as the rust repo (`$OPENCODE_RUST_REPO`) instead of the
  activated directory (user report, 2026-09-15, launching in a `~/apps/*notes` dir).
- The default resolves to `openrouter/deepseek/deepseek-v4-flash:free`
  ("deepseek by openrouter") instead of `deepseek/deepseek-v4-flash`
  ("deepseek by deepseek").

Expected: `ort` runs against the directory it was activated in, and defaults to the
deepseek-direct provider unless that workspace's own config overrides it.

## Evidence (2026-09-15)

Same binary, different cwd (`target/debug/opencode config`):

- `cd ~/repos/opencode-modded-rust` -> `Default model: deepseek/deepseek-v4-flash`,
  `Working directory: .../opencode-modded-rust`.
- `cd /tmp` -> `Default model: openrouter/deepseek/deepseek-v4-flash:free`,
  `Working directory: /tmp`.

Launcher (`~/standards/opencode-config`): `_opencode_rust_exec` forced
`cd "$repo"` (workspace always the rust repo) and ran with
`env -u OPENCODE_CONFIG_CONTENT -u OPENCODE_CONFIG_DIR`. Outside the repo there is no
project config, so the product fell back to the shared global
`~/.config/opencode/opencode.json`, whose `model` is written to
`openrouter/deepseek/deepseek-v4-flash:free` by `opencode-use-auto` at shell load.

## Root cause

Two independent mechanisms stack:

1. **Workspace force-retarget (launcher).** `_opencode_rust_exec` `cd`-ed into
   `$OPENCODE_RUST_REPO`, so the TUI/server cwd — and therefore the displayed workspace
   and config search root — was always the rust repo regardless of where `ort` was called.
2. **Wrong default provider (config fallback).** The rust product has no built-in default
   model; with no workspace config it inherits the shared vanilla global config
   (`~/.config/opencode/opencode.json` -> openrouter). The rust product "owns its
   provider/model default" only for the repo workspace.

## Interim local change (2026-09-15, not repo-tracked)

`~/standards/opencode-config` `_opencode_rust_exec` was changed to:

- drop the forced `cd "$repo"` so `ort` runs in the activated directory, and
- seed a low-precedence inline default instead of leaking the vanilla overlay:
  `env -u OPENCODE_CONFIG_DIR OPENCODE_CONFIG_CONTENT='{"$schema":"https://opencode.ai/config.json","model":"deepseek/deepseek-v4-flash"}'`.

Verified: `/tmp` -> deepseek + workspace `/tmp`; a dir with its own `opencode.jsonc` ->
project config wins; repo -> deepseek + workspace repo. Project config still beats the
inline default, matching `BUG-010`.

Caveats:

- Machine-local only; not durable in the product.
- Requires re-sourcing/restarting the shell. Long-lived tmux panes holding the pre-change
  `_opencode_rust_exec` are why a re-test still showed the rust-repo workspace/openrouter.

## Why this exists

The V1 target is a personal daily-driver on a narrow workflow. A launcher that silently
retargets to the rust repo and uses a different provider than the configured default is
unusable for working in any other workspace, and it makes every provider/model
observation untrustworthy.

## Scope

- Make `ort` operate on the directory it was activated from (workspace, config root, and
  TUI display must all reflect that directory).
- Give the rust product a durable default model/provider (`deepseek/deepseek-v4-flash`)
  that does not depend on a shell wrapper, while still letting workspace config win.
- Do not regress `BUG-010` precedence: workspace config beats global config and the
  inline env overlay.

## Non-goals

- Changing the vanilla `opencode` global default / `opencode-use-auto` behavior.
- Broad onboarding UX.
- Model catalog/provider work.

## Done when

- `ort` launched in any directory shows that directory as the workspace.
- With no workspace config, the effective default is `deepseek/deepseek-v4-flash`.
- A workspace `opencode.json{,c}` still overrides the default.
- Behavior holds without relying on an updated shell function (i.e. in the product), or a
  documented, tested launcher contract is explicitly accepted as the mechanism.

## Recommended verification

- Fresh shell, `cd /tmp && opencode-rust config` -> deepseek + workspace `/tmp`.
- Fresh shell in the repo -> deepseek + workspace repo.
- Fresh shell in a dir with a distinct `opencode.jsonc` -> project model wins.
- Live `ort` from a non-repo workspace: confirm the TUI workspace matches and the model
  label is deepseek-direct.

## Related Items

- `BUG-010` Repo default model and provider ignored due to config precedence (precedence fix this must not regress)
- `FEAT-014` Enforce a single local TUI server per workspace (server/workspace selection)
- `FEAT-016` Remove local TUI server reuse so every ort run starts a fresh server for the activated workspace
- `FEAT-015` Preserve manually selected model and provider across sessions
- `START-027` Unify provider setup into one authoritative user path
- `PHASE-001` V1 daily-driver hardening

## Notes

- Surfaced after `BUG-010` moved the fix into the product; the launcher `cd` remained and
  only masked the wrong-default path while inside this repo.
- No repo code changed for this card yet; this card exists for user evaluation of scope.

## Implementation - 2026-09-15

### What changed

- `crates/opencode-config/src/loader.rs`: added `pub const DEFAULT_MODEL =
  "deepseek/deepseek-v4-flash"`. In `ConfigLoader::load_all`, the product default is applied
  immediately after `load_global()`, replacing the global-config-derived `model`. The shared
  `~/.config/opencode/opencode.json` (vanilla, usually generated by `opencode-use-auto`) no
  longer dictates this product's default; `OPENCODE_CONFIG`, `OPENCODE_CONFIG_CONTENT`,
  project `opencode.json{,c}`, `.opencode` directories, and managed config still override it.
  This preserves `BUG-010` precedence (workspace config beats everything below it).
- `AGENTS.md`: launcher section updated (activated directory is the workspace; product owns
  the default model; fresh server per launch).
- Machine-local, not repo-tracked: `~/standards/opencode-config` `_opencode_rust_exec` dropped
  the forced `cd "$repo"` and now only unsets `OPENCODE_CONFIG_CONTENT` /
  `OPENCODE_CONFIG_DIR` (no model seeding). `ort` therefore runs in the activated directory
  and the product provides the deepseek default.

### Verification

- `cargo test -p opencode-config` -> 53 passed, including the new
  `product_default_model_applies_without_workspace_config` and
  `workspace_config_overrides_product_default_model`.
- `cargo build -p opencode-cli` succeeds.
- Fresh shell from `/tmp`: `opencode-rust config` -> `deepseek/deepseek-v4-flash`,
  working directory `/tmp`.
- Directory with its own `opencode.jsonc` -> project model wins (`ollama/qwen3:30b` in test).
- Repo -> `deepseek/deepseek-v4-flash`, working directory the repo.
- Fresh detached server started in a non-repo workspace (`/config/providers`) ->
  `effective_model: deepseek/deepseek-v4-flash`.
- Explicit override still wins: `OPENCODE_CONFIG_CONTENT={"model":"openrouter/..."}` from a
  non-repo dir -> openrouter.

### Still open (this card)

- Live TUI verification on the PR branch by the user: launch `ort` from a non-repo workspace
  and confirm the workspace and model label are correct.
- The workspace-display half depends on the machine-local launcher change (re-source or
  restart the shell); the product default is the durable half.

### PR Link

- https://github.com/cchris-p/opencode-modded-rust/pull/33 (base `development`)

## Merge status

- Merged into `development` on 2026-09-15 via PR #33 (merge commit `9ef8b47`); PR branch deleted.
- Item in `qa`; awaiting post-merge user verification on `development` (activated workspace +
  `deepseek/deepseek-v4-flash` default). Note the workspace half also requires the
  machine-local launcher update (`~/standards/opencode-config`).

## Follow-up - 2026-09-15

- Applied the machine-local launcher update on this machine: `_opencode_rust_exec` no longer
  changes into `$OPENCODE_RUST_REPO` before running the built binary.
- The launcher execution path now unsets `OPENCODE_CONFIG_CONTENT` and `OPENCODE_CONFIG_DIR`,
  matching the environment needed for the product default to win in unconfigured workspaces.
- Verified directly from a temp non-repo workspace with the cleaned launcher environment:
  working directory remained the temp workspace and default model resolved to
  `deepseek/deepseek-v4-flash`.

## Follow-up - 2026-09-15 (second pass)

- Found a remaining product-side miss: a normal shell still exports
  `OPENCODE_CONFIG_DIR=$HOME/.config/opencode` and model-only `OPENCODE_CONFIG_CONTENT` from
  `opencode-use-auto`. Running the binary directly under that environment still selected the
  vanilla openrouter model.
- Updated `opencode-config` so the shared global config dirs are protected from the late
  `OPENCODE_CONFIG_DIR` scan, and model-only inline content associated with that shared config
  is ignored by this product. Workspace config and richer inline config overlays still apply.
- Verification: `cargo test -p opencode-config` -> 56 passed; `cargo build -p opencode-cli`
  passed; direct `target/debug/opencode config` and fresh-shell `opencode-rust config` from a
  temp non-repo workspace both reported that workspace and `deepseek/deepseek-v4-flash` under
  the normal shell environment.

## Completion - 2026-09-15

- User verified the live `ort` workflow now uses the activated workspace and the expected
  `deepseek/deepseek-v4-flash` default.
- Moved to `done`.

## Follow-up QA - 2026-09-16

- User re-verified `ort` from `/Users/cchrisleepyles/apps/tss-notes`; the fresh local server
  started with workspace `/Users/cchrisleepyles/apps/tss-notes`.
- No edit to the vanilla/global `~/.config/opencode/opencode.json` is required for this product's
  default model on other machines. The repo code owns the `deepseek/deepseek-v4-flash` product
  default.
- Cross-machine launcher requirement remains: the machine's `ort` wrapper must run the Rust binary
  in the activated directory and must not force the vanilla `OPENCODE_CONFIG_CONTENT` /
  `OPENCODE_CONFIG_DIR` overlay into this product.

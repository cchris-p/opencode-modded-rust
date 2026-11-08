---
id: "BUG-010"
title: "Repo default model and provider ignored due to config precedence"
priority: "P1"
type: "bug"
area: "BUG"
spec: "wiki/v1.md"
status: "done"
created: "2026-09-11"
---

# Repo default model and provider ignored due to config precedence

## Summary

The repo's configured default model/provider (`opencode.jsonc` -> `deepseek/deepseek-v4-flash`) was not the effective default for the Rust product. Commit `36dbbd2` set the project config, but the value was silently overridden at runtime.

## Root cause

Two independent overrides outranked the project config:

1. **Global config re-applied after project config (product bug).**
   `ConfigLoader::load_all` loads global config, then project config, then scans discovered `.opencode` directories. `collect_opencode_directories` includes the global config directory (`~/.config/opencode`), and the scan unconditionally reloaded `opencode.json{,c}` from every discovered directory. That re-merged the global `opencode.json` *after* the project config, so global `model = openrouter/deepseek/deepseek-v4-flash:free` beat the project `model = deepseek/deepseek-v4-flash`. This also contradicted the precedence model recorded on `START-026` (workspace-local config should beat global config).

2. **Shell-launch env overlay.**
   The shell exports `OPENCODE_CONFIG_CONTENT='{"model":"openrouter/deepseek/deepseek-v4-flash:free"}'` (an auto-selected vanilla-opencode default) and `OPENCODE_CONFIG_DIR=~/.config/opencode`. `OPENCODE_CONFIG_CONTENT` was applied *after* project config (upstream "local" precedence), and `OPENCODE_CONFIG_DIR` pointing at the global config dir caused the directory scan to re-merge global config after project config. So `ort`/`opencode-rust-tui` always inherited the vanilla opencode model selection.

## Session work (2026-09-11)

- `crates/opencode-config/src/loader.rs`: added `directory_contributes_config(dir, global_config_dir, config_dir_override)` and used it in `load_all`. The directory scan now only loads `opencode.json{,c}` from real `.opencode` directories or an explicit `OPENCODE_CONFIG_DIR` override, and never from the global config directory (even when `OPENCODE_CONFIG_DIR` redundantly points there).
- `crates/opencode-config/src/loader.rs`: `OPENCODE_CONFIG_CONTENT` is now applied *before* project config in `load_all`, so an explicit workspace config beats the inline env overlay. This is a deliberate deviation from strict TS parity (documented in the code comment) because the Rust product is launched from shells that set that var as a vanilla-opencode model default.
- Added unit tests `test_directory_contributes_config_only_for_opencode_dirs` and `project_config_overrides_inline_env_content`.
- `~/standards/opencode-config`: `_opencode_rust_exec` now also runs the Rust binary with `env -u OPENCODE_CONFIG_CONTENT -u OPENCODE_CONFIG_DIR` as defense-in-depth. This is a machine-local launcher change, not repo-tracked. It requires the interactive shell to be re-sourced; the product-side fix above makes it unnecessary.

## Verification

- `cargo build -p opencode-cli`.
- `cargo test -p opencode-config` -> 51 passed.
- `./target/debug/opencode debug config` with the full leaking shell env: `"model": "deepseek/deepseek-v4-flash"`.
- Fresh server with the full leaking shell env: `/config/providers` -> `effective_model: deepseek/deepseek-v4-flash`, `selection_source: Settings > Provider`.
- A manual `PATCH /config` selection is written to project `opencode.json` and wins over `opencode.jsonc` on the next load (persistence path verified).

## Done when

- The repo-configured default model/provider is the effective default when the user has not explicitly overridden it.
- A workspace/project config beats the global config for `model`.
- A workspace/project config beats the shell's `OPENCODE_CONFIG_CONTENT` inline overlay.

## Notes

- First re-test still showed openrouter because the long-running tmux shell still held the pre-fix `_opencode_rust_exec` function; the launcher fix only applies after re-sourcing. This is why the fix was moved into the product rather than relying on the shell wrapper.

## Related Items

- `START-027` Unify provider setup into one authoritative user path (settings persist selection; env overlays documented)
- `START-026` Make permission allow/deny config deterministic (precedence model)
- `START-019` Add native Ollama support for the local-model-first V1 path
- `FEAT-015` Preserve manually selected model and provider across sessions (follow-up)
- Prior attempt: commit `36dbbd2` (`chore(config): default to deepseek/deepseek-v4-flash provider`)
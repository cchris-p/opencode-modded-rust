---
id: "FEAT-053"
title: "Make TUI display-toggle defaults configurable from opencode.json"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "qa"
predecessors: "BUG-022"
created: "2026-09-23"
updated: "2026-09-23"
---

# Make TUI display-toggle defaults configurable from opencode.json

## Summary

The TUI display toggles (thinking, tool calls, tool details, timestamps, message density, header,
scrollbar, tips, semantic highlight) persist across sessions, but only in an internal runtime state
file. They cannot be set or defaulted from `opencode.json`, so a user cannot express display
preferences declaratively, cannot scope them per workspace, and cannot commit them to a repo. Add
config-schema keys for these toggles and make the TUI honor them at startup.

## Why this exists

Reported by user 2026-09-23: after the `FEAT-028` tool-calls toggle closed, they asked whether the
setting survives across sessions and, if not, requested it be made persistent and editable in a
config file. Persistence already exists, but configurability does not — the state lives only in
`dirs::state_dir()/opencode/kv.json`, which is global to the machine, undocumented, and not
expressible in config.

## Current behavior and evidence

- Display state is loaded from a JSON key-value blob and written back on every toggle:
  `UiKv::load` / `UiKv::persist` and `ui_kv_path()` at
  `crates/opencode-tui/src/context/app_context.rs:373-471`.
- Each flag is seeded from that blob with a hard-coded default, then flipped by a toggle that writes
  the blob:
  - `show_thinking` (`app_context.rs:154`, `toggle_thinking` `:238-242`)
  - `show_tool_calls` (`app_context.rs:155`, `toggle_tool_calls` `:252-256`)
  - `show_tool_details` (`app_context.rs:156`, `toggle_tool_details` `:244-250`)
  - `show_timestamps` (`app_context.rs:153`, `toggle_timestamps` `:232-236`)
  - `message_density` (`app_context.rs:157-159`, `toggle_message_density` `:258-267`)
  - `semantic_highlight` (`app_context.rs:160`, `toggle_semantic_highlight` `:269-273`)
  - `show_header` (`:147`), `show_scrollbar` (`:148`), `tips_hidden` (`:149`)
- The observed file on this macOS host is `~/.local/state/opencode/kv.json` and currently contains,
  among others, `"tool_calls_visibility": false`. It is machine-global, not per workspace.
- The config schema already has a `tui` section and display-adjacent keybinds, but no visibility
  defaults: `Config.tui` (`crates/opencode-config/src/schema.rs:23`), `TuiConfig`
  (`schema.rs:331-342`), `keybinds.tool_details` (`schema.rs:207`), `keybinds.display_thinking`
  (`schema.rs:319`).
- The TUI already consumes `opencode-config` in the settings/provider surfaces and can `PATCH`
  server config (`crates/opencode-tui/src/api.rs:616`), and the settings screen is provider/auth
  only, not display toggles (`crates/opencode-tui/src/components/settings.rs`).

## Scope

- Add `tui` config keys mirroring the persisted ui keys, e.g. `thinking`, `tool_calls`,
  `tool_details`, `timestamps`, `message_density`, `semantic_highlight`, `header`, `scrollbar`,
  `tips_hidden` (names and serde aliases to be settled during refinement).
- Define precedence explicitly, e.g. persisted kv value (last explicit toggle) > config value >
  built-in default, or make config authoritative on startup and kv the runtime override.
- Seed `AppContext` display flags from config when the corresponding kv key is absent, instead of
  the current hard-coded defaults (`app_context.rs:146-160`).
- Document the keys in `docs/opencode-config.md` and the persistence file in `docs/opencode-tui.md`
  so the runtime state file is no longer hidden.

## Non-goals

- Replacing or removing the `kv.json` runtime persistence or the slash commands / palette entries /
  keybinds that toggle these settings.
- Per-tool-type filtering or a new dedicated display-settings screen.
- Server-side session storage or the message/part model.
- Changing the meaning of any individual toggle.

## Done when

- Setting the display keys in `opencode.json` changes the TUI's startup rendering without requiring
  a `/` command first.
- Precedence between config and the persisted kv state is defined, implemented, and covered by a
  unit test.
- Existing slash commands, command-palette entries, and keybinds still toggle the same state and
  still persist to `kv.json`.
- `docs/opencode-config.md` and `docs/opencode-tui.md` describe the keys and the state file.

## Recommended verification

- Run `ort-build`, then `ort`; set `tui.tool_calls: false` (or the settled key) in a clean
  `opencode.json` with no prior kv entry, and confirm tool calls start collapsed.
- Toggle with `/tool-calls`, restart the TUI, and confirm the runtime toggle still wins/persists per
  the chosen precedence.
- Repeat for at least one boolean and the `message_density` string toggle.
- Add unit tests for config-seeded defaults and precedence.
- Confirm no regression in the existing display-toggle tests.

## Product decisions

- Persistence already works; this item is about declarative configuration and precedence, not about
  fixing a missing persistence path.
- Prefer defining config as the startup default with the persisted kv value as an explicit override,
  so an in-session toggle is never silently reverted on restart.
- Keep `kv.json` as the runtime store rather than promoting it into the config file.

## Related Items

- `FEAT-028` Add a hide-tool-calls toggle that compacts runs to a tool-call count
- `FEAT-008` Disable TUI sidebar by default
- `FEAT-032` Rebrand TUI branding constants

## Notes

- `dirs::state_dir()` is the resolution root (`app_context.rs:460-471`), with a `~/.local/state`
  fallback; confirm the macOS path matches the observed host when documenting.
- `UiKv` also holds keys not defined in `AppContext` (for example `generic_tool_output_visibility`,
  `assistant_metadata_visibility`, `diff_wrap_mode`, `sidebar`, `thinking_mode`), so the config keys
  should be introduced deliberately rather than mirroring the whole file.
- Coordinate with `BUG-022` (`/thinking` collapse semantics) so the config `thinking` key maps to
  the corrected behavior, not the current line-count rendering.
- Refinement settled 2026-09-23 (handoff `H-007`): keys are snake_case with no aliases —
  `thinking`, `tool_calls`, `tool_details`, `timestamps`, `message_density`, `semantic_highlight`,
  `header`, `scrollbar`, `tips_hidden`; `message_density` accepts `compact`/`cozy`. Precedence is
  persisted `kv.json` override > config startup default > built-in default. The TUI loads config with
  `ConfigLoader::load_all(workspace_dir)` and seeds via `AppContext::new_with_config`.

## Dev Notes - 2026-09-23

- Added the nine snake_case keys (`thinking`, `tool_calls`, `tool_details`, `timestamps`,
  `message_density`, `semantic_highlight`, `header`, `scrollbar`, `tips_hidden`) to `TuiConfig`
  (`crates/opencode-config/src/schema.rs`).
- Added `UiKv::get_bool_opt` / `get_string_opt` / `get_timestamps_opt` and
  `seed_bool` / `seed_string` / `seed_timestamps` helpers implementing the precedence rule.
- `AppContext::new_with_config(&Config)` seeds every display flag; `AppContext::new()` delegates to
  `Config::default()`. `App::new()` resolves the workspace directory, loads config once with
  `opencode_config::load_config` (same as the existing FEAT-048 capability read), and builds the
  context from it.
- Tests: `config_seeds_display_flags_when_kv_is_absent`,
  `persisted_kv_override_beats_config_startup_default`,
  `builtin_defaults_apply_when_kv_and_config_are_absent`,
  `timestamps_opt_decodes_string_and_bool_encodings`.
- Docs: `docs/opencode-config.md` gained the `tui` key table; `docs/opencode-tui.md` documents the
  `kv.json` runtime store and precedence.
- Verification: `cargo test -p opencode-config` (64 passed) and `cargo test -p opencode-tui` green.
- Branch `feature/tui-display-cluster`; awaiting local QA on the open PR.

### Merge Closeout - 2026-09-23

- Merged into `development` via PR #103 (merge commit `21d7d55`).
- Feature branch `feature/tui-display-cluster` deleted (remote and local); local checkout returned to `development`.
- Remains in `qa` pending a QA report or explicit completion.

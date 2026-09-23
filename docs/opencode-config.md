# opencode-config

`opencode-config` handles config file discovery, loading, parsing, and merging. It is the configuration entrypoint for runtime behaviour.

## Responsibilities

- Search for config files (project and global)
- Parse JSON/JSONC (comments supported)
- Map config to strongly typed structures
- Provide well-known paths and defaults

## Module structure

- `loader.rs` – Config loading, path lookup, merge flow
- `schema.rs` – Config structure definitions
- `wellknown.rs` – Common directory/file path constants

## Config paths (common)

- Project: `opencode.jsonc` / `opencode.json`
- Project extension: `.opencode/opencode.jsonc` / `.opencode/opencode.json`
- Global: `~/.config/opencode/opencode.jsonc` (or `.json`)

## TUI display defaults (`tui`)

Display toggles can be declared under `tui` in `opencode.json{,c}` and act as
startup defaults. The persisted runtime value (see `opencode-tui`) always wins
over config, and config wins over the built-in default.

| Key | Type | Built-in default |
| --- | --- | --- |
| `thinking` | boolean | `true` |
| `tool_calls` | boolean | `true` |
| `tool_details` | boolean | `true` |
| `timestamps` | boolean | `false` |
| `message_density` | `compact` \| `cozy` | `compact` |
| `semantic_highlight` | boolean | `true` |
| `header` | boolean | `true` |
| `scrollbar` | boolean | `false` |
| `tips_hidden` | boolean | `true` |

Keys are snake_case with no aliases. They do not change the meaning of the
existing slash commands or keybinds, and they do not replace the other `tui`
fields (`mode`, `sidebar`, `scroll_speed`, `diff_style`).

## Usage notes

- When adding new config fields, define default behaviour
- Merge behaviour should remain predictable
- Changes to provider/mcp/agent fields should be reflected in docs

## Validation

```bash
cargo check -p opencode-config
```

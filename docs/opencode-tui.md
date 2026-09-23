# opencode-tui

`opencode-tui` provides the terminal UI: home screen, session view, input, sidebar, dialogs, shortcuts, and theme system.

## Branding and display

- `APP_NAME`: `scopemux-code`
- `APP_SHORT_NAME`: `scopemux-code`
- `APP_VERSION_DATE`: `2026.02.23`

Defined in: `crates/opencode-tui/src/branding.rs`

## Responsibilities

- Render session messages and tool results
- Manage input, completion, command palette, and dialogs
- Connect to server API and local event loop
- Theme, layout, and interaction state

## Key modules

- `app/` – Main event loop and state sync
- `components/` – home / session / prompt / sidebar / dialog
- `context/` – App state, key bindings, cache
- `api.rs` – Client for local server
- `file_index.rs` – `@path` completion index (nucleo matcher)
- `components/markdown/` – Code block rendering and syntect highlighting

## Current enhancements

- Overlay sidebar with explicit toggle (including `☰` button)
- Braille/KnightRider switchable spinner
- Refined message block layout and status line
- Syntect code highlighting and path-aware completion

## Display state and toggles

- Display toggles (thinking, tool calls, tool details, timestamps, message
  density, semantic highlight, header, scrollbar, tips) persist in a runtime
  key-value file: `dirs::state_dir()/opencode/kv.json` (for example
  `~/.local/state/opencode/kv.json`).
- Startup precedence: persisted `kv.json` value > `opencode.json` `tui` default
  > built-in default. See
  `crates/opencode-tui/src/context/app_context.rs` (`new_with_config`).
- `/thinking` surfaces live reasoning content by default; any block can be
  manually collapsed to the `▶ Thinking (N lines)` header.
- Tool calls render through the shared block pipeline (gutter, background,
  padding, word-aware wrapping). Bash/shell uses a distinct terminal block: a
  `$`-prefixed command wrapped to width, output in an inset `│` region, and a
  trailing running/exit-status line.

## Development notes

- UI changes should preserve scroll stability and low CPU usage
- Mouse handling should be tested for hover + scroll
- Text rendering must use character-boundary-safe handling (avoid UTF-8 slice panics)

## Validation

```bash
cargo check -p opencode-tui
```

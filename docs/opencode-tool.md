# opencode-tool

`opencode-tool` provides the tool invocation system: tool definition, registry, execution context, and built-in tool implementations.

## Responsibilities

- Define unified `Tool` interface
- Maintain tool registry (`registry`)
- Provide built-in tools (read/write/edit, shell, search, patch, todo, etc.)
- Integrate with permission system, plugin hooks, LSP/MCP

## Built-in modules (selection)

- File: `read`, `write`, `edit`, `multiedit`, `ls`
- Search: `grep_tool`, `code_search_tool`, `glob_tool`, `codesearch`
- Execution: `bash`, `batch`, `apply_patch`
- Task: `plan`, `task`, `todo`, `question`
- Network: `webfetch`, `websearch`
- Support: `registry`, `tool`, `truncation`

## Map-backed code search (`code_search`)

`code_search_tool` exposes `code_search`, which answers "where is X" from the
persistent ScopeMux map in one call. It receives the neutral `WorkspaceSearch`
capability through `ToolContext`; it never parses model strings or calls FFI.

- Supported languages and a fresh map: results come from the node registry with
  file, symbol, `block_id`, confidence, provenance, and freshness.
- Absent capability, unsupported workspace, stale/unknown map, or an empty map
  result: the tool falls back to the same bounded walk as `grep` (shared
  exclusions, file/byte/duration/result limits, `spawn_blocking`).
- `mode="prose"` always uses grep; `mode="code"` prefers the map; `mode="auto"`
  (default) uses the map when the capability is present and usable.
- Activation is the runtime's Qwen-only scope (`workspace_search_for`); every
  other model gets grep/generic behavior. The tool is evidence-only and has no
  task authority.

## Feature flags

- `lsp` feature: enables `opencode-lsp` and `lsp-types` integration

## Development notes

- New tools should be idempotent where possible and log for observability
- All side-effectful tools should go through the permission system
- Tool output should account for TUI truncation

## Validation

```bash
cargo check -p opencode-tool
cargo check -p opencode-tool --features lsp
```

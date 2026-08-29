# scopemux-code documentation index

This documentation set corresponds to the current code state of `scopemux-code` (repository shorthand: `opencode-modded-rust`, kept as a legacy repository identifier).

 ## Quick links
 
 - Project overview: `README.md`
 - User guide: `USER_GUIDE.md`
 - CLI: `docs/opencode-cli.md`
 - TUI: `docs/opencode-tui.md`
 - Server: `docs/opencode-server.md`
- Archived session brainstorm artifact: `docs/archive/session.md`
- Wiki index: `wiki/README.md`
- Vision: `wiki/vision.md`
- Version roadmap: `wiki/v1.md`, `wiki/v2.md`, `wiki/v3.md`
- Invariants index: `invariants/README.md`

## Module documentation

- `docs/opencode-agent.md` – Agent registration, execution, and message handling
- `docs/opencode-cli.md` – `opencode` command and subcommands
- `docs/opencode-command.md` – Slash command registration and rendering
- `docs/opencode-config.md` – Configuration loading and merging
- `docs/opencode-core.md` – Event bus and ID infrastructure
- `docs/opencode-grep.md` – Code and text search abstraction
- `docs/opencode-lsp.md` – LSP client and registry
- `docs/opencode-mcp.md` – MCP client, OAuth, transport layer
- `docs/opencode-permission.md` – Permission rules and decision engine
- `docs/opencode-plugin.md` – Hook system and TS subprocess bridge
- `docs/opencode-provider.md` – Multi-provider model adapter layer
- `docs/opencode-server.md` – HTTP routes, event stream, control endpoints
- `docs/opencode-session.md` – Session lifecycle and message flow
- `docs/opencode-storage.md` – SQLite storage and repository layer
- `docs/opencode-tool.md` – Built-in tools and tool registry
- `docs/opencode-tui.md` – Terminal UI architecture and interaction
- `docs/opencode-types.md` – Shared data types across modules
- `docs/opencode-util.md` – Filesystem, logging, and common utilities
- `docs/opencode-watcher.md` – Filesystem watcher

 ## Code and documentation conventions

 - `scopemux-code` is the canonical product name in product-facing and planning docs.
 - The command name is still `opencode` for compatibility with existing scripts.
 - `opencode-*` document and crate names follow the current code layout and are not alternate product branding.
 - Documentation should follow the source code and `--help` output as the source of truth.
 - Behaviour differences or refactoring plans are documented in `docs/overview/` (if that directory exists).
 - Product direction, version planning, and invariants live in `wiki/` and `invariants/`.

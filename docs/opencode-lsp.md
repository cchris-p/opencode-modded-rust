# opencode-lsp

`opencode-lsp` provides Language Server Protocol client support for code intelligence and editing assistance.

## Responsibilities

- Start and manage LSP subprocesses
- Handle JSON-RPC requests, responses, and notifications
- Maintain client state and event stream
- Manage the config-driven, lazy `(root, server)` registry

## Configuration

LSP is opt-in per workspace. Define servers under the `lsp` key in `opencode.json(c)`:

```jsonc
{
  "lsp": {
    "rust": {
      "command": ["rust-analyzer"],
      "extensions": [".rs"],
      "env": {},
      "initialization": {},
      "disabled": false
    }
  }
}
```

With no `lsp` block (or `"lsp": false`) no server is started and the product keeps
its generic behavior. The registry starts a server lazily the first time a matching
file needs it, keys clients by `(workspace root, server id)`, gates by extension,
and suppresses a server that fails to start for the registry's lifetime.

## Key types

- `LspClient`
- `LspClientRegistry` (`configure`, `client_for_file`, `ensure_for_file`, `status`, `shutdown_all`)
- `LspServerConfig`
- `LspServerStatus` / `LspConnectionStatus`
- `LspEvent`
- `JsonRpcRequest` / `JsonRpcResponse` / `JsonRpcNotification`
- `LspError`

## Use cases

- `opencode-tool` LSP tools (the `lsp` feature is on by default)
- Post-edit diagnostics in `write`, `edit`, and `apply_patch` when a server is available
- Server `/lsp` status and the TUI LSP indicator
- `opencode-modded-rust debug lsp ...` for one-shot diagnostics and symbols

## Semantic-fact interface

`opencode_lsp::facts` is the narrow contract consumers (notably ScopeMux) use instead of
importing `lsp_types` or issuing requests directly:

- `SemanticFactRequest { path, line, character, kind }` (0-based LSP positions).
- `SemanticFactKind`: definition, references, type definition, implementation, supertypes
  (type hierarchy), incoming/outgoing calls.
- `SemanticFactOutcome::Exact(SemanticFact)` or
  `Unavailable { reason: NoServer | Unsupported | Timeout | Failure, detail }`.
- `SemanticFact` carries `confidence = exact`, the server id, workspace root, document
  version, and content hash.

`LspClientRegistry::semantic_facts` negotiates against the server's advertised
capabilities, opens the document, applies the bounded request timeout, caches by
`(server, path, anchor, kind, content hash)`, and invalidates a file's entries when the
hash changes. `set_request_timeout` bounds every JSON-RPC request (default 10s). Consumers
must treat `Unavailable` as a normal fallback, never as a task failure.

## Development notes

- Handle timeouts, retries, and uninitialized server
- Keep URI/path conversion consistent
- Frame messages with `Content-Length` + exact body bytes; do not line-split the stream
- Consider backpressure and capacity for event broadcast channels

## Validation

```bash
cargo test -p opencode-lsp
```

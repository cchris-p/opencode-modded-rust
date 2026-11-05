---
id: "LSP-001"
title: "Standardize LSP runtime wiring and configuration"
priority: "P1"
type: "feature"
area: "LSP"
spec: "wiki/scopemux-lsp-boundary.md"
status: "qa"
created: "2026-11-05"
updated: "2026-11-05"
---

# Standardize LSP runtime wiring and configuration

## Summary

Make LSP a real, config-driven capability in ORT with vanilla-compatible configuration and
lifecycle semantics. The client and tool code already exist; the runtime wiring,
configuration behavior, status surface, and default build path do not.

This is the prerequisite for `LSP-002` and `SCOPE-013`.

## Why this exists

The current state is scaffolded but not usable end-to-end:

- `crates/opencode-lsp/src/lib.rs` implements the client, registry, and operations.
- `crates/opencode-tool/src/lsp_tool.rs` exposes the tool, but `opencode-tool`'s `lsp`
  feature is not default and the tool emits placeholder output without it.
- `write.rs`, `edit/tool.rs`, and `apply_patch.rs` collect diagnostics only when a registry
  is attached; no runtime path constructs one outside tests.
- The server `/lsp` endpoint is a stub returning an empty list
  (`crates/opencode-server/src/routes.rs:8160`).

So the product advertises LSP but does not actually start or use it.

## Scope

- Adopt vanilla's `lsp` config shape and activation semantics: a server map with
  `command`, `extensions`, `env`, `initialization`, and `disabled`; disabled unless
  configured.
- Implement the registry lifecycle for real: lazy per-`(root, server)` spawn, extension
  gating, root detection, broken-server suppression, shutdown, and health.
- Attach the registry to `ToolContext` and `SessionPrompt`.
- Make the LSP build path default (compile `opencode-lsp` in by default, or make the
  `opencode-tool` feature default) so released binaries are not placeholders.
- Replace the stub `/lsp` status endpoint with real connected-server status.
- Keep post-edit diagnostics bounded: current-file diagnostics, capped project-wide like
  vanilla's 5-file limit, with timeouts and no-op safeguards.
- Expose the existing debug CLI path consistently with the runtime registry.

## Acceptance criteria

- Configuring `lsp` starts the matching server lazily for a matching file; omitting or
  disabling `lsp` starts nothing and preserves the current generic behavior.
- The LSP tool runs against a real server instead of placeholder output in a default
  product build.
- Edit, write, and apply-patch return bounded diagnostics when a server is available and
  degrade cleanly when it is not.
- `/lsp` reports actual connected servers and roots; the TUI status reflects the same data.
- A missing, broken, or slow server does not fail the task and does not block the session.
- Config merge behavior matches the documented vanilla semantics.
- LSP remains opt-in per workspace; enabling it is not required for the product to work.

## Non-goals

- Building the semantic-fact interface for ScopeMux (that is `LSP-002`).
- Moving any LSP logic into ScopeMux or the retrieval provider.
- Requiring LSP for any task.
- Shipping a specific curated server list as a hard dependency.

## Dev Notes

- Reuse `opencode-lsp` and the existing diagnostics helpers; do not add a second client.
- Keep the tool surface identical to vanilla's operations plus any already-shipped
  extensions, and document any intentional divergence.
- Diagnostics must be best-effort; a server that never responds must not hang the tool.
- The server and TUI already have status types to reuse
  (`crates/opencode-tui/src/context/app_context.rs:51`).

## QA Verification

- PR: https://github.com/cchris-p/opencode-modded-rust/pull/149 (merged into `development`).
- `cargo fmt --all`, `cargo check --workspace`, `cargo clippy --workspace --all-targets`: pass.
- `cargo test -p opencode-lsp`: 12/12, including a fake JSON-RPC server that starts lazily
  and answers requests, broken-server suppression, extension gating, disabled server, and
  root selection.
- `cargo test -p opencode-tool`: 68/68.
- `cargo test --workspace --exclude opencode-scopemux --exclude opencode-tui`: all green.
- Pre-existing/environmental, reproduced on an untouched branch: `opencode-scopemux
  --features native` segfaults in the pinned FFI artifact; `opencode-tui
  tab_autocomplete_uses_first_candidate` fails. Neither is touched by this card.
- Manual `/lsp`/TUI and slow-server checks remain for QA. A bounded request timeout is
  delivered with `LSP-002` because it shares the same client.

## Related Items

- `LSP-002` Add a capability-negotiated LSP semantic-fact interface.
- `SCOPE-012` Consolidate ScopeMux and LSP responsibilities.
- `SCOPE-013` Ingest LSP-derived exact type facts into the ScopeMux map.
- `wiki/scopemux-lsp-boundary.md`
- `docs/opencode-lsp.md`

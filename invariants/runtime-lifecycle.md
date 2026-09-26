# Runtime Lifecycle Invariants

Canonical current/target behavior for the CLI/TUI launch, detach, attach, and session lifecycle:
`wiki/cli-surface.md`.

## Scope

- Covers runtime ownership of lifecycle state, explicit stage transitions, and the rule that leaving a
  session view never cancels background execution.
- Covers `ort` TUI launch and the explicit `/detach` + `opencode attach` reattachment paths.
- Sources: `crates/opencode-cli/src/main.rs` (`LocalTuiServer`, `:959-989,973-976`),
  `crates/opencode-tui/src/app/app.rs:345-346`, `crates/opencode-tui/src/command.rs:382-389`,
  board items `CLI-003`/`CLI-004` (`done`), and `wiki/cli-surface.md`.

## Invariants

- The runtime owns lifecycle state transitions.
- The model may make bounded decisions inside the runtime, but it does not control the overall workflow.
- Tasks move through explicit system-defined stages.
- Exiting a session view must not implicitly cancel active session execution.
- A user must be able to leave and later revisit a running session while the runtime continues advancing that session in the background.
- Completion state may only be reached through explicit verification.
- Every `ort` launch starts a fresh local server for the activated workspace; a prior server must not be
  implicitly reused, and reattachment is explicit (`opencode attach <url>` / `ort --attach <url>`).
- Only an explicit user detach leaves the launched server alive; normal exit terminates it.
- Same-workspace server attach/reuse is an open human decision (`CLI-005`, `hold`) and must not be
  reintroduced implicitly.


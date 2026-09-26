---
id: "BUG-048"
title: "Bash permission arity panics on short command nodes, killing the run"
priority: "P1"
type: "bug"
area: "BUG"
spec: "invariants/coding-session-behavior.md"
status: "qa"
created: "2026-09-26"
---

# Bash permission arity panics on short command nodes, killing the run

## Summary

`BashArity::prefix` indexes `tokens[..arity]` without clamping `arity` to the token count. When a
tree-sitter command node has fewer tokens than the command's configured arity (for example a
heredoc-only `python`, which maps to arity 2), the slice index panics with
`range end index 2 out of range for slice of length 1`.

This panic runs inside `BashTool::execute` → `parse_bash_command`, so it unwinds the whole prompt run.
It is the confirmed root cause of the deepseek run stalls: on a build without the `BUG-043`
terminal-state guard, the panicking run never finalizes, leaving the session stuck `busy` forever and
uninterruptible.

## Evidence

- Captured by the `BUG-045` panic hook in
  `~/Library/Application Support/opencode/traces/server.log`:
  - `[PANIC] range end index 2 out of range for slice of length 1`
  - `at crates/opencode-permission/src/arity.rs:10:30`
  - Backtrace: `opencode_permission::arity::BashArity::prefix` → `opencode_tool::bash::process_command_node`
    (`crates/opencode-tool/src/bash.rs:442`) → `parse_bash_command` → `BashTool::execute` →
    `execute_tool_calls` → `loop_inner` → `run_prompt_turn` → `drain_session_queue`.
- Live session `ses_2c1168ee88f644f49315ca1736064d20` remained `{status: busy}` after the panic
  (running binary predated the `BUG-043` guard).
- Deterministic reproduction: `cargo test -p opencode-permission --lib prefix` failed with the exact
  panic before the fix.

## Root cause

`crates/opencode-permission/src/arity.rs`:

```rust
if let Some(&arity) = ARITY.get(prefix.as_str()) {
    return tokens[..arity].to_vec(); // arity can exceed tokens.len()
}
```

Commands such as `python`, `git`, `docker`, `npm`, `cargo` are registered with arity 2+, but
`process_command_node` can produce a token list containing only the command name (for example when the
arguments are a heredoc redirect), so `arity > tokens.len()`.

## Fix

- Clamp the slice: `tokens[..arity.min(tokens.len())].to_vec()`.
- Regression tests: `prefix_does_not_panic_when_arity_exceeds_token_count`,
  `prefix_keeps_multi_token_arity`.

## Why this exists

Tree-sitter command parsing and the arity table disagree on token counts for redirect/heredoc forms.
The permission layer must tolerate short nodes; a panic here is fatal to the run and, without the
`BUG-043` guard, wedges the session.

## Done when

- `BashArity::prefix` never panics for any token slice.
- The regression tests pass.
- A `python - <<'EOF' …` style command parses and executes without panicking.

## Related Items

- `BUG-043` A session run can end without a terminal state - the panic this caused to wedge; now a
  safety net.
- `BUG-045` Server stderr/panics are discarded - the hook that captured this panic.
- `BUG-046` A never-idle provider stream is unbounded - companion bound.
- `BUG-016` / `BUG-023` Unresolved tool-call state in the session prompt loop.

## Notes

- Confirmed 2026-09-26. Relevant files: `crates/opencode-permission/src/arity.rs`,
  `crates/opencode-tool/src/bash.rs`.

## Dev Notes - 2026-09-26

- Clamped `arity` to `tokens.len()` in `BashArity::prefix` and added the regression tests.

## Verification - 2026-09-26

- Before fix: `cargo test -p opencode-permission --lib prefix` reproduced the exact panic
  (`range end index 2 out of range for slice of length 1` at `arity.rs:10:30`).
- After fix: `cargo test -p opencode-permission --lib` -> 16 passed, 0 failed.
- `cargo check --workspace` clean; `cargo fmt --all` clean.
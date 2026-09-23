# Agent Debugging Without The TUI

## Purpose

Use non-TUI paths as the primary way to debug the agent/session runtime. The TUI is a client of the server/session runtime; it should not be the only way to prove agent behavior.

Direct TUI testing is still required for TUI-specific behavior, but agent correctness should be debugged first through repeatable tests and HTTP/session APIs.

## Canonical Runtime Path

The canonical product path for agent-session QA is the server/session path:

- start `opencode serve`
- create or select a session through the server API
- send work through `/session/{id}/prompt`
- inspect results through `/session/{id}/message`

This is the same runtime layer that the TUI uses for coding-session prompts. It exercises `SessionPrompt`, agent context resolution, provider/model resolution, tool execution, permission/question plumbing, session updates, and stored messages without depending on terminal rendering or TUI input handling.

## Existing QA Suite

`QA-001` delivered the baseline non-TUI debug suite for session/stream runtime regressions:

- `cargo test -p opencode-provider`
- `cargo test -p opencode-session`
- `DEEPSEEK_API_KEY=<key> scripts/qa/stream-smoke.sh`

The deterministic tests cover SSE parsing and multi-turn session-loop behavior without network credentials. The live smoke script starts a fresh server, creates a session over HTTP, sends three prompts through `/session/{id}/prompt`, and reads replies through `/session/{id}/message`.

## Why Non-TUI First

Non-TUI debugging is preferred for agent/session failures because it isolates the runtime components that matter for agent correctness:

- provider streaming
- SSE parsing
- session prompt loop
- tool execution
- permission rules
- question handling
- multi-turn continuity
- message persistence and replay

It avoids unrelated TUI concerns:

- terminal rendering
- key handling
- focus and input state
- scrollback behavior
- stale visual state
- manual observation errors

## Standard Debug Flow

For agent/session bugs, start with the deterministic suite:

```sh
cargo test -p opencode-provider
cargo test -p opencode-session
```

When a provider credential is available, run the live smoke path:

```sh
DEEPSEEK_API_KEY=<key> scripts/qa/stream-smoke.sh
```

If the non-TUI path fails, debug the runtime before opening the TUI. If the non-TUI path passes and the TUI still fails, the likely bug is in the TUI/client layer, launch lifecycle, attach behavior, or rendering/input state.

## Relationship To `opencode run`

`CLI-002` improved `opencode run` by fixing the interim `AgentExecutor` path: tools are now attached, a bounded tool loop runs, and tool results are fed back to the provider.

That path is still not the canonical TUI/session runtime path. It does not become equivalent to TUI/session behavior until it is routed through the same `SessionPrompt` and server/session machinery as `/session/{id}/prompt`.

Use `opencode run` as a useful CLI smoke path, not as proof that TUI-backed sessions behave identically.

The canonical map of CLI/lifecycle behavior and the owning `CLI-*` cards is `wiki/cli-surface.md`; use it to see what exists today versus the target `opencode task` surface.

## Relationship To CLI-001

`CLI-001` should make the canonical non-TUI runtime path ergonomic. Its future `opencode task new`, `opencode task send`, and `opencode task view` commands should submit and inspect work through the server/session prompt path rather than through `AgentExecutor`.

If `CLI-001` is implemented on top of the same session/server prompt runtime as the TUI, it can replace most manual TUI testing for agent-session work. In that case, a CLI task smoke should exercise the same agent result that the TUI would show.

The equivalence only holds if CLI-001 uses the same runtime pieces:

- session model
- `/session/{id}/prompt` behavior or equivalent server call
- `SessionPrompt`
- agent context resolution
- provider/model selection
- tool resolution
- permission and question handling
- message storage and status updates

If CLI-001 uses `AgentExecutor`, it is useful but not TUI/session-equivalent.

## What Non-TUI QA Can Replace

When routed through the canonical session/server prompt path, non-TUI QA can replace most manual TUI testing for:

- multi-turn agent behavior
- tool calling
- provider streaming
- prompt/session loop regressions
- session message correctness
- response completeness
- raw filepath task workflows

## What Non-TUI QA Cannot Replace

Non-TUI QA cannot prove TUI-specific behavior:

- rendering correctness
- keyboard shortcuts
- command palette behavior
- interactive permission/question UI presentation
- scrolling and transcript display
- attach screen behavior
- TUI lifecycle or detach UX

## Practical Rule

For agent/session bugs, test non-TUI first. Use the TUI only after the runtime is proven, or when the suspected bug is specifically in the TUI/client layer.

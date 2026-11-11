---
id: "BUG-068"
title: "run --attach is non-agentic and cannot create a session (404)"
priority: "P1"
type: "bug"
area: "BUG"
spec: "invariants/coding-session-behavior.md"
status: "qa"
created: "2026-11-10"
updated: "2026-11-11"
---

# run --attach is non-agentic and cannot create a session (404)

## Summary

`opencode-modded-rust run --attach <url>` is advertised as the non-interactive
attach path, but it does not run the agentic loop and cannot create a session
against a stock `serve` instance.

## Evidence

1. Session creation uses a trailing-slash endpoint:

   `crates/opencode-cli/src/main.rs:1706`

   ```rust
   let create_endpoint = server_url(base_url, "/session/");
   ```

   The server routes `GET/POST /session` without the trailing slash
   (`crates/opencode-server/src/routes.rs:104`), so the request returns
   `404 Not Found`. Observed against a local `serve` on `127.0.0.1`.

2. With a manually created session, `run --attach` posts to
   `/session/{id}/stream` (`crates/opencode-cli/src/main.rs:1894`). That route
   builds a raw `ChatRequest` with `system: None` and `tools: None`
   (`crates/opencode-server/src/routes.rs:2247-2256`), so it is a chat stream,
   not the agentic loop.

   Observed result: only `message_start`, `usage` (completion=4096), and
   `message_end`; no tool calls, no final content. A T03 task that requires
   running tests produced no usable report.

## Why this is a product defect

`--attach` is the documented remote/non-interactive path, and `run` is expected
to behave like the local agentic run. It currently cannot reach the agent/tool
loop. This also makes the CLI unusable as a token/usage probe for real runs.

## Expected

- Create sessions at `/session` (fix the trailing slash) or make the server
  tolerate it.
- Route `run --attach` through the canonical queued prompt path
  (`POST /session/{id}/prompt`) and consume the session event stream/messages.
- Do not use `/session/{id}/stream` for attach; it is a raw chat stream with
  `tools: None` and `system: None`.
- Add an integration test that proves a tool-requiring prompt executes at least
  one tool and returns a report.

## Workaround

`qwen30-eval` uses `POST /session` then `POST /session/{id}/prompt` and reads
`/session/{id}/message`; see `scripts/run-one-attach.sh` in
`~/apps/qwen30-eval`.

## Related

- `BUG-069` — `stats` reports zero usage for server-driven sessions.
- `BUG-062` — Qwen/Hermes inline tool-call normalization.
## Implementation 2026-11-11

Branch: `bug/BUG-068-attach-agentic`. Not merged; owner opens and merges the PR.

Changes:
- Session creation now posts to `/session` (no trailing slash); the
  `--continue` listing also uses `/session?roots=...`.
- `run --attach` posts `{message, model, variant}` to `POST /session/{id}/prompt`,
  which runs the same queued agentic loop (tools, system prompt, agent rules) as
  the TUI server path.
- It polls `/session/status` until the session is idle, then reads
  `/session/{id}/message` and prints tool activity plus the assistant report.
  Client wait is bounded by `OPENCODE_RUN_TIMEOUT_MS` (default 30 min); on
  timeout it aborts the remote prompt. `/session/{id}/stream` is no longer used.
- `RemoteMessageInfo`/`RemoteMessagePart` now read `completed_at`, `error`, and
  `tool_call` so the client can surface tool calls and remote failures.

Verification:
- `cargo fmt --all`
- `SCOPEMUX_PREBUILT_DIR=... cargo test -p opencode-cli -p opencode-server`
  -> 22 + 76 + 3 passed, 0 failed.
- New integration test `routes::attach_tool_tests::prompt_path_executes_tool_and_returns_final_text`
  drives `session_prompt` with a scripted provider that emits a `read` tool
  call and asserts the tool call, its result content, and the final text.

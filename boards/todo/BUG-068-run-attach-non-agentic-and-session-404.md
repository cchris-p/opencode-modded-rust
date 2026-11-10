---
id: "BUG-068"
title: "run --attach is non-agentic and cannot create a session (404)"
priority: "P1"
type: "bug"
area: "BUG"
spec: "invariants/coding-session-behavior.md"
status: "todo"
created: "2026-11-10"
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

- Create sessions at `/session` (or make the server tolerate the trailing
  slash).
- Have `run --attach` execute the canonical queued prompt path
  (`POST /session/{id}/prompt`) or make `/stream` run the agentic loop with the
  session's tools/system prompt.
- Do not silently downgrade to a tool-less chat stream.

## Workaround

`qwen30-eval` uses `POST /session` then `POST /session/{id}/prompt` and reads
`/session/{id}/message`; see `scripts/run-one-attach.sh` in
`~/apps/qwen30-eval`.

## Related

- `BUG-069` — `stats` reports zero usage for server-driven sessions.
- `BUG-062` — Qwen/Hermes inline tool-call normalization.

---
id: "BUG-005"
title: "OpenAI-compatible chat providers reject requests once tools are attached (missing wire-format conversion)"
priority: "P1"
type: "bug"
area: "BUG"
spec: ""
status: "doing"
created: "2026-09-09"
---

# OpenAI-compatible chat providers reject requests once tools are attached (missing wire-format conversion)

## Summary

After `BUG-004` attached tool definitions to coding-session requests, every message sent to an OpenAI-compatible `/chat/completions` provider failed with `400 Bad Request: tools[0]: missing field 'type'`. This is a regression introduced by actually attaching tools to a request path that never had a wire-format converter.

## Reported behavior

- User QA of `BUG-004` on the merged build: "It keeps erroring every time I send a message."
- Reproduced against `deepseek/deepseek-v4-flash` on the live server: every prompt stored only a user message plus an assistant message containing `Provider error: API error: 400 Bad Request: ... tools[0]: missing field 'type'`.
- Reproduced regardless of message content (even `Reply with exactly OK`).

## Root cause

`BUG-004` makes `session_prompt` attach the resolved tool set to every coding-session request. The OpenAI-compatible chat/completions providers (`deepseek`, `openrouter`, `xai`, `together`, `perplexity`, `mistral`, `groq`, `cohere`, `cerebras`, `deepinfra`, and the `openai.rs`/`azure.rs` legacy chat bodies) serialize the internal, provider-neutral `ChatRequest` directly:

- `ChatRequest.tools` is `Vec<ToolDefinition>` whose serde shape is `{ name, description, parameters }` — but OpenAI-compatible chat requires `{ "type": "function", "function": { name, description, parameters } }`.
- Internal messages carry tool calls/results as Anthropic-style `Content::Parts` (`tool_use`/`tool_result`), which OpenAI chat does not accept. Once a model emits a tool call, the follow-up request would also fail (`assistant message with 'tool_calls' must be followed by tool messages`).

The Responses-API path already had a dedicated converter (`crates/opencode-provider/src/responses_convert.rs`); the chat-completions path had none because tools were never attached there before `BUG-004`.

## Fix (this branch)

`bug/BUG-005-openai-chat-tool-wire-format`

- Added `crates/opencode-provider/src/openai_chat.rs`:
  - `openai_chat_tools` / `openai_chat_tool` wrap each `ToolDefinition` as `{ "type": "function", "function": {...} }`.
  - `convert_messages` rewrites internal messages into OpenAI chat wire messages: text stays text; assistant `tool_use` parts become `tool_calls: [{id, type:"function", function:{name, arguments}}]`; `tool_result` parts (on assistant or `Role::Tool` messages) become separate `{ role: "tool", tool_call_id, content }` messages; reasoning parts are dropped.
  - A message containing only `tool_result` parts emits only `role: "tool"` messages (no empty assistant shell), preserving required assistant→tool ordering.
  - `openai_chat_completions_body` builds the full body from a `ChatRequest`.
- Routed the OpenAI-compatible chat providers through the converter: `deepseek`, `openrouter`, `xai`, `together`, `perplexity`, `mistral`, `groq`, `cohere`, `cerebras`, `deepinfra`, and the `openai.rs`/`azure.rs` chat request bodies.
- Added unit tests for tool wrapping, assistant tool_calls + tool messages, `Role::Tool` handling, separate-assistant tool-result ordering, and plain-text passthrough.

## Verification

- `cargo test -p opencode-provider` (86 lib + 7 integration) and `cargo test -p opencode-server --lib` (11) pass.
- Live against `deepseek/deepseek-v4-flash` on a fresh server:
  - `Reply with exactly OK and nothing else.` → assistant `OK` (no error).
  - `Look at the files in this workspace and summarize them.` → model issues `ls` tool calls that execute against the working directory.

## Still open / follow-ups (not this branch)

- **deepseek reasoning pass-back**: a tool-loop follow-up on deepseek emitted `400 The reasoning_content in the thinking mode must be passed back to the API` when the model produced reasoning tokens and then a tool call. Needs reasoning-content preservation/round-trip for reasoning models.
- **Split tool-call delta on deepseek**: the session stored two tool-call parts for one logical call (`call_00_...` name `ls` + a stray `tool-call-0` with empty name carrying the arguments), suggesting a tool-call streaming-delta aggregation issue. Verify whether it is a provider parser or session aggregation problem.

## Related Items

- `BUG-004` Coding sessions run as bare chat (this is a regression of attaching tools)
- `FEAT-010` Anthropic provider tool transport parity

## Notes

- This is a provider-boundary wire-format gap, not a session-loop issue; `BUG-004` simply exposed it by making tools reach the wire for the first time.
- The typed OpenAI-compatible providers (`vercel`, `github_copilot`, `gitlab`) drop tool parts in their own converters and are out of scope here.

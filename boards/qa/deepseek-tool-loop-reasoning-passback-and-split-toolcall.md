---
id: "BUG-006"
title: "DeepSeek tool loop fails: reasoning_content must be passed back and a split tool-call delta produces a stray empty tool call"
priority: "P1"
type: "bug"
area: "BUG"
spec: ""
status: "qa"
created: "2026-09-09"
---

# DeepSeek tool loop fails: reasoning_content must be passed back and a split tool-call delta produces a stray empty tool call

## Summary

Tool transport now works (BUG-005 fixed), but a deepseek tool loop still cannot complete. In a real session (`ses_3c567f00846647feaa47e52d0b8e8dac`, exported `reply-with-exactly-ok.md`):

1. The model issues tool calls; they execute (`ls` returns a directory listing).
2. The follow-up request then fails with `400 The reasoning_content in the thinking mode must be passed back to the API`.
3. The stream also produced a stray empty-name tool call (`Tool: ` with `name: ""` and the `path` argument attached), which fails as `Tool '' not found in registry`.

These block the actual "look at the files and summarize" workflow that BUG-004/BUG-005 were meant to enable.

## Evidence

Stored messages for the QA session:

- assistant message contains two `toolCall` parts:
  - `tool-call-0`, `name: ""`, `input: { "path": "/home/matrillo/repos/opencode-modded-rust" }`
  - `call_00_...`, `name: "ls"`, `input: {}`
- the `ls` result is returned; the empty-name call fails (`Tool '' not found`).
- the following assistant message is `Provider error: 400 ... reasoning_content in the thinking mode must be passed back to the API`.

## Root-cause hypotheses (to confirm during investigation)

1. **reasoning_content is dropped.** OpenAI-compatible chat streaming models (deepseek thinking mode) send `reasoning_content` deltas. The request-side converter (`opencode_provider::openai_chat`) drops reasoning parts entirely. DeepSeek requires the prior assistant `reasoning_content` to be echoed on the follow-up request, otherwise it 400s. Reference/AI SDKs pass reasoning back in the assistant message.
2. **Split tool-call delta.** The stream parser or session tool-call assembly split one logical tool call (`ls` with a `path` argument) into two parts: the name (`ls`, empty input) and a separately-id'd call (`tool-call-0` with empty name carrying the argument). This is likely an index-vs-id mismatch in the OpenAI-compatible SSE tool-call handling, or the session `ToolCallEnd` aggregation keying on a name-less delta.

## Scope

- Preserve reasoning content end-to-end on the OpenAI-compatible chat path so reasoning models accept the follow-up request: capture `reasoning_content` from the stream, store it as reasoning parts, and re-emit it on the assistant message for the next request (matching what the model/API requires).
- Fix tool-call assembly so one streamed tool call maps to one `toolCall` part with its real name and full arguments (no empty-name stray part).
- Verify the full loop on deepseek: "look at the files and summarize them" completes with tool calls and a final textual answer.

## Non-goals

- General provider parity beyond the OpenAI-compatible chat path.
- Anthropic tool transport (tracked by FEAT-010).

## Done when

- A deepseek session can issue tool calls, execute them, feed results back, and produce a final answer without a 400 on the follow-up request.
- No empty-name stray tool call appears in the stored/exported transcript.

## Recommended verification

- `cargo test -p opencode-provider`
- Live: fresh server on `development`, session prompt "Look at the files in this workspace and summarize them" against `deepseek/deepseek-v4-flash`; confirm tool calls execute and a final summary is returned (no 400, no `Tool '' not found`).

## Related Items

- `BUG-005` OpenAI-compatible chat providers reject requests once tools are attached (fixed; this is the follow-up tool-loop bug)
- `BUG-004` Coding sessions run as bare chat
- `FEAT-010` Anthropic provider tool transport parity

## Notes

- Reproduced and recorded from the user QA session export; root-cause hypotheses need confirmation before implementing.

## Implementation - 2026-09-09

### Root cause (confirmed)

The deepseek-family providers parsed OpenAI-compatible SSE with the **stateless** `parse_openai_sse`/`sse_event_stream(...openai_compat_line_events)`:

1. **Split tool call**: a tool call's `id`/`name` arrive in one delta; argument-only fragments arrive in later deltas carrying only `{index}`. The stateless parser minted a fresh `tool-call-{index}` id for those fragments, so the session recorded two tool calls — the real one (empty input) plus a stray empty-name `tool-call-0` (with the arguments) → `Tool '' not found in registry`.
2. **reasoning dropped**: `reasoning_content`/`reasoning_text` deltas were ignored, so a thinking model's reasoning never entered the session. DeepSeek requires the prior assistant `reasoning_content` to be echoed on follow-up requests → `400 reasoning_content ... must be passed back to the API`.

### What changed

- `crates/opencode-provider/src/stream.rs`:
  - Added `OpenAiCompatParserState` (per-index tool-call id/name, reasoning-open flag) and `parse_openai_sse_stateful`, modeled on `OpenAIProvider::parse_legacy_sse_data`: retains tool-call id/name across fragments so argument-only deltas join the real call, and surfaces reasoning deltas as `ReasoningStart/Delta/End`.
  - Added `openai_compat_sse_stream(chunks)` (buffered, stateful) so providers route raw bytes through the stateful parser.
  - Unit tests: split-tool-call join, reasoning capture/close, multi-frame + trailing flush.
- Routed the openai-compatible chat providers through `openai_compat_sse_stream`: `deepseek`, `openrouter`, `xai`, `together`, `perplexity`, `mistral`, `groq`, `cohere`, `cerebras`, `deepinfra`.
- `crates/opencode-session/src/prompt.rs` `parts_to_content`: keep `PartType::Reasoning` as a `reasoning` content part so reasoning survives into provider messages.
- `crates/opencode-provider/src/openai_chat.rs`: assistant `reasoning` parts are echoed as the message's `reasoning_content` field instead of dropped (deepseek wire requirement); assistant shell emitted when reasoning present.
- `crates/opencode-provider/src/anthropic.rs`: skip `reasoning` content parts (Anthropic handles thinking via its own block, not echoed as text).

### Verification

- `cargo test -p opencode-provider` (90 lib + 7 integration), `opencode-session` (144), `opencode-server --lib` (11) pass.
- Live on `deepseek/deepseek-v4-flash` (fresh server): "Look at the files in this workspace and summarize them" now reasons, issues well-formed `ls`/`read`/`bash` tool calls that execute, and returns a complete summary — **no 400 and no empty-name stray tool call**.

### PR Link

- https://github.com/cchris-p/opencode-modded-rust/pull/29 (merged into `development` as `103b778`)

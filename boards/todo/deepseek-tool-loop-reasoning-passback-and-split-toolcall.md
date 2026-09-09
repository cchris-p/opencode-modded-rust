---
id: "BUG-006"
title: "DeepSeek tool loop fails: reasoning_content must be passed back and a split tool-call delta produces a stray empty tool call"
priority: "P1"
type: "bug"
area: "BUG"
spec: ""
status: "todo"
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

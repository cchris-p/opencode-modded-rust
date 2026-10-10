---
id: "BUG-060"
title: "Novita AI DeepSeek R1 leaks native tool-call tokens in content; tools are not executed"
priority: "P1"
type: "bug"
area: "BUG"
spec: "invariants/coding-session-behavior.md"
status: "done"
created: "2026-10-08"
---

# Novita AI DeepSeek R1 leaks native tool-call tokens in content; tools are not executed

## Summary

On the `odn` profile (`novita-ai/deepseek/deepseek-r1-0528`), a turn that should call a tool often
returns the tool call as DeepSeek's **native special-token text** inside the assistant `content`
instead of an OpenAI `tool_calls` array. The runtime treats that content as ordinary text, so no tool
runs: no `tool_call`/`tool_result` part, no side effect, and the run ends with `finish_reason: stop`.
This is the "DeepSeek novita ai conversation / tool calls" failure seen in transcripts and QA.

The model is flaky, not always broken: with the identical request it sometimes returns a proper
`tool_calls` array and sometimes leaks the native tokens. Either way the product must not silently
drop a tool call.

## Observed behavior

Raw assistant text captured from a real runtime turn:

```
...Here's the tool call:<｜tool▁calls▁begin｜><｜tool▁call▁begin｜>function<｜tool▁sep｜>bash
```json
{"command": "echo captured", "description": "Run echo command to output 'captured'"}
```<｜tool▁call▁end｜><｜tool▁calls▁end｜>
```

with `finish_reason: "stop"` and no `tool_calls` field.

## Evidence (2026-10-08)

Environment: built binary `target/debug/opencode`, commit `f30920f`; headless server on
`127.0.0.1`.

1. **Live runtime, `novita-ai/deepseek/deepseek-r1-0528`** - a prompt forcing the `bash` tool did not
   execute it across several runs: sometimes the assistant text carried the native tokens above,
   sometimes a `tool_call` part was recorded with no `tool_result`. A marker-file side effect
   (`touch EXECUTED.marker`) was never created.
2. **Control, `deepseek/deepseek-flash`** - the same prompt executed: `tool_call` + `tool_result`
   parts present and the marker file created. Harness and permissions are not the cause.
3. **Direct endpoint probe** - `POST https://api.novita.ai/openai/chat/completions` with the
   runtime's exact captured request (via a local logging proxy) replayed 5x: **2/5 returned a proper
   `tool_calls` array, 3/5 leaked the native tokens in `content`**. So the endpoint sometimes honors
   OpenAI tools and sometimes does not - it is model-side flakiness, not a missing-`tools` bug.
4. The runtime **does** send tools: the captured request had `stream: true`, 19 tool definitions
   (including `bash`), and `[system, user]` messages. Catalog marks the model `tool_call: true`.

## Root cause (confirmed)

`novita-ai` routes through the generic `OpenAIProvider` legacy SSE parser
(`crates/opencode-provider/src/openai.rs`, `parse_legacy_sse_data`), which emits every `content`
delta as a plain `TextDelta` (`openai.rs:376-386`). There is no handling for DeepSeek's native
tool-call token format (`<｜tool▁calls▁begin｜>...<｜tool▁calls▁end｜>`), so when the model emits a call
that way the tokens land in the assistant text and no tool call is produced. The stateful parser
(`crates/opencode-provider/src/stream.rs`, `parse_openai_sse_stateful`) has the same gap.

## Reproduction

- `QA_MODEL=novita-ai/deepseek/deepseek-r1-0528`; prompt the server to run a marker-file `bash`
  command with the tool; observe no `tool_result` and no marker file (or the raw token text).
- Or replay the captured request body from the local proxy 5x and observe the native tokens.

## Required solution

- Detect DeepSeek native tool-call tokens in streamed assistant `content` and convert them into
  proper tool-call stream events (`ToolCallStart`/`ToolCallEnd`), stripping the raw tokens from the
  assistant text. Handle the tokens split across chunk/delta boundaries.
- Apply to both OpenAI-compatible parsers: the generic legacy parser (`openai.rs`, novita path) and
  the stateful parser (`stream.rs`, deepseek-direct path), via one shared extractor.
- Cover with offline fixtures/tests using the captured native-token bytes; extend the `QA-002`
  harness rather than adding a new one.
- Verify live on the `odn` model that the tool executes end to end.

## Scope

- `crates/opencode-provider/src/stream.rs` (shared extractor + stateful parser).
- `crates/opencode-provider/src/openai.rs` (legacy parser).
- Provider tests (`crates/opencode-provider`), and the `QA-002` fixture harness.

## Non-goals

- Retrying or re-modeling the turn; the fix is a deterministic content-to-tool-call normalization.
- Changing the model capability/catalog metadata (that is `FEAT-013` territory).
- v1/v2 prompt-loop consolidation (`FEAT-011`).

## Done when

- A novita `deepseek-r1-0528` turn that leaks native tool tokens executes the tool and feeds the
  result back; the raw tokens never appear in the stored/exported transcript.
- Offline parser tests cover a native-token stream (including chunk-split boundaries) and assert the
  parsed events are `ToolCallStart`/`ToolCallEnd` with the correct name and JSON input.
- `cargo test -p opencode-provider` passes plus `cargo fmt --all -- --check`.
- Live marker-file check on `odn` passes.

## Verification

- Offline: parser tests against captured native-token fixtures.
- Live: headless server on `novita-ai/deepseek/deepseek-r1-0528`, prompt a marker-file `bash` call,
  assert `tool_call` + `tool_result` parts and the marker file exists.

## Related Items

- `EPIC-001` DeepSeek default-model reliability hardening (parent; tool-call/request-protocol class).
- `BUG-006` DeepSeek tool loop fails (done) - the OpenAI-style split tool-call / reasoning fix; this
  is the same class on a different provider and wire signal.
- `QA-002` Deterministic DeepSeek stream/loop fixture harness (qa) - the regression net to extend.
- `FEAT-067` Add Novita AI to provider settings and auth surfaces (qa) - the surfaces work; this is
  the follow-up runtime defect found during its QA.
- `FEAT-010` Anthropic provider tool transport parity (todo) - sibling provider-transport gap.
- `FEAT-013` Model capability gating and deprecated-model surfacing (todo).

## Notes

- `FEAT-067` recorded this failure inline during its QA; that finding is superseded by this card.
- Control proved the test path works (`deepseek-flash` executes the same prompt), so this is not a
  harness or permission limitation.

## Implementation - 2026-10-08

Added `DeepSeekNativeToolCallExtractor` in `crates/opencode-provider/src/stream.rs`: it detects the
DeepSeek native calls block (`<｜tool▁calls▁begin｜>...<｜tool▁calls▁end｜>`) in streamed `content`,
emits `ToolCallStart`/`ToolCallEnd` with the parsed name and JSON arguments, strips the raw tokens
from the visible text, and buffers across chunk/delta boundaries (holding back a partial token
suffix). Wired into both OpenAI-compatible parsers - the generic legacy parser `openai.rs`
(`parse_legacy_sse_data`, the `novita-ai` path) and the stateful parser `stream.rs`
(`parse_openai_sse_stateful`, deepseek-direct) - and flushed on `[DONE]`/finish.

## Verification - 2026-10-08

- Offline: `cargo test -p opencode-provider` - 114 lib (new extractor tests: char-by-char split,
  multiple calls, fenceless args, unterminated flush) + integration pass; new fixture
  `tests/fixtures/deepseek/native_toolcall_tokens.sse` replayed via `openai_compat_sse_stream` incl.
  a char-boundary split matrix; legacy-parser tests in `openai.rs` (single + split frames).
- `cargo test -p opencode-session` - 170 + 2 + 11 pass. `cargo fmt --all -- --check` clean.
- Live: headless server on `novita-ai/deepseek/deepseek-r1-0528`, 5 sequential turns each required to
  `touch` a marker file - **5/5 executed** (marker created), no raw tokens in text/reasoning.
- Repro confidence: replaying the runtime's exact captured request against Novita leaked native
  tokens in content 3/5 times pre-fix; the fix turns those into executed calls.

## PR Link

- https://github.com/cchris-p/opencode-modded-rust/pull/138 (base `development`)

## QA Report - 2026-10-10 (user, live `odn` TUI)

Post-merge QA on `development` (`a3c0925`) in a real `ort` TUI session
(`ses_f1f1aa182d5e4ebf85833fafd7ecdec5`, `~/apps/c-cpp-projects`,
`novita-ai/deepseek/deepseek-r1-0528`, server port 3187).

- Inspected the live session via `GET /session/<id>/message`: **6 `tool_call` parts and 6
  `tool_result` parts** - `ls` (x3), `glob`, `write` - all executed.
- The `write` call returned `Successfully wrote 660 bytes (23 lines) to
  .../cpp-banking-system/learning-roadmap.md`; the file is present on disk.
- **No** native token markers (`tool▁calls▁begin`) in any text/reasoning part.
- User: "It did manage to run one tool successfully ... Drastic improvement."

Result: PASS. Closed `done`.

---
id: "FEAT-071"
title: "Verify Qwen/Hermes tool-call coverage for 2507 and neutralize the kill-switch name"
priority: "P2"
type: "feature"
area: "FEAT"
spec: "invariants/providers.md"
status: "done"
created: "2026-11-11"
updated: "2026-11-11"
---

# Verify Qwen/Hermes tool-call coverage for 2507 and neutralize the kill-switch name

## Summary

The provider already normalizes Qwen/Hermes inline `<tool_call>` markup in the
shared OpenAI-compatible parsers (BUG-062). Tighten coverage for the 2507 form
and stop naming the shared kill switch after DeepSeek only.

## Scope

1. Add or confirm a provider fixture for the 2507 plain-JSON form:
   `<tool_call>{"name": "...", "arguments": {...}}</tool_call>`, split across
   SSE chunks, in both the content and reasoning channels.
2. Add `OPENCODE_DISABLE_NATIVE_TOOLCALLS` as the neutral kill-switch name.
   Keep `OPENCODE_DISABLE_DEEPSEEK_NATIVE_TOOLCALLS` working as a compatibility
   alias, with the same truthy semantics.
3. Update `wiki/deepseek-native-tool-call-normalization.md` and any provider
   docs to describe the shared switch.
4. Confirm the server prompt path (after BUG-068) consumes the same parser; do
   not add a second parser in the server or session layer.

## Non-goals

- No model-id gating.
- No change to native `tool_calls` behavior.
- No new per-provider parser.

## Done when

- Provider tests cover the 2507 plain-JSON form and both env names.
- `cargo test -p opencode-provider` passes.
- The documentation names the shared switch and the Qwen signal.
## Implementation 2026-11-11

Branch: `feat/FEAT-071-hermes-2507-coverage`. Not merged; owner opens and merges the PR.

Changes:
- `OPENCODE_DISABLE_NATIVE_TOOLCALLS` is the neutral kill switch. It is resolved
  before `OPENCODE_DISABLE_DEEPSEEK_NATIVE_TOOLCALLS`, which stays a
  compatibility alias with identical truthy semantics (`1/true/on/yes`).
- New tests cover the 2507 plain-JSON `<tool_call>{"name":...,"arguments":{...}}</tool_call>`
  form split across SSE frames in both the `content` and `reasoning` channels,
  plus truthy/neutral-vs-legacy env resolution.
- `wiki/deepseek-native-tool-call-normalization.md` and
  `invariants/providers.md` now name the shared switch and the Qwen/Hermes
  signal, and state that the server prompt turn consumes the shared parser.
- Path verification after BUG-068: the server prompt turn (`session_prompt` ->
  `run_prompt_turn` -> `SessionPrompt`) uses the generic provider, so it
  consumes the same OpenAI-compatible parser; no second parser was added in the
  server or session layer.

Verification:
- `cargo fmt --all`
- `cargo test -p opencode-provider` -> 152 + 9 + 4 + 2 + 7 passed, 0 failed
  (including `qwen_2507_plain_json_{content,reasoning}_split_across_chunks` and
  `neutral_and_legacy_kill_switches_share_truthy_semantics`).

## Self-QA 2026-11-11 (rebased branch)

Branch `feat/FEAT-071-hermes-2507-coverage` (`0d70c64`), binary built from the
branch. Drove the real server prompt path with a mock provider streaming the
2507 plain-JSON `<tool_call>{"name":"read","arguments":{...}}</tool_call>` form
split across chunks.

- Normalization on: `read` tool executed, fixture content appeared in the tool
  result, final assistant report returned, raw markup not visible.
- `OPENCODE_DISABLE_NATIVE_TOOLCALLS=1`: no tool call, raw markup passed through
  as text, server healthy.
Result: **PASS**.
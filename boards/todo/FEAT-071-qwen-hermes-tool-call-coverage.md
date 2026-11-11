---
id: "FEAT-071"
title: "Verify Qwen/Hermes tool-call coverage for 2507 and neutralize the kill-switch name"
priority: "P2"
type: "feature"
area: "FEAT"
spec: "invariants/providers.md"
status: "todo"
created: "2026-11-11"
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

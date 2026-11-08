---
id: "FEAT-010"
title: "Anthropic provider tool transport parity"
priority: "P2"
type: "feature"
area: "FEAT"
spec: "invariants/coding-session-behavior.md"
status: "todo"
created: "2026-09-09"
---

# Anthropic provider tool transport parity

## Summary

The Rust Anthropic provider cannot carry tools to the model at all. `AnthropicRequest` (`crates/opencode-provider/src/anthropic.rs:228-239`) has no `tools` field, and `convert_request` (`anthropic.rs:99-144`) never reads `ChatRequest.tools`. Tools that `BUG-004` attaches on the session path will be silently dropped for any Anthropic model.

## Why this exists

`BUG-004` makes the coding-session prompt path attach an agent system prompt, environment block, and permission-filtered tool set. The session layer cannot rely on every provider honoring tools by default: `OpenAIProvider` serializes what it is given, but Anthropic structurally cannot receive tools today. This is a provider-boundary gap, not a session-layer gap.

## Scope

- Add a `tools` field to `AnthropicRequest` with reference-compatible Anthropic tool schema serialization.
- Convert `ChatRequest.tools` (`ToolDefinition` → Anthropic `tool` objects) in `convert_request`.
- Ensure `Role::System` messages (used to carry the system prompt) still lift into the Anthropic `system` parameter as they do today, and do not collide with the request `system` field.
- Preserve tool-call and tool-result round-trips through the Anthropic stream parser so declared tools execute and results return to the session.
- Verify against the live Anthropic path used in `BUG-003` QA (`anthropic/claude-haiku-4-5` or equivalent configured model).

## Non-goals

- Rewriting the Anthropic transport or auth.
- Provider breadth beyond the daily-driver path tracked here.

## Done when

- A session on an Anthropic model receives the declared tool set and the model can issue tool calls that execute and return results.
- The system prompt still reaches Anthropic as its `system` parameter.

## Recommended verification

- `cargo check -p opencode-provider`
- `cargo test -p opencode-provider anthropic`
- Live: `./target/debug/opencode run -m anthropic/<model> "List the files in this workspace"` and confirm tool calls execute.

## Related Items

- `PHASE-002` (phase parent)
- `BUG-004` Coding sessions run as bare chat (foundation this follows)
- `BUG-003` Session stops completely after first prompt
- `START-019` Add native Ollama support for the local-model-first V1 path

## Notes

- Coordinate with `BUG-004`; implement only after the session path can actually attach tools.

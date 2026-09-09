---
id: "FEAT-011"
title: "Consolidate v1 and v2 session prompt loops"
priority: "P3"
type: "feature"
area: "FEAT"
spec: "invariants/coding-session-behavior.md"
status: "todo"
created: "2026-09-09"
---

# Consolidate v1 and v2 session prompt loops

## Summary

The Rust session layer contains two competing model-loop implementations:

- **v1** `SessionPrompt::loop_inner` (`crates/opencode-session/src/prompt.rs:955-1404`) — the live product path used by `session_prompt` on the server. Tool execution, ask/question callbacks, title/summary generation, and reminders are wired here.
- **v2** `LlmProcessor` / `StreamProcessor` (`crates/opencode-session/src/llm.rs`) with `StreamInput` — more parity-complete request assembly (agent tool resolution at `llm.rs:840`, system prompt construction, tool choice, caching) but currently only reachable via compaction (`compaction.rs:511-527`), never for normal prompts.

## Why this exists

`BUG-004` chose to extend the v1 `loop_inner` path rather than migrate the live product to v2, to keep the parity foundation surgical. This item records the resulting debt: two loops with different capabilities makes behavior drift likely and duplicate-permission/agent logic hard to keep consistent.

## Scope

- Decide the single source-of-truth loop (recommended: retire the v1 `loop_inner` in favor of the v2 `StreamProcessor`, or fold v2's request-assembly into v1 — resolution happens during implementation with an evidence note).
- Port the capabilities the surviving loop lacks (tool execution wiring, ask/question callbacks, title/summary hooks, snapshot patches, permission evaluation, resume).
- Remove the dead loop and its now-unused helpers/tests.
- Keep behavior identical on the live TUI/server path; verify with the `BUG-003`/`QA-001` regression suites.

## Non-goals

- New agentic behavior beyond what each loop already offers.
- Rewriting compaction.

## Done when

- Exactly one session model-loop implementation is used for normal prompts, compaction, and (where applicable) subtasks.
- The live path passes `QA-001` (SSE integrity + multi-turn session-loop regressions) and the `BUG-004` verification script.

## Recommended verification

- `cargo test -p opencode-session`
- Live `ort-build`/`ort` multi-turn session per `BUG-003`.

## Related Items

- `BUG-004` Coding sessions run as bare chat (defers this consolidation)
- `QA-001` Repeatable debug/QA verification suite
- `BUG-003` Session stops completely after first prompt

## Notes

- This is intentionally deferred until `BUG-004` lands so the live path is exercised with correct agent/tool/permission wiring before restructuring.

---
id: "BUG-004"
title: "Coding sessions run as bare chat: no agent prompt, environment context, or tools attached to model requests"
priority: "P1"
type: "bug"
area: "BUG"
spec: "invariants/coding-session-behavior.md"
status: "todo"
created: "2026-09-09"
---

# Coding sessions run as bare chat: no agent prompt, environment context, or tools attached to model requests

## Summary

In the current Rust product, a normal coding-session request reaches the model with no resolved agent, no agent system prompt, no workspace/environment block, and no tool set. The model therefore behaves like a plain chat completion: it cannot see the local workspace and asks the user to upload files. This is the parity gap that makes the daily-driver agent unusable for any "look at the files / work on the workspace" workflow.

## Reported behavior

- Asking the agent to review the project or look at the current workspace produces a plain-chat refusal ("I don't have the ability to browse your local file system…") instead of using read/glob/grep/bash tools.
- Reproduced in a stored session (`ses_ae81decdf5024e638e67f2dc1d7aac04`, exported as `project-review-request.md`): all messages are plain text with zero tool calls and no environment context.
- Evidence-backed audit: `wiki/coding-session-parity-audit.md`.

## Why this exists

The Rust product targets a serious daily-driver coding workflow. A coding session that cannot observe or act on the workspace is not a coding session. The reference implementation always resolves the default `build` agent, attaches the model-appropriate system prompt plus the environment block, and attaches the agent's permission-filtered tool set to every request. The Rust port built all the pieces but only wired them into the CLI path — which itself still omits tools — and left the TUI/server prompt path as a bare chat request.

## Root cause chain (file:line evidence)

1. TUI sends only `message/agent/model/variant`: `crates/opencode-tui/src/api.rs:504-532`.
2. `session_prompt` invokes the prompt loop with `system_prompt = None`, `tools = Vec::new()`, `AgentParams::default()`: `crates/opencode-server/src/routes.rs:1863-1893`.
3. `loop_inner` hard-codes `ChatRequest.system = None` and sends `tools: None` when the merged set is empty: `crates/opencode-session/src/prompt.rs:1068-1090`.
4. `SystemPrompt::for_model/environment/instructions` are only called from the CLI crate, never the server/session path: `crates/opencode-cli/src/main.rs:1598-1606, 5199-5207` vs `crates/opencode-session/src/system.rs`.
5. `ServerState` holds no `ToolRegistry`/`AgentRegistry`: `crates/opencode-server/src/server.rs:112-119`. The 26-tool default registry (`crates/opencode-tool/src/registry.rs:222-253`) is never surfaced to the model; the only production registry→definition conversion is the subtask fallback `execute_inline` (`prompt.rs:2830-2865`).
6. CLI/`AgentExecutor` attaches the system prompt as a conversation message but never attaches tools: `ChatRequest::new` defaults `tools`/`system` to `None` (`crates/opencode-agent/src/executor.rs:164, 226, 274`; `crates/opencode-provider/src/message.rs:212-226`).
7. Providers do not compensate (OpenAI emits only what it is given; Anthropic never reads `request.tools`): `crates/opencode-provider/src/openai.rs:591-640`, `crates/opencode-provider/src/anthropic.rs:99-144`.
8. Secondary: configured default model `opencode/trinity-large-preview-free` is deprecated and filtered from the provider registry at bootstrap: `crates/opencode-provider/src/bootstrap.rs:1956-1959, 3191-3200`.

## Scope

- Make the TUI/server session prompt path resolve the active agent (default `build`), assemble the model-appropriate system prompt plus environment block, and attach the agent's permission-filtered tool set to every agentic request.
- Make the CLI/`AgentExecutor` path attach tools to its requests (system prompt is already present there).
- Attach the resolved tool set through the provider boundary for all providers used on the daily-driver path.
- Persist/restore the resolved agent, model, and tool context on the session so resume reconstructs the same agentic behavior.
- Surface explicit model-capability gating when a model cannot call tools, instead of silently degrading to chat.
- Keep parity documentation in this repo per `invariants/coding-session-behavior.md` and `wiki/coding-session-parity-audit.md`.

## Non-goals

- Broad OpenCode parity beyond agentic coding-session behavior (deferred by `START-008`).
- Rebuilding the agent registry or the whole prompt-template system; reuse `SystemPrompt::for_model/environment` and `create_default_registry()`.
- Provider-setup/auth work tracked elsewhere (`START-009`, `START-015`, `START-019`, `START-027`).

## Done when

- A fresh session prompt on the Rust product can answer "look at the files / review this workspace" by issuing read/glob/grep/bash tool calls against the working directory.
- The model request demonstrably contains the agent system prompt, the environment/workspace block, and the resolved tool set on both the TUI/server path and the CLI path.
- Session resume reconstructs the same agentic context (agent, model, tools) without losing tools/system.
- Provider paths used for the daily-driver workflow deliver declared tools and return tool results into the session.
- A model that cannot call tools produces an explicit, visible outcome rather than a silent plain-chat session.

## Recommended verification

- `cargo check -p opencode-server -p opencode-session -p opencode-cli -p opencode-agent -p opencode-tui`
- `cargo test -p opencode-session -p opencode-server`
- Launch via `ort-build`/`ort`, open a session, and prompt with "Look at the files in this workspace and summarize them"; confirm the agent issues tool calls rather than asking for uploads.
- Reproduce the stored-session symptom path (export transcript) and confirm tool parts appear in the exported transcript.

## Related Items

- `BUG-003` Session stops completely after first prompt
- `FEAT-007` Advanced coding-session polling
- `START-008` Full parity deferred
- `QA-001` Repeatable debug/QA verification suite

## Notes

- Refer to `wiki/coding-session-parity-audit.md` for the full evidence chain and the reference comparison.
- Do not start this item by re-implementing prompts; first wire the existing `SystemPrompt` and tool-registry resolution into the session prompt loop.

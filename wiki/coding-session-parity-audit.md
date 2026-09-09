# Coding-Session Parity Audit

This document audits the Rust coding-session agentic behavior against the reference OpenCode implementation (frozen line: `$HOME/repos/opencode-modded` at commit `e62912b5d18b73316c7bfd6e894b040698f6c880`). It is the evidence-backed source for `BUG-004` and the normative rules captured in `invariants/coding-session-behavior.md`.

## Purpose

- capture what the current Rust coding-session path actually sends to the model
- identify the parity gaps that make a coding session behave like plain chat instead of an agent
- give `BUG-004` an evidence-backed implementation target

## Observed symptom

A real session (`project-review-request.md`, session `ses_ae81decdf5024e638e67f2dc1d7aac04`, stored in `~/.local/share/opencode/opencode.db` under the Rust plural schema) shows the assistant repeatedly replying that it cannot see the local workspace and asking the user to upload files. All six stored messages are plain text: **no tool calls and no environment/workspace context ever reached the model**. The model behaved correctly for what it was given — it was given nothing to act on.

## Current Rust behavior (confirmed in this repo)

- The TUI sends a chat message to `POST /session/{id}/prompt` with a body of only `message`, `agent`, `model`, `variant` (`crates/opencode-tui/src/api.rs:504-532`).
- `session_prompt` (`crates/opencode-server/src/routes.rs:1654-1946`) invokes the session prompt loop with `system_prompt = None`, `tools = Vec::new()`, and `AgentParams::default()` (`routes.rs:1863-1893`).
- `ServerState` holds no `ToolRegistry` and no `AgentRegistry` (`crates/opencode-server/src/server.rs:112-119`); nothing on the server surfaces the default tool set or the selected agent to the prompt loop.
- Inside `loop_inner`, the `ChatRequest` is built with `system: None` hard-coded and `tools: None` whenever the merged tool set is empty (`crates/opencode-session/src/prompt.rs:1068-1090`).
- `SystemPrompt::for_model` / `SystemPrompt::environment` / `SystemPrompt::instructions` (`crates/opencode-session/src/system.rs`) are only called from the CLI crate (`crates/opencode-cli/src/main.rs:1598-1606, 5199-5207`), never from the server/session prompt path.
- `AgentParams::default()` is empty (`prompt.rs:128-133`); a fresh `Session` stores no agent/system/tools and carries no field for them (`crates/opencode-session/src/session.rs:284-316`).
- The default tool registry exists — `create_default_registry()` registers 26 tools (`crates/opencode-tool/src/registry.rs:222-253`) — but nothing converts it to `ToolDefinition`s and attaches it on the main prompt path. The only registry→definition conversion that runs in production is the subtask fallback `execute_inline` (`prompt.rs:2830-2865`), not the main loop.
- The CLI/`AgentExecutor` path attaches the system prompt as a conversation system message but also never attaches tools: `ChatRequest::new(...)` is used with `tools`/`system` defaulting to `None` (`crates/opencode-agent/src/executor.rs:164, 226, 274`; `crates/opencode-provider/src/message.rs:212-226`).
- Providers do not compensate: OpenAI emits only what it is given (`crates/opencode-provider/src/openai.rs:591-640`), and the Anthropic converter does not even read `request.tools` (`crates/opencode-provider/src/anthropic.rs:99-144`).
- Secondary factor: the repo default model `opencode/trinity-large-preview-free` (`opencode.jsonc`) is marked deprecated in the catalog, and deprecated models are filtered out of the runtime provider registry during bootstrap (`crates/opencode-provider/src/bootstrap.rs:1956-1959, 3191-3200`).

## Reference behavior (confirmed from the frozen line)

- The reference default agent is `build` (`agent/agent.ts:141-155`; `defaultInfo()` picks `config.default_agent`).
- Each user message records the resolved agent; each request-loop iteration re-resolves that agent (`prompt.ts:656-670, 1170`).
- Per request the loop resolves the tool set via `SessionTools.resolve` over the agent/model/permission (`prompt.ts:1226-1241`; `session/tools.ts:41-134`), so every allowed tool is attached as a callable AI tool.
- Per request the system array is assembled from the environment block, instructions, MCP instructions, and skills (`prompt.ts:1257-1286`; `session/system.ts:67-103`).
- The final request always carries the agent prompt (or model default), the full system array, and the resolved, permission-filtered tool set (`session/llm/request.ts:56-66, 184, 208-214`).

## Parity gaps

1. No agent resolution on the server path

- The `agent` string from the TUI (`"build"` by default) is stored as session metadata but never resolved into an agent definition (`routes.rs:1715-1719`). The prompt loop uses it only for message-reminder insertion (`prompt.rs:1063-1066`).

2. No system prompt in the server/session path

- `session_prompt` passes `system_prompt = None` and `loop_inner` hard-codes `ChatRequest.system = None`, so the "You are opencode…" agent prompt and the environment block never reach the model in the TUI/server flow.

3. No environment/workspace context

- `SystemPrompt::environment` (working directory, workspace root, git state, platform, date) is never called on the server path; the model is not told where it is or that a workspace exists.

4. No tool attachment on the main prompt path

- `session_prompt` passes `tools = Vec::new()`; the merged set stays empty; `ChatRequest.tools` becomes `None`. The 26-tool default registry is never declared to the model in a normal session. The CLI `AgentExecutor` path has the same gap.

5. No persistence of agent/tool/system context

- `Session` has no agent/system/tools fields; resume cannot reconstruct the agentic context that vanilla records per user message.

6. Model gating is not explicit

- The configured default model is deprecated and filtered from the registry, which can leave a user with no usable agent path; no visible gate explains the resulting behavior.

## Fix target

Close gaps 1-4 so that every coding-session request resolves the default `build` agent, attaches the model-appropriate system prompt plus the environment block, and attaches the agent's permission-filtered tool set — and close gap 5 so session resume reconstructs the same context. Gap 6 should be surfaced explicitly rather than silently degrading to chat. See `BUG-004`.

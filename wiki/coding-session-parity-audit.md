# Coding-Session Parity Audit

This document audits the Rust coding-session agentic behavior against the reference OpenCode implementation (frozen line: `$HOME/repos/opencode-modded` at commit `e62912b5d18b73316c7bfd6e894b040698f6c880`). It is the evidence-backed source for `BUG-004` and the normative rules captured in `invariants/coding-session-behavior.md`.

## Purpose

- capture what the current Rust coding-session path actually sends to the model
- identify the parity gaps that make a coding session behave like plain chat instead of an agent
- give `BUG-004` an evidence-backed implementation target
- record the follow-up gaps that fall outside `BUG-004`'s foundation scope

## Observed symptom

A real session (`project-review-request.md`, session `ses_ae81decdf5024e638e67f2dc1d7aac04`, stored in `~/.local/share/opencode/opencode.db` under the Rust plural schema) shows the assistant repeatedly replying that it cannot see the local workspace and asking the user to upload files. All six stored messages are plain text: **no tool calls and no environment/workspace context ever reached the model**. The model behaved correctly for what it was given — it was given nothing to act on.

## Live product path

The default `opencode` command launches the TUI (`crates/opencode-cli/src/main.rs:769-787`). The TUI talks HTTP to a (possibly detached) server and sends prompts via `POST /session/{id}/prompt`:

- TUI request body: only `message`, `agent`, `model`, `variant` (`crates/opencode-tui/src/api.rs:504-532`, struct at `api.rs:141-149`). Default agent string is `"build"` (`crates/opencode-tui/src/context/app_context.rs:138`).
- Server handler `session_prompt` (`crates/opencode-server/src/routes.rs:1654-1946`) invokes `SessionPrompt::prompt_with_update_hook` with `system_prompt = None`, `tools = Vec::new()`, `AgentParams::default()`, and no MCP/LSP clients (`routes.rs:1863-1893`).
- `ServerState` holds no `ToolRegistry` and no `AgentRegistry` (`crates/opencode-server/src/server.rs:112-119`).
- The live loop is v1 `SessionPrompt::loop_inner` (`crates/opencode-session/src/prompt.rs:955-1404`). It builds `ChatRequest` with `system: None` hard-coded and `tools: None` whenever the merged tool set is empty (`prompt.rs:1068-1090`).
- `SystemPrompt::for_model` / `environment` / `instructions` (`crates/opencode-session/src/system.rs`) are only called from the CLI crate (`crates/opencode-cli/src/main.rs:1598-1606, 5199-5207`), never from the server/session prompt path.
- A separate v2 loop (`crates/opencode-session/src/llm.rs::StreamProcessor` / `LlmProcessor`, request assembly at `llm.rs:808-950`, agent tool resolution at `llm.rs:840`) is more parity-complete but is only reachable via compaction (`compaction.rs:511-527`); it is dead for normal prompts.

## Tool execution already works (declaration is the gap)

Tool execution on the live loop is functional: after a model turn with tool calls, `loop_inner` builds `create_default_registry()` and runs `execute_tool_calls`, wiring ask/question callbacks and continuing the loop (`prompt.rs:1335-1379`, `execute_tool_calls` at `prompt.rs:1514+`). The primary defect is on the **declaration side**: tools are never attached to `ChatRequest`, and no system/environment prompt is sent.

Two secondary defects would block usability even after declaration is fixed:

1. **Ask-storm hazard.** Server-path tools call `ctx.ask_permission(...)` unconditionally on every invocation (`read.rs:119-124`, `write.rs:105-111`, `edit/tool.rs:142-148`, `grep_tool.rs:119-125`, `glob_tool.rs:89-91`, `ls.rs:113-119`, `todo.rs:65-70,150-151`, `webfetch.rs:89-94`, `skill.rs:375-376`, `task.rs:95-102`, `codesearch.rs:99-100`). The server ask callback (`routes.rs:1747-1804`) always round-trips to the TUI and never evaluates `AgentInfo.permission`, `Session.permission` (`session.rs:303`, settable via `routes.rs:594-614`), or `PermissionRequest.always` (`tool.rs:213-255`). Reference behavior evaluates the merged ruleset and only prompts when the decision is `Ask`; Rust never evaluates on the server path. `ToolContext.agent` is populated with an empty string on this path (`prompt.rs:1347`).
   - The CLI/`AgentExecutor` path is the inverse: it evaluates rules via `PermissionNext::evaluate` (`agent.rs:111-125`, `444-446`) in `ensure_tool_allowed` (`executor.rs:514-526`) but hard-denies `Ask` with no ask UI, and its `execute()` loop has no production caller — the CLI drives `execute_streaming` (`executor.rs:266-284`), a single tool-less stream pass.
2. **Default-agent inconsistency.** `AgentRegistry::default_agent()` returns `general` (`agent.rs:668-685`), while reference and the TUI default to `build`. Tool-capability hints exist on `ModelInfo` (`supports_tools`/`tool_call` in provider model tables) but no request-construction site consults them.

## Current Rust behavior (confirmed, grouped)

### Server/TUI session path (`session_prompt` → `loop_inner`)

- No agent resolution: the `agent` string is stored as metadata only (`routes.rs:1715-1719`); `loop_inner` uses it only for reminder insertion (`prompt.rs:1063-1066`).
- No system prompt: `system_prompt = None` passed in; `ChatRequest.system` hard-coded `None`; `SystemPrompt::*` never called (`prompt.rs:1080`).
- No environment/workspace context block reaches the model.
- No tools: `tools = Vec::new()` → merged set empty → `ChatRequest.tools = None` (`prompt.rs:1081-1085`). The default registry (26 tools, `registry.rs:222-253`) is created only at execution time, not declared.
- Tool execution exists and loops correctly; permission decisions are not applied (ask fires for everything).
- Resume: `resume_session` (`prompt.rs:873-953`) exists but is dead code (no HTTP route calls it); it does propagate `system_prompt`/`tools` into `loop_inner`, so it is a prepared seam.

### CLI path (`opencode run` / interactive / `github run` → `AgentExecutor`)

- System prompt attached as a `Role::System` conversation message: `Conversation::with_system_prompt` (`message.rs:92-96`) → `to_provider_messages` (`message.rs:121-131`); built in `main.rs:1589-1607, 5189-5208`.
- Tools never attached: `ChatRequest::new(...)` used at `executor.rs:164, 226, 274` with `tools: None` (`message.rs:212-226`).
- `execute_streaming` (the CLI's actual entrypoint) runs a single stream pass and never executes tools; `execute()`'s tool loop + permission gate has no production caller other than the subagent callback.

### Providers

- No auto-injection: OpenAI emits only what it is given (`openai.rs:591-640` serializes `tools`; legacy `chat_stream_legacy` uses `build_request_body` which serializes `ChatRequest`, `openai.rs:507-532, 892-976`).
- Anthropic structurally cannot receive tools: `AnthropicRequest` has no tools field and `convert_request` never reads `request.tools` (`anthropic.rs:99-144, 228-239`). It lifts `Role::System` messages into the `system` parameter but ignores `request.system`/`request.tools`.
- Secondary: repo default model `opencode/trinity-large-preview-free` (`opencode.jsonc`) is deprecated in the catalog and filtered from the runtime provider registry at bootstrap (`bootstrap.rs:1956-1959, 3191-3200`).

## Reference behavior (confirmed from the frozen line)

- Default agent is `build` (`agent/agent.ts:141-155`; `defaultInfo()` honors `config.default_agent`).
- Each user message records the resolved agent; each request-loop iteration re-resolves that agent (`prompt.ts:656-670, 1170`).
- Per request the loop resolves tools via `SessionTools.resolve` over agent/model/permission (`prompt.ts:1226-1241`; `session/tools.ts:41-134`), so every allowed tool is attached as a callable AI tool.
- Per request the system array is assembled from the environment block, instructions, MCP instructions, and skills (`prompt.ts:1257-1286`; `session/system.ts:67-103`). The env block carries model id, working directory, workspace root, git state, platform, and date.
- The final request always carries the agent prompt (or model default), the full system array, and the resolved, permission-filtered tool set (`session/llm/request.ts:56-66, 148, 184, 208-214`).
- Permission decisions are evaluated at tool execution against the merged agent + session ruleset; only `Ask` decisions prompt the user.

## Parity gaps

1. No agent resolution / default mismatch on the server path (`build` vs `general`).
2. No system prompt in the server/session path (`SystemPrompt::*` never called there).
3. No environment/workspace context in the server/session path.
4. No tool attachment on the main prompt path (registry never converted to declared `ToolDefinition`s on either path).
5. No permission evaluation at server-path tool execution (ask fires unconditionally; `Session.permission` unused).
6. No persistence of agent/tool/system context for resume.
7. Tool-attachment is not gated on model tool capability; default model can silently drop out of the registry (deprecated/filtered).

## Follow-up work (outside BUG-004 foundation)

- `FEAT-010` Anthropic provider tool transport (`AnthropicRequest.tools` field + conversion).
- `FEAT-011` v1 `loop_inner` / v2 `StreamProcessor` consolidation.
- `FEAT-012` CLI/`AgentExecutor` tool-loop parity (attach tools + run execution loop + ask path).
- `FEAT-013` Model capability gating and deprecated-default-model surfacing.

## Fix target

`BUG-004` closes gaps 1-6 on the live v1 server/TUI path: resolve the default `build` agent (honoring `config.default_agent`), attach the model-appropriate system prompt plus the environment block, attach the agent's permission-filtered tool set, gate on model capability, evaluate permission decisions at tool execution, and persist/restore agentic context on resume. See `BUG-004`.

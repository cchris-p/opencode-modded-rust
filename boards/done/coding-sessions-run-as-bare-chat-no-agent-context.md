---
id: "BUG-004"
title: "Coding sessions run as bare chat: no agent prompt, environment context, or tools attached to model requests"
priority: "P1"
type: "bug"
area: "BUG"
spec: "invariants/coding-session-behavior.md"
status: "done"
created: "2026-09-09"
---

# Coding sessions run as bare chat: no agent prompt, environment context, or tools attached to model requests

## Summary

In the current Rust product, a normal coding-session request reaches the model with no resolved agent, no agent system prompt, no workspace/environment block, and no tool set. The model therefore behaves like a plain chat completion: it cannot see the local workspace and asks the user to upload files. This is the parity gap that makes the daily-driver agent unusable for any "look at the files / work on the workspace" workflow.

This card is scoped as a **parity-first foundation**: one coherent fix that makes the live TUI/server session path behave like the reference coding agent (default `build` agent identity, full system array, permission-filtered tool set, and permission evaluation at tool execution), so future parity work builds on a correct agentic request path rather than drifting.

## Reported behavior

- Asking the agent to review the project or look at the current workspace produces a plain-chat refusal ("I don't have the ability to browse your local file system…") instead of using read/glob/grep/bash tools.
- Reproduced in a stored session (`ses_ae81decdf5024e638e67f2dc1d7aac04`, exported as `project-review-request.md`): all messages are plain text with zero tool calls and no environment context.
- Evidence-backed audit: `wiki/coding-session-parity-audit.md`.

## Why this exists

The Rust product targets a serious daily-driver coding workflow. A coding session that cannot observe or act on the workspace is not a coding session. The reference implementation always resolves the default `build` agent, attaches the model-appropriate system prompt plus the environment block, and attaches the agent's permission-filtered tool set to every request.

The Rust port built all the pieces but wired them only into the CLI path — which itself still omits tools — and left the TUI/server prompt path as a bare chat request.

## Root cause chain (file:line evidence)

The live product session path is the TUI → server route → `SessionPrompt` v1 loop:

1. TUI sends only `message/agent/model/variant`: `crates/opencode-tui/src/api.rs:504-532`.
2. `session_prompt` invokes the prompt loop with `system_prompt = None`, `tools = Vec::new()`, `AgentParams::default()`: `crates/opencode-server/src/routes.rs:1863-1893`.
3. `loop_inner` hard-codes `ChatRequest.system = None` and sends `tools: None` when the merged set is empty: `crates/opencode-session/src/prompt.rs:1068-1090`.
4. `SystemPrompt::for_model/environment/instructions` are only called from the CLI crate, never the server/session path: `crates/opencode-cli/src/main.rs:1598-1606, 5199-5207` vs `crates/opencode-session/src/system.rs`.
5. `ServerState` holds no `ToolRegistry`/`AgentRegistry`: `crates/opencode-server/src/server.rs:112-119`. The 26-tool default registry (`crates/opencode-tool/src/registry.rs:222-253`) is never surfaced to the model on the main prompt path; the only production registry→definition conversion is the subtask fallback `execute_inline` (`prompt.rs:2830-2865`).
6. Secondary: configured default model `opencode/trinity-large-preview-free` is deprecated and filtered from the provider registry at bootstrap: `crates/opencode-provider/src/bootstrap.rs:1956-1959, 3191-3200`.

### Important correction (scope refinement)

Tool **execution already works** on the live loop: `loop_inner` builds a default registry and runs `execute_tool_calls` with ask/question callbacks wired (`prompt.rs:1335-1379`). The gap is the **declaration side** plus **permission evaluation**:

- **Ask-storm hazard.** Server-path tools call `ctx.ask_permission(...)` unconditionally (`read.rs:119-124`, `write.rs:105-111`, `edit/tool.rs:142-148`, grep/glob/ls/todo/webfetch/skill…). The server ask callback (`routes.rs:1747-1804`) always round-trips to the TUI and never evaluates `AgentInfo.permission`, `Session.permission`, or `PermissionRequest.always`. Attaching tools without wiring permission evaluation would make every read/grep/edit prompt the user — still unusable. Reference behavior evaluates the merged agent+session ruleset and only prompts when the decision is `Ask`.
- **Dual loop implementations.** The live product path is v1 `loop_inner` (`prompt.rs`). A more parity-complete v2 exists (`llm.rs::StreamProcessor`/`StreamInput`, with agent-tool resolution at `llm.rs:840`) but is only reachable via compaction. Consolidation is a follow-up, not this card.
- **Provider gap.** `anthropic.rs` `AnthropicRequest` has no tools field at all (`anthropic.rs:228-239`); tools are structurally impossible there. Tracking a follow-up.
- **Default-agent inconsistency.** `AgentRegistry::default_agent()` returns `general` (`agent.rs:668-685`), while reference and the TUI default to `build`.

## Architecture decision

- **Extend the v1 `loop_inner` path** (`opencode-session::SessionPrompt`) as the parity foundation. Do not migrate the live product to the v2 `llm.rs` processor in this card; record that consolidation separately (see Related Items).
- Resolution lives in the session layer and is reusable by fresh prompts and resume; callers no longer pass `None`/`Vec::new()`.

## Scope (this card)

- Add a per-request resolution step (fresh prompt and resume) that, given the active agent and model, produces:
  - the full system array in reference order: `agent.prompt ?? SystemPrompt::for_model(model)` + environment block (`SystemPrompt::environment`: model id, working directory, workspace root, git, platform, date) + instructions + skills + MCP instructions, reusing existing `system.rs` builders (`crates/opencode-session/src/system.rs`);
  - the resolved `AgentParams` (temperature/top_p/max_tokens) from the agent definition;
  - the permission-filtered `ToolDefinition` set built from `create_default_registry()` schemas, gated by the agent's permission rules and model tool capability.
- Resolve the default agent to `build` when the request or session does not name one (fix `AgentRegistry::default_agent()` inconsistency so it honors `config.default_agent` then `build`).
- Thread the resolved system array, tools, and agent params through `session_prompt` → `loop_inner` (`prompt.rs`) and `resume_session` (`prompt.rs:873-953`), consuming the previously dead `PromptInput.system`/`PromptInput.tools` (`prompt.rs:40,43`) or equivalent explicit args.
- Wire permission evaluation into server-path tool execution: evaluate the tool request's permission+patterns against the merged agent + `Session.permission` ruleset using `opencode-permission` (`ruleset.rs` `evaluate`/helpers). `Allow` → run silently, `Ask` → TUI callback, `Deny` → `PermissionDenied` surfaced to the model as an error tool result. Respect `PermissionRequest.always`.
- Populate tool `ToolContext.agent` with the resolved agent name on the server path (currently empty string: `prompt.rs:1347`).
- Persist the resolved agent/model on session user messages so resume re-resolves identically (supports `invariants/coding-session-behavior.md`).
- Verification on the live path (OpenAI-compatible / `opencode` provider) that "look at the files" issues read/glob/grep/bash tool calls without prompting for every invocation.

## Non-goals (explicit follow-ups, see Related Items)

- Anthropic (and other structural) provider tool transport: `AnthropicRequest` needs a tools field + conversion.
- v2 `llm.rs`/`StreamProcessor` ↔ v1 `loop_inner` consolidation.
- CLI/`AgentExecutor` (`opencode run`) tool-loop parity (attach tools + run an execution loop + ask UI). System prompt is present there; tools and execution are not.
- Explicit model-capability/deprecated-default-model surfacing beyond gating tool attachment.
- Broad OpenCode parity (deferred by `START-008`).

## Done when

- A fresh session prompt on the Rust product answers "look at the files / review this workspace" by issuing read/glob/grep/bash tool calls against the working directory.
- The outbound request demonstrably contains: agent identity (`build` default), the model/agent system prompt, the environment/workspace block, and the resolved, permission-filtered tool set.
- Default agent resolution honors `config.default_agent` and falls back to `build`.
- Tool execution on the server path applies permission decisions: `Allow` runs without prompting, `Ask` prompts once, `Deny` returns an error the model can see. Reading project files does not prompt on every call.
- Session resume reconstructs the same agentic context (agent + tools + system) instead of dropping to bare chat.
- A model that cannot call tools does not get a tool set; the session behavior remains explicit rather than silently chat-like.

## Recommended verification

- `cargo check -p opencode-server -p opencode-session -p opencode-tool -p opencode-agent`
- `cargo test -p opencode-session -p opencode-server -p opencode-permission`
- Launch via `ort-build`/`ort`, open a session, and prompt with "Look at the files in this workspace and summarize them"; confirm the agent issues tool calls rather than asking for uploads, and that ordinary reads do not prompt for permission.
- Export a transcript and confirm tool parts appear in it.

## Implementation - 2026-09-09

### What changed

Foundation slice shipped on `bug/BUG-004-agentic-session-foundation`:

- Added `opencode-server::agentic` (`crates/opencode-server/src/agentic.rs`) with:
  - `resolve_agentic_context` — resolves the agent (requested → `config.default_agent` → `build`), builds the system prompt (agent prompt or `SystemPrompt::for_model` + environment/workspace block), resolves the permission-filtered tool set from the default registry (excluding `invalid` and `Deny` tools), and derives `AgentParams`.
  - `classify_permission` / `merged_ruleset` / `ruleset_from_session` — permission decision helpers mirroring the reference ruleset evaluation.
- Wired `session_prompt` (`crates/opencode-server/src/routes.rs`) to resolve the agentic context and pass the resolved system prompt, tool set, and agent params into the prompt loop instead of `None`/`Vec::new()`/defaults. Session metadata now always records the resolved agent.
- Permission-aware ask callback: tool-execution asks are evaluated against the merged agent + session ruleset; `Allow` runs silently, `Deny` errors, only `Ask` round-trips to the TUI.

### Tests

- `cargo test -p opencode-server --lib agentic` (6 tests): default-agent resolution, system-prompt/env assembly, agent-prompt override, tool filtering, permission allow/deny classification, session-overlay merge.
- `cargo test -p opencode-server --lib` (11) and `cargo test -p opencode-session --lib` (144) pass.
- `cargo check -p opencode-cli -p opencode-tui` passes.

### Still open (this card)

- Verify the live TUI path via `ort-build`/`ort` on a real provider: "Look at the files in this workspace" should issue read/glob/grep/bash without prompting for every call.
- Confirm the ask callback doesn't regress `START-018` approval UX for `Ask`/`Deny` cases.

### PR Link

- https://github.com/cchris-p/opencode-modded-rust/pull/26 (branch `bug/BUG-004-agentic-session-foundation`, base `development`)

## QA Report - 2026-09-09 (user)

- User QA on `development` after BUG-005 merged:
  - "Reply with exactly OK" → returns `OK` on deepseek (agentic system prompt + tools attached and accepted; no bare-chat confusion).
  - "Look at the files in this workspace and summarize them" → the agent now issues real `ls` tool calls that execute against the working directory (tool parts appear in the transcript).
- Residual (tool-loop only, tracked by `BUG-006`): the follow-up request after a tool result fails on deepseek with `400 reasoning_content ... must be passed back to the API`, and a split tool-call delta produces a stray empty-name tool call.
- Note: an earlier QA attempt on this card was invalid because a stale detached server reused a pre-fix binary; cleared and re-verified on a fresh server.

## Merge status (BUG-004 / PR #26)

- Merged into `development` on 2026-09-09 via PR #26 (merge commit `1ca3ed9`).
- PR branch `bug/BUG-004-agentic-session-foundation` deleted.
- Item relaned to `qa`; awaiting user verification of the combined BUG-004 + FEAT-014 behavior on `development`.
- QA note: the first QA attempt was invalid because a stale detached server (old binary, `/proc/<pid>/exe` deleted) was reused by `ort`. Stale servers were killed and the reuse record cleared; FEAT-014 now prevents stale-server reuse going forward.

## QA Report - 2026-09-10 (user, session `ses_0ffc9b44f96944039f8621c4d980d11b`)

- User confirmed agentic coding sessions work on `development`: "A lot better, it is calling tools."
- Session `summarize-workspace-files.md` shows the `build` agent prompt + environment + tools attached on every request, the model issuing `ls`/`read`/`bash` tool calls, consuming results, and producing a final summary.
- The original bare-chat symptom is resolved: the agent no longer claims it cannot see the workspace.
- Closing as completed. Residual quality defects surfaced during this QA are tracked separately: `BUG-007` (`ls` partial listings), `BUG-008` (`batch` unusable on the session path), `BUG-009` (instruction re-injection).

## Related Items

- `BUG-003` Session stops completely after first prompt
- `FEAT-007` Advanced coding-session polling
- `START-008` Full parity deferred
- `QA-001` Repeatable debug/QA verification suite
- `START-018` Complete TUI approval and question handling (coordinate permission UX)
- `FEAT-010` Anthropic provider tool transport parity
- `FEAT-011` Consolidate v1/v2 session prompt loops
- `FEAT-012` CLI/AgentExecutor tool-loop parity
- `FEAT-013` Model capability gating and deprecated-default-model surfacing

## Notes

- Refer to `wiki/coding-session-parity-audit.md` for the full evidence chain and reference comparison.
- Do not start by re-implementing prompts; wire the existing `SystemPrompt`, `create_default_registry()`, and `opencode-permission` ruleset helpers into the session prompt path.
- Start only after the earlier BUG-004 evidence/invariants commit is synced to the branch.

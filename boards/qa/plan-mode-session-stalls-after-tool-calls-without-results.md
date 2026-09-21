---
id: "BUG-016"
title: "Plan-mode session stalls after tool calls without tool results"
priority: "P1"
type: "bug"
area: "BUG"
spec: "invariants/coding-session-behavior.md"
status: "qa"
created: "2026-09-19"
---

# Plan-mode session stalls after tool calls without tool results

## Summary

A Rust `ort` session in plan mode accepted a simple request, emitted read/search tool calls, and then stopped without recording any tool results or final answer in the exported transcript.

The model should be able to use non-edit tools in plan mode. Plan mode is intended to disallow edits, not to strand read/search/list tool calls.

## Reported behavior

Transcript: [`new-session-2026-09-19t0409592215950000.md`](../../docs/transcripts/new-session-2026-09-19t0409592215950000.md)

Observed transcript state:

- Session ID in export: `ses_9b85fa20680b4dbfa7d2a2507335a4c1`.
- User prompt: `Give me all the skills board items`.
- Assistant mode/model header: `Plan · deepseek/deepseek-v4-flash`.
- Assistant thinking correctly identified that it should inspect the codebase.
- Assistant emitted three tool calls:
  - `ls` with `path: /Users/cchrisleepyles/repos/opencode-modded-rust`
  - `grep` for `skills[-_ ]?board`
  - `grep` for `skillsboard|SkillsBoard|skills_board`
- The transcript ends immediately after the tool-call inputs, with no tool results and no final response.

The request was answerable from the current board state. At the time of investigation, the relevant skills cards were:

- `SKILLS-001` `Align skills with current and reference OpenCode behavior`
- `SKILLS-002` `Plan URL-backed skills parity`

## Investigation - 2026-09-19

Current code evidence:

- `crates/opencode-agent/src/agent.rs` defines plan mode as `Plan mode. Disallows all edit tools.`
- `crates/opencode-permission/src/ruleset.rs` builds plan rules by allowing `question` and `plan_exit`, denying `edit`, and inheriting default allow rules for other tools.
- `crates/opencode-agent/src/executor.rs` and `crates/opencode-server/src/agentic.rs` attach permission-filtered tools by excluding only tools whose agent decision is `Deny`; plan-mode `ls` and `grep` should therefore be available.
- `crates/opencode-session/src/prompt.rs` finalizes streamed assistant messages with `ToolCall` parts and later calls `execute_tool_calls` to append a separate assistant message containing `ToolResult` parts.
- `crates/opencode-session/src/prompt.rs` has abort handling for cancelled pending tool calls, but a non-cancelled stall or execution-path error can still leave the visible/exported session at a tool-call-only state unless the loop records an explicit error/result before stopping.

Storage caveat (corrected 2026-09-21):

- An earlier draft of this item had the Rust and vanilla storage paths inverted. The Rust product DB
  on macOS is `~/Library/Application Support/opencode/opencode.db`
  (`dirs::data_local_dir()`, `crates/opencode-storage/src/database.rs:138-144`); the vanilla OpenCode
  DB is `~/.local/share/opencode/opencode.db`.
- On re-check, the exported session ID `ses_9b85fa20680b4dbfa7d2a2507335a4c1` **is present** in the
  Rust DB: one `sessions` row created `2026-09-19T04:09:59.221Z` with `updated_at == created_at`, and
  **zero `messages` rows**. The transcript therefore came from TUI in-memory state that was never
  persisted, not from a missing storage root.
- See `BUG-023` for the confidence-ranked root-cause investigation of this stall and the associated
  persistence gap.

Likely failure area to confirm:

- The live session prompt/tool loop can expose an assistant turn containing tool calls without ever appending the matching tool-result/error turn or surfacing a clear session error.
- This may be caused by a stall in session tool execution, a permission/ask path mismatch, an update/persistence gap after streaming tool calls, a cancellation path that is not detected, or a provider/session loop edge case specific to plan mode plus DeepSeek.

## Scope

- Reproduce or isolate the missing-tool-result state on the Rust session path used by `ort`.
- Confirm whether the failure occurs before tool execution starts, during tool execution, while appending/persisting tool results, while continuing the provider loop after results, or during transcript export.
- Make plan-mode read/search/list tool-call turns resolve deterministically.
- If a tool call cannot execute, append an explicit tool-result error part for that call or surface a durable session error instead of leaving the transcript at unresolved tool calls.
- Preserve plan-mode edit restrictions.

## Non-goals

- Changing plan mode into build mode or allowing edit tools in plan mode.
- Reworking the entire provider/tool transport stack beyond what is needed for unresolved session tool calls.
- Fixing board lane/frontmatter drift for existing `SKILLS-*` cards.
- Changing vanilla OpenCode storage or config behavior.

## Implementation notes for the next agent

Start with evidence, not a speculative fix:

1. Build a focused test or harness around the session prompt loop that simulates an assistant stream ending with tool calls and verifies matching tool-result/error parts are appended before the loop stops.
2. Include plan-mode agent/tool permissions in the reproduction path so `ls`, `glob`, `grep`, and `read` stay allowed while edit tools remain denied.
3. Inspect `SessionPrompt::execute_tool_calls` and the call site around `crates/opencode-session/src/prompt.rs` where `has_tool_calls` is detected and `continue` restarts the prompt loop.
4. Verify whether errors from `execute_tool_calls` are too easily swallowed: the current call site logs `Tool execution error` and continues state updates, but the user needs a durable visible result or error.
5. Check whether `ToolContext` in plan mode carries enough agent/session information for permission-gated tools and path validation.
6. Check transcript export only after the runtime state is known; the export may be faithfully showing an unresolved runtime state.

Potential regression target:

- A unit/integration test in `opencode-session` that feeds a mocked provider stream with multiple tool calls (`ls` plus `grep` is ideal) and asserts that after the prompt loop returns, each tool-call ID has exactly one corresponding `ToolResult` part or explicit error result.

## Done when

- A plan-mode session that emits read/search/list tool calls does not remain stuck at unresolved tool-call parts.
- Every assistant tool call in the affected session path receives a matching persisted tool-result/error part before the loop stops or the session exposes a clear durable error state.
- The original class of request, `Give me all the skills board items`, can complete in `ort` plan mode with a final answer after any needed read/search/list tools.
- Plan mode still denies edit tools.
- Transcript export of the reproduced case includes the tool results/errors and final answer, not only the tool-call inputs.

## Recommended verification

- `cargo test -p opencode-session` with a focused regression test for unresolved tool-call completion.
- `cargo test -p opencode-agent plan` or the nearest existing permission test after adding/adjusting plan-mode coverage.
- `cargo check -p opencode-session -p opencode-agent -p opencode-server -p opencode-tui`
- Live: run `ort-build`, then `ort` from `/Users/cchrisleepyles/repos/opencode-modded-rust`, switch/use plan mode, ask `Give me all the skills board items`, and confirm the session returns a final answer.
- Export the live session transcript and confirm it contains tool results/errors for every tool call plus the final answer.

## Related Items

- `BUG-003` Session stops completely after first prompt.
- `BUG-005` OpenAI-compatible chat providers reject requests once tools are attached.
- `BUG-006` DeepSeek tool loop reasoning passback and split toolcall.
- `BUG-008` batch tool is advertised but unusable on the session path.
- `BUG-012` Session summary runs before tool results and breaks OpenAI-compatible continuation.
- `FEAT-012` CLI/AgentExecutor tool-loop parity.
- `SKILLS-001` and `SKILLS-002` were the board items the original prompt was trying to list.

## Implementation Notes - 2026-09-19

- Added a prompt-loop repair guard after `execute_tool_calls` so any recorded `ToolCall` parts that still lack matching `ToolResult` parts receive durable error results before the loop continues to the provider.
- Refactored abort cleanup to use the same unresolved-tool-call detection so cancellation and non-cancellation failures both leave resolved transcript state.
- Added focused regression coverage for repairing unresolved `ls`/`grep` tool calls.
- Added plan-mode permission coverage showing edit/write tools remain disabled while `ls`, `grep`, and `read` remain available.

## Verification - 2026-09-19

- `cargo test -p opencode-session append_missing_tool_results_repairs_unresolved_calls` passed.
- `cargo test -p opencode-permission plan_rules_disable_edit_tools_but_keep_read_search_tools` passed.
- `cargo test -p opencode-permission` passed.
- `cargo check -p opencode-session -p opencode-agent -p opencode-server -p opencode-tui` passed.
- `cargo test -p opencode-session` ran with 148 passed and 2 failed in `instruction::tests::{test_find_up_stops_at_stop_dir,test_find_up_walks_parents}`; those failures are pre-existing and unrelated to BUG-016.

## PR Link

- https://github.com/cchris-p/opencode-modded-rust/pull/44

## QA / Merge Disposition

- Moved to `qa` on 2026-09-19 for continued observation.
- Per user request, PR #44 is intended to merge directly into `development` while the card remains in `qa` until the issue is observed again or considered stable.

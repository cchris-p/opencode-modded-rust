---
id: "BUG-065"
title: "Fallback system prompt drives Qwen (and default DeepSeek) to answer without tools and fabricate state"
priority: "P1"
type: "bug"
area: "BUG"
spec: "invariants/coding-session-behavior.md"
status: "todo"
created: "2026-10-12"
---

# Fallback system prompt drives Qwen (and default DeepSeek) to answer without tools and fabricate state

## Summary

On a live `ollama/qwen3:30b` session the agent loaded the `board-where-left-off` skill (the correct
first move), then made **zero tool calls** and emitted the fabricated placeholder answer `#123`. The
board was never read and the skill workflow was never executed. The turn is structurally valid — the
model chose to answer instead of act — so this is **not** a parser/transport defect like
`BUG-060`/`BUG-062`. The driver is the assembled system prompt.

## Evidence

- Exported transcript: `~/apps/tss-notes/lmk-where-we-left-off-with-this-board.md` (session
  `ses_ff6d4223c4644d2197194c7fbbe95b53`, model `ollama/qwen3:30b`, agent `Build`).
- Sequence: user prompt -> `skill(board-where-left-off)` -> skill body returned (a full workflow) ->
  59.2s of thinking, **no tool call** -> final answer `#123`.
- No `bd`, `read`, `glob`, or `bash` calls occurred anywhere in the session.
- The thinking quotes the system prompt back at itself as the source of the failure: "the examples in
  the prompt have answers like '4' or 'Yes', so maybe the correct answer here is the ID of the next
  step" and "Keep it under four lines".

## Root Cause

- `SystemPrompt::for_model` (`crates/opencode-session/src/system.rs:52`) falls back to `PROMPT_QWEN`
  (`crates/opencode-session/src/prompt_templates/qwen.txt`) for any model id that is not
  gpt/gemini/claude/trinity. Both `ollama/qwen3:30b` **and** the product default
  `deepseek/deepseek-flash` hit this fallback: the `build` agent has `system_prompt: None`
  (`crates/opencode-agent/src/agent.rs:201`), so `build_system_prompt`
  (`crates/opencode-server/src/agentic.rs:127`) uses the model default.
- `qwen.txt` mandates "**One word answers are best**" with toy examples (`4`, `Yes`, `ls`,
  `150000`), and contains no skill-execution guidance, no "investigate with tools before answering"
  guidance, and no anti-fabrication rule. It is `anthropic.txt` with the Task-Management and
  Professional-Objectivity sections removed.
- The available-skills block only says "load a skill when a task matches"
  (`crates/opencode-server/src/agentic.rs:153`), and the skill tool only says "inject the skill's
  instructions into the current conversation" (`crates/opencode-tool/src/skill.rs`). Nothing tells the
  model that loading a skill starts a workflow it must now execute **with tools**.
- The prompt loop correctly ends on a tool-call-free turn, so a "skill loaded, never acted on" turn
  terminates silently with no guard.

## Why this is a product defect

`invariants/coding-session-behavior.md` states: "An agentic coding session must behave like the
reference OpenCode coding agent, not like a plain chat completion." A turn that ignores attached tools
and a loaded skill, then fabricates state, violates that. Qwen's instruction-following is weak, but our
prompt makes the *wrong* behavior the compliant one; the same conflict is exercised by the default
DeepSeek model, which currently tolerates it better.

## Proposed Fix

1. **Rewrite the fallback prompt** (`crates/opencode-session/src/prompt_templates/qwen.txt`): add a
   grounding/no-fabrication rule and an explicit skill-execution rule; remove the "one word answers
   are best" toy examples. This benefits the default `deepseek-flash` model as well.
2. **Skill-tool output directive**: append a line after `<skill_content>` instructing the model to
   execute the workflow with tools rather than summarize the skill text.
3. **Optional loop guard**: if a skill was just loaded and the turn ends with no tool call, nudge a
   continuation.
4. Track the residual "model answers without calling tools" reliability gap under `EPIC-001`; this
   transcript is a clean deterministic fixture for `QA-002`.

## Scope

- In scope: the fallback system prompt, the assembled skills preamble, the skill-tool output framing,
  and an optional post-skill loop guard.
- Out of scope: transport/parser normalization (`BUG-060`/`BUG-061`/`BUG-062`), provider catalog
  changes, and the broader qwen effectiveness survey (`RESEARCH-007`).

## Non-goals

- Reworking provider architecture or consolidating the v1/v2 prompt loops.
- Changing model catalog metadata.

## Related Items

- `RESEARCH-007` Breadth survey: improving local qwen3:30b instantiation, reasoning, and
  effectiveness — the follow-up research scoping the qwen-specific levers
  (`docs/research/RESEARCH-007-qwen-effectiveness-breadth-survey.md`).
- `EPIC-001` DeepSeek default-model reliability hardening — owns the "model answers without tool
  calls" reliability class.
- `BUG-062` qwen3:30b Ollama tool-loop investigation — parser/encoding sibling; left this residual.
- `BUG-060`/`BUG-061` DeepSeek native tool-call normalization and guard — the transport-side fixes
  that this is *not*.
- `QA-002` Deterministic DeepSeek stream/loop fixture harness — regression net to extend.
- `invariants/coding-session-behavior.md`, `invariants/providers.md`.

## Acceptance Criteria

- The fallback prompt no longer instructs "one word answers are best" and carries an explicit
  act-with-tools + no-fabrication clause.
- The skill-tool output tells the model to execute the loaded workflow with tools.
- A deterministic check reproduces the transcript shape (skill loaded -> tool-call-free terminal
  answer) on the pre-fix prompt and shows the corrected behavior on the fixed prompt.
- `cargo test -p opencode-session` / `-p opencode-provider` pass; `cargo fmt --all` clean.

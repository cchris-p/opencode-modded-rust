---
id: "BUG-071"
title: "Session loop silently stops at MAX_STEPS=100 and requires a manual continue"
priority: "P1"
type: "bug"
area: "BUG"
spec: "invariants/coding-session-behavior.md"
status: "qa"
created: "2026-11-11"
updated: "2026-11-11"
---

# Session loop silently stops at MAX_STEPS=100 and requires a manual continue

## Summary

A long agentic turn ends after exactly 100 tool-call steps with no terminal message, no error, and
no visible reason. The last thing persisted is the final tool result, so the TUI looks like the
assistant "just stopped after tool calls", and the only way forward is for the user to send another
prompt ("continue"). Each new user prompt resets the step counter, so "continue" silently buys
another 100 steps.

The loop should either have no silent hard cap (matching vanilla) or, when a configured budget is
reached, end the turn with a visible terminal message instead of a silent `break`. The per-agent
`max_steps` configuration must also be honored; today it is ignored on the session loop path.

## Reported behavior

- A session runs a long tool-call sequence and then stops mid-task without an assistant summary.
- No error is shown; the final persisted message is a tool result.
- The user has to type "continue" to resume; the prompt resets and the same failure recurs at the
  next 100 steps.
- Reproduced in ORT session `ses_057e2a4e78b042c1a13950ee14ddae57` (the fenced-snippet session):
  runs 4 and 5 each ended at exactly 100 tool calls / 100 tool results, followed by a user
  "continue".

## Evidence

Session `ses_057e2a4e78b042c1a13950ee14ddae57` (ORT store, workspace
`/Users/cchrisleepyles/apps/opencode-modded-rust/boards`), 2026-11-11:

| Run | Window (UTC) | Tool calls | How it ended |
|---|---|---|---|
| 2 | 16:02:10–16:02:22 | 4 | `finish=aborted` (user interrupted by sending a prompt) |
| 3 | 16:02:25–16:32:25 | 10 | exactly 30 min → run budget (BUG-043 path) |
| 4 | 16:34:59–16:43:04 | **100** | silent stop → user "continue" |
| 5 | 16:54:58–17:02:01 | **100** | silent stop → user "continue" |

The tool-call/tool-result counts were computed directly from the ORT `messages` table. Runs 4 and 5
both terminating at exactly 100 is the hard step cap, not a provider or network stop.

## Root cause

The session prompt loop uses a hard-coded module constant and silently breaks:

- `crates/opencode-session/src/prompt.rs:32` — `const MAX_STEPS: u32 = 100;`
- `crates/opencode-session/src/prompt.rs:1298-1301` —
  `step += 1; if step > MAX_STEPS { tracing::warn!("Max steps reached for session {}", session_id); break; }`

The `break` returns `Ok(())`, so the server treats the run as a normal completion and
`finalize_run_without_terminal` (`crates/opencode-server/src/routes.rs:2957`) never fires. Only a
`tracing::warn!` is emitted; nothing is written to the transcript and nothing is shown in the TUI.

The per-agent limit is not wired into this path:

- `Cargo`-side `AgentInfo::build().max_steps = Some(100)`, `plan = Some(50)`, `general = None`
  (`crates/opencode-agent/src/agent.rs:205, 227, 256`) are ignored by `loop_inner`.
- `pub fn max_steps_for_agent(agent_steps: Option<u32>) -> u32` (`prompt.rs:3838`) is dead code.
- `AgentParams` (`prompt.rs:130`) has no `max_steps` field, and `agentic.rs:83` cannot pass it in.

Inconsistency: the other executor loop, `crates/opencode-agent/src/executor.rs:217`, returns
`AgentError::MaxStepsExceeded` (loud) when its `max_steps` is reached. The session loop does not.

## Reference behavior

**opencode vanilla** (branch `dev`, verified from source):

- `packages/core/src/session/runner/max-steps.ts` defines `MAX_STEPS_PROMPT`, a "MAXIMUM STEPS
  REACHED … respond with text only" wrap-up instruction.
- `packages/core/src/session/runner/llm.ts:261` computes
  `stepLimitReached = loaded.agent.info.steps !== undefined && step >= loaded.agent.info.steps`.
  If `steps` is undefined there is no cap.
- On reach it does not kill the loop: it appends `MAX_STEPS_PROMPT` and sets `toolChoice: "none"`
  for the final step (`llm.ts:277-278`), so the model must produce a text summary and the turn ends
  on a real assistant reply.

**DeepSeek Harness** (`deepseek-ai/deepseek-harness`):

- The core `ReactLoopAgent.turn()` (`packages/core/agent-loop/src/agent.ts`) has no aggregate
  `maxSteps`/`maxTurns`; it completes when the model returns no tool calls.
- Bounding is a lifecycle policy (`agent/turn-stopping`) or the fail-closed `dsh-turn-budget` plugin
  (`maxStepsPerTurn`, `maxToolCallsPerTurn`, `maxProviderTokensPerTurn`). When a limit is hit the
  proposed step is rejected and the turn closes as `blocked` — a terminal, logged outcome.

## Proposed fix

Match the references with the smallest correct change:

1. Add `max_steps: Option<u32>` to `AgentParams` and plumb the resolved agent's `max_steps` into it
   at both construction sites: `crates/opencode-server/src/agentic.rs:83` and the subtask executor
   in `crates/opencode-session/src/prompt.rs` (~2210).
2. In `loop_inner`, replace the hard-coded `MAX_STEPS` break with the effective limit:
   - `None` means **unbounded** (no step cap), matching vanilla; the existing wall-clock
     `OPENCODE_RUN_TIMEOUT_MS` (default 30 min, `routes.rs:2936`) remains the runaway backstop.
   - `Some(limit)` stops after `limit` steps.
3. When a finite limit is reached, end the turn **visibly** instead of silently:
   call `SessionPrompt::finalize_incomplete_turn(session, "max-steps", "step budget reached (N
   steps); send a new prompt to continue")`, emit the session update, log a warning, then break.
   This guarantees a terminal assistant message the TUI can render.
4. Set `AgentInfo::build().max_steps` to `None` (vanilla parity: no cap) while keeping `plan`'s
   explicit bound. Remove or repurpose the now-dead `max_steps_for_agent`.
5. Add a regression test for the visible terminal and for honoring a small configured limit.

The vanilla `MAX_STEPS_PROMPT` + `tool_choice: none` wrap-up is an optional follow-up: `ChatRequest`
(`crates/opencode-provider/src/message.rs`) has no `tool_choice` field, so that variant would touch
every provider and is out of scope for this card. The visible terminal plus unbounded default
already removes the silent stop.

## Scope

- `crates/opencode-session/src/prompt.rs` (`AgentParams`, `loop_inner`, subtask params).
- `crates/opencode-server/src/agentic.rs` (pass `agent.max_steps`).
- `crates/opencode-agent/src/agent.rs` (build default).
- Regression tests in `opencode-session`.

## Non-goals

- Adding `tool_choice` to `ChatRequest` or the vanilla wrap-up prompt (optional follow-up).
- Changing the 30-minute wall-clock run budget (`OPENCODE_RUN_TIMEOUT_MS`).
- Reworking provider stream termination or the `BUG-043` terminal-state contract.
- Changing `opencode-agent/src/executor.rs` behavior beyond keeping it consistent if trivial.

## Done when

- A long turn no longer stops silently at 100 steps; with no configured cap it continues until the
  model stops or the wall-clock budget ends.
- A configured finite `max_steps` is honored and, when reached, leaves a visible terminal assistant
  message naming the step budget, not a bare `break`.
- `AgentParams` carries `max_steps` and both construction sites populate it.
- Regression tests cover the visible terminal and the finite-limit path.
- `cargo test -p opencode-session` and `cargo check --workspace` pass.

## Verification

- Unit/loop test: a mocked provider that issues tool calls; with `max_steps = Some(n)`, assert the
  loop stops at `n` and the session's last assistant message contains the visible step-budget text
  and a terminal finish reason.
- Test that `max_steps = None` does not break at the old 100 boundary (drive >100 lightweight steps,
  or assert the effective limit is `None`).
- `cargo test -p opencode-session`, `cargo test -p opencode-server`, `cargo check --workspace`,
  `cargo fmt --all -- --check`.
- Optional live sanity: run a long tool-heavy prompt on `deepseek/deepseek-flash` past 100 steps and
  confirm it no longer needs a manual "continue".

## Related Items

- `BUG-043` (`boards/done/session-run-can-end-without-terminal-state.md`) — terminal-state
  contract; this card reuses `finalize_incomplete_turn` but fixes a loop path that bypasses it.
- `BUG-038` (`boards/done/deepseek-reasoning-turn-stalls-mid-turn.md`) — 90s stream idle timeout;
  its notes describe the same "last message is a user Continue" shape, but the 100-step cap is a
  different cause.
- `BUG-044` (qa) — continuation/resume point; the manual "continue" recovery path.
- `BUG-016` / `BUG-023` (qa) — plan-mode stalls after tool calls without results.
- `EPIC-001` (refinement) — DeepSeek default-model reliability hardening (umbrella).
- `BUG-070` (qa) — the same fenced-snippet session's actual deliverable; unrelated to this stop.

## Dev Notes - 2026-11-11

Implemented the visible-terminal + configurable-limit approach.

- `AgentParams` (`crates/opencode-session/src/prompt.rs`) now carries `max_steps: Option<u32>`.
  `None` means no step cap; a finite value bounds the turn.
- `loop_inner` replaced the hard-coded `MAX_STEPS = 100` break with the configured limit. When a
  finite limit is reached it calls `SessionPrompt::finalize_incomplete_turn(session, "max-steps",
  "step budget reached (N steps); send a new prompt to continue")`, emits a session update, and
  breaks. The transcript now ends with a visible terminal assistant message instead of a bare tool
  result.
- Removed the dead `max_steps_for_agent` helper and the module `MAX_STEPS` constant.
- `crates/opencode-server/src/agentic.rs` passes `agent.max_steps` into `AgentParams`; both prompt
  routes already forward `resolved.params`.
- `AgentInfo::build()` now leaves `max_steps: None` (reference opencode parity: no step cap), so a
  normal `build` session continues past 100 steps until the model stops or the wall-clock budget
  ends. `plan` keeps its explicit `Some(50)`; custom agents keep `Some(100)` and now at least fail
  visibly at that limit.
- Subtask `AgentParams` sets `max_steps: None` because subtask execution uses
  `opencode-agent/src/executor.rs`, which already applies the agent's own `max_steps`.

### Verification

- New regression test `configured_max_steps_ends_turn_with_visible_terminal`
  (`crates/opencode-session/src/prompt.rs`): a mock provider that always emits a tool call with
  `max_steps = Some(3)` stops at exactly 3 tool calls and leaves the last assistant message with
  `finish_reason = "max-steps"` and visible "step budget reached" text. Passes.
- `cargo test -p opencode-session -p opencode-agent -p opencode-server` → all pass (22 + 76 + 3 +
  178 + 2 + 11), excluding two pre-existing macOS `/tmp` symlink failures in
  `instruction::tests::test_find_up_*` that also fail on the unmodified baseline.
- `cargo check --workspace` passes with `SCOPEMUX_CORE_DIR=/Users/cchrisleepyles/apps/scopemux-core`
  (the default `third_party/scopemux-core` checkout is absent in this workspace).
- `cargo fmt --all -- --check` clean.

### Behavior change to note in QA

- `build` sessions are now unbounded by step count (wall-clock `OPENCODE_RUN_TIMEOUT_MS` remains the
  backstop). `plan` and custom agents now honor their configured `max_steps` (50 and 100
  respectively) and will show the visible terminal message if they reach it.
- The reference `MAX_STEPS_PROMPT` + `tool_choice: none` graceful wrap-up was not implemented:
  `ChatRequest` has no `tool_choice` field. That is an optional follow-up if a summary-on-budget is
  wanted instead of a plain terminal message.

## Notes

- This is a distinct cause from BUG-043: the run returns `Ok(())`, so the terminal-state finalizer
  is never invoked.
- `max_steps_for_agent` is currently unused; implementers should either wire it or delete it.
- `ChatRequest` has no `tool_choice`, so the vanilla wrap-up variant needs a separate card if
  desired.

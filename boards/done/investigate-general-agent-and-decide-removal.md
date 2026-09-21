---
id: "FEAT-031"
title: "Investigate the builtin \"general\" agent and decide whether to remove it"
priority: "P2"
type: "research"
area: "FEAT"
spec: ""
status: "done"
created: "2026-09-21"
---

# Investigate the builtin "general" agent and decide whether to remove it

## Summary

Determine what the builtin `general` agent actually does in this product, why it exists, and whether it should be removed, demoted to a subagent, or kept. The current expectation is that it is redundant with `build` and that removing it is the right call, but that decision needs a grounded trace before anything is deleted.

## Outcome (2026-09-21)

Investigation completed and the decision was made to **disable `general` for now** rather than delete it.

- Verdict: `general` was redundant, not useless. It was selectable and did affect requests, but only as a strictly inferior duplicate of `build`: generic system prompt, `temperature 0.7`, and a defaults-only permission set that denied `question` and `plan_enter`. As a default it was unreachable because `resolve_agent_name` prefers `build`. Vanilla's `general` is a subagent, not a primary mode.
- Action taken: removed `BuiltinAgent::General` from `BuiltinAgent::all()` (so it is not registered), made `AgentInfo::default_agent()` return `build()`, dropped the `general` preference from `AgentRegistry::default_agent()`, and removed `general` from the TUI `known_agents`, the task-tool subagent catalog, and the session/metadata agent fallbacks (now `build`).
- The `BuiltinAgent::General` variant and `AgentInfo::general()` were retained so the disable is a one-line reversal.
- Documentation: `wiki/agent-modes-and-custom-agents.md` is the extensive write-up of the agent model, default resolution, custom-agent config, permission rulesets, and the disable decision.

Remaining open question (why this item stays open): should `general` be removed permanently, or reintroduced as a `Subagent` matching the reference's parallel general-purpose role? Revisit after some use of the disabled state.

## Why this exists

Vanilla OpenCode has no primary "general" mode. Its primary agents are `build` (the default) and `plan`; `general` exists only as a **subagent** for parallel multi-step work (`$HOME/repos/opencode-modded/packages/core/src/plugin/agent.ts:121-157`, where `AgentV2.defaultID` is build and `general` is created with `mode: "subagent"`).

This product models `general` differently: it is a **primary** agent and is wired as a fallback default, which makes it show up as a selectable "mode" in the TUI even though nothing on the live path actually selects it. That is confusing and is a known parity inconsistency already flagged in `wiki/coding-session-parity-audit.md:35` and `:71`. Since the product stance is a narrow daily-driver with parity deferred, the cheapest correct outcome may be to delete the builtin rather than reconcile it.

## Current understanding (starting point, verify before relying on it)

- `BuiltinAgent::General` is a first-class builtin (`crates/opencode-agent/src/agent.rs:35`, included in `all()` at `:53-62`).
- `AgentInfo::general()` sets `mode: AgentMode::Primary`, `system_prompt = "You are a helpful assistant. Complete the task given to you."`, `temperature = 0.7`, `max_steps = 20` (`crates/opencode-agent/src/agent.rs:238-260`). Compare `build()` (`:192-214`), which has no system prompt (so the model default prompt is used), no temperature override, and `max_steps = 100`.
- `AgentInfo::default_agent()` returns `general()` outright (`crates/opencode-agent/src/agent.rs:188-190`).
- `AgentRegistry::default_agent()` prefers `general` when present, then any non-hidden non-subagent, then any agent (`crates/opencode-agent/src/agent.rs:668-685`).
- The live server selection path ignores that fallback: `resolve_agent_name` prefers the requested agent, then `build`, then `registry.default_agent()` (`crates/opencode-server/src/agentic.rs:85-98`). So `general` is effectively never chosen as the default on the main path, while the registry API still advertises it as the default.
- The TUI defaults its current agent string to `"build"` (`crates/opencode-tui/src/context/app_context.rs:138`).
- The agent picker is populated from `GET /agent` (`crates/opencode-tui/src/api.rs:753`, `crates/opencode-server/src/routes.rs` `list_agents`), which returns every non-subagent agent — so `general` appears as a mode alongside `build` and `plan`.
- The prompt's hardcoded `known_agents` list also includes `"general"` (`crates/opencode-tui/src/components/prompt.rs:168-175`).
- `summary` is registered separately (`crates/opencode-agent/src/agent.rs:169`), so the builtin set is broader than the selectable modes; `compaction`/`title`/`explore` are `Subagent` modes and are filtered out of the picker already.

## Questions to answer

- Does any live code path (server, TUI, CLI, config merge, subagent dispatch) ever select the `general` agent by name, or is it purely advertised?
- Is `general` referenced by config defaults (`default_agent`), skills, plugins, docs, or tests in a way that would break on removal?
- What, if anything, would a user lose by deleting `general` that `build` does not already cover? Is its different temperature/system-prompt/max_steps ever observable today given the current prompt path?
- Should the reference's `general` **subagent** semantics (parallel multi-step work) be tracked as a separate future item, so removing the primary mode does not silently drop that capability?
- If `general` is removed, what is the correct `default_agent` behavior — `build`, or `config.default_agent` honored, or an explicit error when neither exists?
- Are there stale docs (e.g. `docs/archive/session.md:339`) or invariants that assert a `general` agent exists?

## Scope

- Trace every reference to the `general` builtin across `crates/`, config defaults, and runtime selection, and write down which are load-bearing.
- Determine the intended default-agent contract for this product and whether `build` is sufficient as the sole primary default.
- Recommend and record a decision: remove `general`, demote it to `Subagent` to match the reference, or keep it with a corrected role.
- If removal is chosen, enumerate the exact edit sites (enum variant, `from_builtin`, `all()`, `default_agent` fallbacks, TUI `known_agents`, tests) so implementation is ready.
- Only implement removal if the item is explicitly promoted to an implementation task; this item is learn-and-decide first.

## Non-goals

- Implementing a `general` subagent with parallel-task orchestration (reference parity), unless separately split out.
- Changing the `build`/`plan` agent behavior or the mode-picker UI design.
- Reworking how subagents are registered, spawned, or filtered.

## Done when

- Every reachable use of the `general` agent is listed with file references and a load-bearing/not-load-bearing judgement.
- The observable behavioral difference between `general` and `build` (if any) on the current live prompt path is stated explicitly.
- A recommendation is recorded: remove, demote, or keep, with the reasoning tied to the product stance.
- If removal is recommended, the concrete edit sites and any required test updates are listed.
- Any dropped reference capability (general-as-subagent) is either explicitly declared out of scope or split into a follow-up board item.

## Recommended verification

- Grep `crates/` for `BuiltinAgent::General`, `Self::general()`, `"general"`, and `default_agent` and classify each hit.
- Read `AgentInfo::general`/`build`, `AgentInfo::default_agent`, `AgentRegistry::default_agent`, `resolve_agent_name`, and the `/agent` route together and confirm which agent a fresh session actually uses.
- Run `ort-build`, then `ort`; open the mode picker/`/models`-style selector and confirm whether `general` is offered and selectable.
- Start a session with `general` explicitly selected and with `build`, and compare system prompt, temperature, and step budget actually sent (trace/logging) to see whether the difference is real or inert today.

## Related Items

- `BUG-004` Coding sessions run as bare chat with no agent context
- `FEAT-013` Model capability gating and deprecated model surfacing
- `START-020` Constrain primary product surface to the V1 workflow

## Notes

- This is the inverse of the parity audit's "default-agent inconsistency": even if `general` were the intended default, `resolve_agent_name` currently overrides it with `build`, so the two halves of the codebase disagree. Any resolution must pick one story.
- `compaction` is also registered as a builtin but is a `Subagent`/internal agent, which is the pattern to follow if `general` is kept as non-primary.

## Dev Notes

- Implemented the disable (see Outcome above). Code changes:
  - `crates/opencode-agent/src/agent.rs` — dropped `General` from `BuiltinAgent::all()` (`BuiltinAgent::all()` now 5 entries), `AgentInfo::default_agent()` returns `build()`, and `AgentRegistry::default_agent()` prefers `build` deterministically before falling back to the first non-hidden non-subagent.
  - `crates/opencode-tool/src/task.rs` — removed `general` from the static subagent catalog.
  - `crates/opencode-session/src/prompt.rs` — three agent-metadata fallbacks now default to `"build"` instead of `"general"`.
  - `crates/opencode-tui/src/components/prompt.rs` — removed `general` from `known_agents`.
- Documentation: added `wiki/agent-modes-and-custom-agents.md` and linked it from `wiki/README.md`.
- Tests: updated `builtin_agents_have_expected_defaults` to assert `general` is absent and the default is `build`; renamed the model-override test to `config_can_reintroduce_general_agent_with_model` to document the config re-enable path.
- Verification: `cargo fmt --all`; `cargo check -p opencode-session -p opencode-tui -p opencode-server`; `cargo test -p opencode-agent -p opencode-tool` (41 passed). Clippy on the affected crates shows only pre-existing unrelated warnings.
- Concurrency note: unrelated in-progress changes from another work session were left uncommitted and were not included in this change set.
- Closeout: merged via PR #51 into `development` (merge commit `9e4a8fd`).
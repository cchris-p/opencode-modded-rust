---
id: "SKILLS-005"
title: "Compare skill referencing and per-step reinjection parity with vanilla"
priority: "P2"
type: "research"
area: "SKILLS"
spec: "invariants/skills/runtime.md"
status: "todo"
created: "2026-09-21"
---

# Compare skill referencing and per-step reinjection parity with vanilla

## Summary

Determine how vanilla OpenCode makes in-session agents aware of available skills and compare it to OpenCode Rust, then decide whether Rust must mirror vanilla's per-step system-prompt reinjection convention.

## Why this exists

Skills sometimes appear to be "called automatically" in a session. That is not an automatic trigger: the model decides to call the `skill` tool because it has been told which skills exist. The mechanism matters for parity and for trustworthy skill use, so it should be captured and decided explicitly rather than left to inference.

## Observed vanilla behavior (evidence)

- `packages/opencode/src/session/system.ts:105-116` defines `SystemPrompt.skills(agent)`. It emits the line "Skills provide specialized instructions and workflows for specific tasks. Use the skill tool to load a skill when a task matches its description." followed by `Skill.fmt(list, { verbose: true })`.
- `packages/opencode/src/skill/index.ts:321` `fmt(list, { verbose: true })` renders an `<available_skills>` XML block with `<name>`, `<description>`, and `<location>` per skill; the non-verbose form renders `## Available Skills` markdown lines.
- `packages/opencode/src/session/prompt.ts:1257-1268` calls `sys.skills(agent)` inside the per-step model loop and appends it to the `system` array. The available-skills block is therefore regenerated and re-sent into the system prompt on every model call/step, not once per session.
- The block is permission-gated: `sys.skills` returns early when the agent denies the `skill` permission.
- `packages/opencode/src/tool/skill.txt` instructs the model to load a skill "listed in the system prompt", and `tool/skill.ts:9` describes the `name` parameter as "The name of the skill from available_skills".
- Net: the invocation is model-driven. Vanilla supplies awareness through a per-step reinjected system-prompt block plus a tool description that points at it.

## Observed Rust behavior (evidence)

- `crates/opencode-session/src/llm.rs:966` `build_system_prompt` assembles only `agent.system_prompt`, `input.system`, and the last user message `system`. It never injects an available-skills section.
- `crates/opencode-tool/src/skill.rs:314` gives the `skill` tool a single generic description line ("Load and execute a skill…"), with no reference to skills listed in the system prompt.
- `crates/opencode-tool/src/skill.rs` `parameters()` enumerates discovered skill names into the `name`/`skill_name` enums, so the model can only learn skill names through the tool JSON schema. It also discovers from `std::env::current_dir()` rather than the session workspace.
- The `/skill` server endpoint and TUI skill browser expose names to the user, but nothing injects available skills into the model's system prompt.

## Research questions

- Should Rust reinject an available-skills block into the system prompt on each model call like vanilla, and if so in which form (verbose `<available_skills>` XML, or a compact markdown list)?
- Should the block be built from the session workspace and filtered by the agent's `skill` permission, mirroring `Skill.available(agent)`?
- Should the `skill` tool description be updated to reference the system-prompt skill list?
- Should skill resolution for the tool schema use the session workspace instead of the process current directory?
- How does this interact with the existing per-step prompt-builder caching (the 2-part system prompt structure in `llm.rs`)?
- Is the current enum-only exposure adequate for the narrow daily-driver workflow, or does it produce under-discovery of skills?

## Scope

- Compare vanilla and Rust referencing/reinjection behavior for local filesystem skills.
- Decide whether Rust should adopt the per-step reinjection convention and record the decision.
- If adoption is chosen, define the exact prompt shape, permission filtering, workspace source, and caching interaction.

## Out of Scope

- URL-backed skills (`SKILLS-002`).
- Local discovery list parity, already covered by `SKILLS-003`.
- General instruction-file (AGENTS.md) reinjection, tracked by `BUG-009`.

## Done when

- The vanilla vs Rust referencing/reinjection difference is documented with file-level evidence.
- A product decision exists: adopt per-step available-skills reinjection, adopt an alternative, or explicitly defer/reject it.
- If implementation is chosen, a specific implementation-ready execution item is created (or this item is promoted), covering prompt shape, permission filter, workspace source, and caching.
- Any resulting contract change is restated in this repo's `invariants/skills/runtime.md`.

## Related Items

- `SKILLS-003` exact local skills list parity (discovery/listing, not prompt referencing).
- `SKILLS-002` URL-backed skills parity.
- `BUG-009` instruction files re-injected per read (adjacent context-injection concern).
- `invariants/skills/runtime.md` skills runtime contract.

## Notes

- Surfaced while investigating why skills appear to be called automatically in a session.
- "Automatic" skill calls are the agent acting on instructional context; the convention that supplies that context differs between vanilla and Rust.

## Investigation update - 2026-09-23

### Verified mechanism (reference pin `f54ce313b`)

- Vanilla `SystemPrompt.skills(agent)` (`packages/opencode/src/session/system.ts:105-116`) is permission-gated, filters to skills that carry a `description` (`Skill.fmt` drops descriptionless entries), and emits a 2-line preamble plus verbose `<available_skills>` XML.
- `Skill.available(agent)` (`packages/opencode/src/skill/index.ts:310-315`) filters per **skill name** via `Permission.evaluate("skill", skill.name, agent.permission)`.
- The block is produced inside the per-step model loop (`packages/opencode/src/session/prompt.ts:1258-1268`) and appended to the `system` array, so it is regenerated and re-sent on every model call.
- The `skill` tool parameter is a static free-form `{ name: string }` ("The name of the skill from available_skills") with no enum, and `tool/skill.txt` directs the model to the skills listed in the system prompt. `execute` calls `skill.require(name)` and asks permission with pattern `[name]`.

### Rust assembly-path correction

- The main agentic server path is `crates/opencode-server/src/routes.rs` `run_prompt_turn` -> `crates/opencode-server/src/agentic.rs` `resolve_agentic_context` -> `agentic::build_system_prompt` (agent/model base prompt + environment block). That static string is passed to `crates/opencode-session/src/prompt.rs` `prompt_with_update_hook` -> `loop_inner`, where `build_chat_messages` and `apply_caching` run *inside* the step loop, so the same system prompt is re-sent every step.
- The item's cited `crates/opencode-session/src/llm.rs:966 build_system_prompt` is real but only feeds the compaction path (`crates/opencode-session/src/compaction.rs` builds `StreamInput`); it is not the main agent-loop assembly point. `LlmAgent` carries no permission field.
- Net: Rust already re-sends the system prompt every step; it simply never contains a skills block. Adoption does not require per-step regeneration in the current architecture.
- `crates/opencode-tool/src/skill.rs` `parameters()` enumerates all discovered names as a JSON-schema `enum` built from `std::env::current_dir()`, while `execute` resolves skills from `ctx.directory`. That is a genuine source mismatch, and the enum also exposes descriptionless skills that vanilla hides from the model.
- Caching: `crates/opencode-provider/src/transform.rs` `apply_caching` marks up to two system messages ephemeral only for Anthropic/OpenRouter/Bedrock/Gateway; `ProviderType::Other` (which includes the product default `deepseek`) receives no explicit cache markers.

### Prompt-cost measurement (this machine)

- 36 local skills, all with descriptions; average description 320 chars, max 577.
- Vanilla verbose XML block: ~17.3k chars (~4.3k tokens) appended per call.
- Compact `## Available Skills` markdown form: ~12.4k chars (~3.1k tokens).

### Options considered

- **A. Vanilla parity** - verbose `<available_skills>` reinjection with per-skill-name permission filter, session-workspace source, vanilla tool description, and free-form `name` (drop the enum). Exact parity and fixes the `current_dir`/`ctx.directory` mismatch. Cost ~4.3k tokens/call, largely offset by provider-side prefix caching.
- **B. Lean enum-only** - no reinjection; keep schema exposure, fix the workspace source, and add descriptions to the schema property. Cheapest but retains under-discovery (JSON-schema enums cannot carry per-value descriptions) and is not parity.
- **C. Hybrid compact** - inject a non-verbose `## Available Skills` markdown block plus free-form `name`. Keeps the mechanism at ~28% less prompt text but is not the exact vanilla form.

### Recommendation

- Adopt **Approach A (vanilla parity)**. It is the only option that closes the discovery gap and matches vanilla's deliberate design (its code comment notes models ingest the verbose system-prompt form better than the tool description); the token cost is the main tradeoff.
- No code or invariant changes were made in this pass. Item stays in `todo`; implementation will be picked up later.

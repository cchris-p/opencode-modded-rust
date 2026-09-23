# Agent Modes and Custom Agents

This document explains how agents (a.k.a. "modes") are defined, registered, selected, and permissioned in the Rust product, and records how the builtin `general` agent was reconciled to the reference as a subagent. It is the authoritative reference for the agent-registry surface until superseded.

## Purpose

- Describe the agent model: `AgentInfo`, `AgentRegistry`, `AgentMode`, and where they live.
- List the builtin agents and their current enabled/disabled state.
- Explain how the effective agent is resolved for a coding-session prompt.
- Explain how to define custom primary modes and subagents through config.
- Document the permission ruleset an agent receives and the special-cased names.
- Record how the builtin `general` agent became a reference-matching subagent.
- Explain how `@agent-name` mentions in a prompt route to a subagent.

## Where agents live

- Definition and registry: `crates/opencode-agent/src/agent.rs`.
- Config shape: `crates/opencode-config/src/schema.rs` (`AgentConfigs`, `AgentConfig`, `AgentMode`).
- Permission rulesets: `crates/opencode-permission/src/ruleset.rs`.
- Server-side resolution: `crates/opencode-server/src/agentic.rs` and the `/agent` + prompt routes in `crates/opencode-server/src/routes.rs`.
- TUI surfaces: `crates/opencode-tui/src/api.rs`, `crates/opencode-tui/src/app/app.rs`, `crates/opencode-tui/src/components/prompt.rs`, `crates/opencode-tui/src/context/app_context.rs`.
- Task/subagent dispatch: `crates/opencode-tool/src/task.rs`.

## The agent model

### `AgentMode`

Three values (`crates/opencode-agent/src/agent.rs:93-99`, config mirror at `crates/opencode-config/src/schema.rs:478-482`):

- `Primary` — a top-level selectable mode (e.g. `build`, `plan`).
- `Subagent` — usable via the `task` tool, not in the primary mode picker (e.g. `explore`).
- `All` — the default for a config-defined custom agent (`AgentInfo::custom`, `agent.rs:366-387`); behaves as selectable but is not filtered as a subagent.

### `AgentInfo`

The runtime shape of an agent (`crates/opencode-agent/src/agent.rs:65-91`). Important fields:

| Field | Meaning |
| --- | --- |
| `name` | Registry key and agent id |
| `description` | Shown in the picker |
| `mode` | `Primary` / `Subagent` / `All` |
| `model`, `model_preference` | Optional model override (`provider/model`) |
| `system_prompt` | Agent prompt; when `None`, the model default prompt is used |
| `temperature`, `top_p`, `max_tokens` | LLM params |
| `max_steps` | Step budget |
| `allowed_tools` | Allowlist; when non-empty, tools not listed are denied |
| `permission` | `PermissionRuleset` evaluated at tool time |
| `hidden` | Excluded from listings |
| `native` | True for builtins, false for config-defined |
| `variant`, `color`, `options` | Misc metadata |

### `AgentRegistry`

Owns the agent map (`crates/opencode-agent/src/agent.rs:455-684`):

- `new()` registers every `BuiltinAgent::all()` entry, plus `summary` (`agent.rs:460-470`).
- `from_config` / `from_optional_config` / `from_project_dir` apply config overrides (`agent.rs:471-489`).
- `list()` returns all non-hidden agents, with `build` sorted first (`agent.rs:617-633`).
- `list_primary()` / `list_subagents()` filter by mode (`agent.rs:637-664`).
- `default_agent()` is described below.

### Builtin agents (`BuiltinAgent`)

Enum at `crates/opencode-agent/src/agent.rs:30-63`; constructors are `AgentInfo::build/plan/general/explore/compaction/title`.

| Agent | Mode | Native | Enabled today | Notes |
| --- | --- | --- | --- | --- |
| `build` | Primary | yes | yes | Default agent. No system prompt, so the model default prompt is used; `max_steps = 100` |
| `plan` | Primary | yes | yes | Plan mode; `edit` denied, `plan_exit` allowed |
| `general` | Subagent | yes | yes | General-purpose multi-step research; defaults plus `todowrite` deny; no dedicated prompt (uses the model default) |
| `explore` | Subagent | yes | yes | Read/search/bash allowlist |
| `compaction` | Subagent | yes | yes | Internal |
| `title` | Subagent | yes | yes | Internal title generator |
| `summary` | — | yes | yes | Registered separately from `BuiltinAgent::all()` (`agent.rs:468`) |

## Default-agent resolution

There are two related but distinct resolution rules:

1. `AgentInfo::default_agent()` (associated constructor, `crates/opencode-agent/src/agent.rs:187-189`) — a static fallback. It now returns `build()`. It has no production callers; it exists as an API affordance.
2. `AgentRegistry::default_agent()` (`crates/opencode-agent/src/agent.rs:667-684`) — returns `build` when present, else the first non-hidden, non-subagent agent, else any agent. The former `general` preference was removed.

The live request path does not use either directly. `resolve_agent_name` in `crates/opencode-server/src/agentic.rs:85-99` is authoritative:

1. Use the requested agent if it exists in the registry.
2. Otherwise fall back to `build` if present.
3. Otherwise fall back to `registry.default_agent()`.

Consequence: `build` is the effective default whenever it is registered, which it always is for the builtins. The TUI also hardcodes its current agent to `"build"` (`crates/opencode-tui/src/context/app_context.rs:138`), so a fresh session starts on `build` end to end.

The per-request resolution then assembles the agentic context (`crates/opencode-server/src/agentic.rs:39-73`): agent identity, system prompt plus environment block, permission-filtered tools, and LLM params. This is the BUG-004 path.

## Selection surfaces

- **Server**: `GET /agent` (`list_agents` in `crates/opencode-server/src/routes.rs`) returns every non-hidden agent whose mode is not `Subagent` — i.e. `Primary` and `All`. This is what the mode picker consumes, so `general` and `explore` are excluded from it.
- **TUI picker**: `refresh_agent_dialog` calls `list_agents` (`crates/opencode-tui/src/api.rs:763`, `crates/opencode-tui/src/app/app.rs:2830`). If the current agent is not in the returned set, the first entry becomes current.
- **Task tool**: registry-driven. `resolve_subagent` (`crates/opencode-agent/src/agent.rs`) filters to subagent-capable agents (`mode` `Subagent`/`All`, not hidden); unknown, primary, and hidden names fail without creating a session. The server supplies the resolver, including the `subagent_depth` guard and the subagent's configured model (`crates/opencode-server/src/routes.rs`, `resolve_task_subagent`). The former static `get_available_agents` catalog no longer exists.
- **Prompt `@agent-name` mentions**: `resolve_prompt_parts` (`crates/opencode-session/src/prompt.rs`) resolves a prompt token file-first, then falls back to a registered subagent name, producing a `PartInput::Agent` that instructs the model to call the `task` tool for that subagent. A mention of a registered non-subagent agent (e.g. `@build`) fails with a clear error; a token that is neither a file nor a registered agent stays plain text. The server wires this in `run_prompt_turn`. TUI `@` autocomplete (`crates/opencode-tui/src/components/prompt.rs`) still sources its agent suggestions from the primary-only `GET /agent` list, so subagent names must currently be typed; unifying that list with the mentionable subagents is an open follow-up.

## The `general` agent

### Previous divergence

The product once modeled `general` as an unrestricted primary and made it the registry default (`AgentInfo::default_agent()`). That diverged from the reference in two ways: vanilla has no primary `general` (its `general` is a **subagent**, `packages/opencode/src/agent/agent.ts:182-195`), and because `general` fell into `build_agent_ruleset`'s default arm it could not ask questions or enter plan mode. It was never the live default anyway, because `resolve_agent_name` prefers `build`.

### Current state (FEAT-049)

`general` is now registered as a reference-matching **subagent** (`crates/opencode-agent/src/agent.rs`):

- `BuiltinAgent::General` is back in `BuiltinAgent::all()` (6 builtins), so `AgentRegistry::new()` registers it.
- `AgentInfo::general()` sets `mode: Subagent`, the reference description ("General-purpose agent for researching complex questions and executing multi-step tasks. Use this agent to execute multiple units of work in parallel."), and no dedicated prompt (the model default prompt is used, as in the reference).
- `build_agent_ruleset("general", &[])` merges `default_ruleset()` with an explicit `todowrite` deny (`crates/opencode-permission/src/ruleset.rs`), matching the reference's `todowrite: "deny"`.
- `AgentRegistry::default_agent()` still returns `build`; `general` is never the default and never appears in the primary picker.

`general` is not a selectable primary, but it is subagent-capable and mentionable: `@general` (or `subagent_type: "general"`) routes through the `task` tool like `explore`.

## Defining custom agents

Agents come from config only; there is no markdown agent-file discovery like vanilla. Config sources and precedence follow the rest of the config loader. Relevant keys (`crates/opencode-config/src/schema.rs`):

- `default_agent: Option<String>` (`schema.rs:65`) — declared in config, but note it is currently **not consulted** by `resolve_agent_name`; `build` is the effective default. The only current reader is a CLI config print (`crates/opencode-cli/src/main.rs:3563`), so honoring `default_agent` remains an open follow-up.
- `mode: Option<AgentConfigs>` (`schema.rs:71`) — primary modes; every entry is forced to `AgentMode::Primary` (`crates/opencode-agent/src/agent.rs:491-494`).
- `agent: Option<AgentConfigs>` (`schema.rs:74`) — general agent definitions; use the entry's `mode` field to pick `primary` / `subagent` / `all` (`agent.rs:495-497`, `:551-555`).

`AgentConfig` fields (`crates/opencode-config/src/schema.rs:439-474`): `name`, `model`, `variant`, `temperature`, `top_p`, `prompt`, `disable`, `description`, `mode`, `hidden`, `options`, `color`, `steps`/`max_steps`, `max_tokens`, `permission`, `tools`.

Merge behavior (`apply_agent_config`, `crates/opencode-agent/src/agent.rs:510-603`):

- A config entry whose key matches an existing builtin patches that builtin; otherwise a new `AgentInfo::custom(key)` is created (`agent.rs:521-525`).
- `disable: true` removes the agent entirely (`agent.rs:516-519`).
- `prompt` sets `system_prompt`; omitting it leaves the model default prompt.
- `tools` maps tool name to enabled bool and is merged with `allowed_tools` (`agent.rs:575-589`).
- If `permission` or `tools` is present, the agent's permission becomes `build_agent_ruleset(key, user_rules)` (`agent.rs:591-600`).

### Example: a custom primary mode

```json
{
  "mode": {
    "review": {
      "description": "Read-only code reviewer",
      "prompt": "You review code and report findings without editing files.",
      "temperature": 0.2,
      "max_steps": 40,
      "permission": {
        "edit": "deny",
        "write": "deny"
      }
    }
  }
}
```

Because it is under `mode`, it is forced to `Primary`, appears in `GET /agent`, and therefore in the TUI mode picker. Selecting it sends its prompt, params, and permission-filtered tools.

### Example: a custom subagent

```json
{
  "agent": {
    "auditor": {
      "description": "Audits dependencies for vulnerabilities",
      "mode": "subagent",
      "prompt": "You audit dependency manifests.",
      "steps": 15
    }
  }
}
```

Subagents are hidden from the primary picker and are intended to be invoked through the `task` tool.

## Permission model for agents

Every agent's permission is a `PermissionRuleset` (`crates/opencode-permission/src/ruleset.rs`).

- `default_ruleset()` (`:206-264`) allows everything except: `doom_loop` = ask, `external_directory` = ask, `question` = deny, `plan_enter` = deny, `plan_exit` = deny, and `.env` reads = ask (`.env.example` = allow).
- `build_agent_ruleset(name, user_rules)` special-cases these names:
  - `build` → defaults + `question` allow + `plan_enter` allow.
  - `plan` → defaults + `question` allow + `plan_exit` allow + `edit` deny.
  - `explore` → a read/search/bash allowlist with everything else denied.
  - `general` → defaults + `todowrite` deny (reference `general` parity).
  - **Everything else, including custom agents, gets defaults only.**

Practical consequence: if you define a custom agent and want it to ask questions or enter plan mode, you must grant it explicitly via `permission` or `tools`; the name alone does not confer it. Note also that `AgentInfo::custom` seeds `permission` with `build_agent_ruleset(&name, &[])`, so a custom agent without config permission starts from the default arm.

## Reference comparison

Reference line `$HOME/repos/opencode-modded` (`dev`, pinned at `f54ce313b99a6661d7758ad042f7a6e05c8e0972`, re-pinned by `GATE-004`; line references re-verified at this pin):

- `build` is the default primary agent; `plan` is the other primary; `general` and `explore` are subagents (`packages/opencode/src/agent/agent.ts:141-216`).
- There is no primary `general` mode in the reference.
- The reference's `general` subagent is described as general-purpose for researching complex questions and running multiple units of work in parallel.
- `@agent-name` mentions in a prompt invoke a specialized subagent; the reference resolves a token file-first and then falls back to an agent name (`packages/opencode/src/session/prompt.ts:160-186`).

The product now matches this shape: `general` is a subagent, primaries are not offered as subagents, and a prompt mention routes to the matching subagent.

## Open questions / follow-ups

- Should `build` become the explicit `default_agent` config value shipped in the repo config?
- Should `AgentInfo::default_agent()` (the unused associated function) be deleted, or kept as the documented fallback affordance?
- Should custom agents with mode `All` appear in the primary picker, or should `All` be reserved for something else?
- Should TUI `@` autocomplete surface subagent-capable agents (not just primaries) so mentions are discoverable without typing the full name?

## File map

- `crates/opencode-agent/src/agent.rs` — `BuiltinAgent`, `AgentInfo`, `AgentRegistry`, resolution.
- `crates/opencode-server/src/agentic.rs` — request-time agent/system/tool resolution.
- `crates/opencode-server/src/routes.rs` — `/agent` listing and prompt handling.
- `crates/opencode-config/src/schema.rs` — `AgentConfigs`, `AgentConfig`, `AgentMode`, `default_agent`.
- `crates/opencode-permission/src/ruleset.rs` — `default_ruleset`, `build_agent_ruleset`.
- `crates/opencode-session/src/prompt.rs` — `PartInput`, `Agent` parts, and `resolve_prompt_parts` `@agent` mention resolution.
- `crates/opencode-tool/src/task.rs` — registry-driven `task` dispatch and subagent output.
- `crates/opencode-tui/src/app/app.rs`, `api.rs`, `components/prompt.rs`, `context/app_context.rs` — mode picker, client, prompt suggestions, default agent.

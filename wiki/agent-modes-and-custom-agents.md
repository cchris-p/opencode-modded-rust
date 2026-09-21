# Agent Modes and Custom Agents

This document explains how agents (a.k.a. "modes") are defined, registered, selected, and permissioned in the Rust product, and records the decision to disable the builtin `general` agent. It is the authoritative reference for the agent-registry surface until superseded.

## Purpose

- Describe the agent model: `AgentInfo`, `AgentRegistry`, `AgentMode`, and where they live.
- List the builtin agents and their current enabled/disabled state.
- Explain how the effective agent is resolved for a coding-session prompt.
- Explain how to define custom primary modes and subagents through config.
- Document the permission ruleset an agent receives and the special-cased names.
- Record why the builtin `general` agent was disabled and how to bring it back.

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
| `general` | Primary | yes | **disabled** | Removed from `BuiltinAgent::all()`; see below |
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

- **Server**: `GET /agent` (`list_agents` in `crates/opencode-server/src/routes.rs`) returns every non-hidden agent whose mode is not `Subagent` — i.e. `Primary` and `All`. This is what the mode picker consumes.
- **TUI picker**: `refresh_agent_dialog` calls `list_agents` (`crates/opencode-tui/src/api.rs:763`, `crates/opencode-tui/src/app/app.rs:2830`). If the current agent is not in the returned set, the first entry becomes current.
- **Prompt suggestions**: the prompt keeps a hardcoded `known_agents` list (`crates/opencode-tui/src/components/prompt.rs:168-174`). `general` was removed from it when the builtin was disabled.
- **Task tool**: `get_available_agents` (`crates/opencode-tool/src/task.rs:176-209`) is a separate static catalog used to derive disabled tools and a preferred model for a subagent; an unknown `subagent_type` still creates a subsession. `general` was removed from this list and the metadata fallbacks now default to `build`.

## The `general` finding

### What it was

`AgentInfo::general()` (`crates/opencode-agent/src/agent.rs:237-259`) defined:

- `mode: Primary`
- `system_prompt: "You are a helpful assistant. Complete the task given to you."`
- `temperature: 0.7`, `max_tokens: 8192`, `max_steps: 20`
- permission via `build_agent_ruleset("general", &[])`

It was made the default by `AgentInfo::default_agent()` and preferred by `AgentRegistry::default_agent()`.

### Why it was disabled

- **Redundant with `build`.** Both were unrestricted, general-purpose primaries. Nothing a user needed was unique to `general`.
- **Strictly worse than `build`.** Because `build_agent_ruleset` only special-cases `build`, `plan`, and `explore`, `general` fell into the default arm and received `default_ruleset()` (`crates/opencode-permission/src/ruleset.rs:266-358`, `:356`). That default denies `question` and `plan_enter` (`ruleset.rs:227-243`), so `general` could not ask clarifying questions or enter plan mode, while `build` explicitly allows both (`ruleset.rs:271-285`). `general` also replaced the model-specific default system prompt with a generic one.
- **Never actually the default on the live path.** `resolve_agent_name` prefers `build`, so `general` was advertised as a mode and as the registry default while being unreachable as the default in practice — an inconsistency flagged in `wiki/coding-session-parity-audit.md`.
- **Wrong reference role.** Vanilla OpenCode has no primary `general`; its `general` is a **subagent** for parallel multi-step work (`$HOME/repos/opencode-modded/packages/core/src/plugin/agent.ts:120-157`), while `build` is the default primary. This product modeled `general` as a primary, which is not parity and not a capability the reference's subagent semantics provide here anyway.

### How it is disabled

- `BuiltinAgent::General` is omitted from `BuiltinAgent::all()` (`crates/opencode-agent/src/agent.rs:53-61`), so `AgentRegistry::new()` never registers it by default.
- `AgentInfo::default_agent()` returns `build()`.
- `AgentRegistry::default_agent()` no longer special-cases `general`.
- The TUI `known_agents`, the task-tool catalog, and the session agent-metadata fallbacks no longer reference `general`.

The `General` enum variant and `AgentInfo::general()` are intentionally retained so re-enabling is a one-line change (see below).

### Re-enabling `general` (for now)

To restore it, add `BuiltinAgent::General` back to `BuiltinAgent::all()` and set `AgentInfo::default_agent()` back to `Self::general()` if it should also be the default. No other wiring is required for it to appear in the mode picker. A workspace can also re-add it without code changes by defining an agent named `general` in config, since config-defined agents with that key are created via `AgentInfo::custom`.

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
- `build_agent_ruleset(name, user_rules)` (`:266-358`) special-cases exactly three names:
  - `build` → defaults + `question` allow + `plan_enter` allow.
  - `plan` → defaults + `question` allow + `plan_exit` allow + `edit` deny.
  - `explore` → a read/search/bash allowlist with everything else denied.
  - **Everything else, including custom agents and the former `general`, gets defaults only.**

Practical consequence: if you define a custom agent and want it to ask questions or enter plan mode, you must grant it explicitly via `permission` or `tools`; the name alone does not confer it. Note also that `AgentInfo::custom` seeds `permission` with `build_agent_ruleset(&name, &[])` (`agent.rs:381`), so a custom agent without config permission starts from the default arm.

## Reference comparison

Frozen line `$HOME/repos/opencode-modded` at `e62912b5d18b73316c7bfd6e894b040698f6c880`:

- `build` is the default primary agent; `plan` is the other primary; `general` and `explore` are subagents (`packages/core/src/plugin/agent.ts:120-178`).
- There is no primary `general` mode in the reference.
- The reference's `general` subagent is described as general-purpose for researching complex questions and running multiple units of work in parallel — a capability this product does not implement.

This product therefore diverges by having `general` as a primary; disabling it reduces that divergence. If parallel general-purpose subagents are wanted later, they should be introduced as a `Subagent`-mode agent with that role, not as a selectable primary.

## Open questions / follow-ups

- Should the `general` builtin be removed permanently, or reintroduced as a `Subagent` matching the reference semantics?
- Should `build` become the explicit `default_agent` config value shipped in the repo config?
- Should `AgentInfo::default_agent()` (the unused associated function) be deleted, or kept as the documented fallback affordance?
- Should custom agents with mode `All` appear in the primary picker, or should `All` be reserved for something else?

These are tracked by `FEAT-031`.

## File map

- `crates/opencode-agent/src/agent.rs` — `BuiltinAgent`, `AgentInfo`, `AgentRegistry`, resolution.
- `crates/opencode-server/src/agentic.rs` — request-time agent/system/tool resolution.
- `crates/opencode-server/src/routes.rs` — `/agent` listing and prompt handling.
- `crates/opencode-config/src/schema.rs` — `AgentConfigs`, `AgentConfig`, `AgentMode`, `default_agent`.
- `crates/opencode-permission/src/ruleset.rs` — `default_ruleset`, `build_agent_ruleset`.
- `crates/opencode-tool/src/task.rs` — subagent catalog and task dispatch.
- `crates/opencode-tui/src/app/app.rs`, `api.rs`, `components/prompt.rs`, `context/app_context.rs` — mode picker, client, prompt suggestions, default agent.

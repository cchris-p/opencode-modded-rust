---
id: "FEAT-013"
title: "Model capability gating and deprecated-default-model surfacing"
priority: "P3"
type: "feature"
area: "FEAT"
spec: "invariants/coding-session-behavior.md"
status: "todo"
created: "2026-09-09"
---

# Model capability gating and deprecated-default-model surfacing

## Summary

The session layer should gate tool attachment and agentic behavior on the model's actual tool-calling capability, and the product should not silently leave the user on a deprecated default model that filters out of the provider registry.

Evidence:

- `ModelInfo` carries a tool capability hint (`supports_tools` / `tool_call` in several providers) but no request-construction site consults it before attaching tools.
- The repo default model `opencode/trinity-large-preview-free` (`opencode.jsonc`) is marked deprecated in the catalog, and deprecated models are removed from the runtime provider registry at bootstrap (`crates/opencode-provider/src/bootstrap.rs:1956-1959, 3191-3200`), which can leave a fresh user with no usable agent path and no explanation.
- `AgentRegistry::default_agent()` returns `general` (`agent.rs:668-685`) while reference and TUI defaults use `build`; capability-aware default selection should not compound this mismatch.

## Why this exists

`BUG-004` scopes model-capability handling to "gate tool attachment" and defers broader surfacing. A model that cannot call tools must not be handed a tool set, and a user must not be silently stranded on a filtered-out default model with no visible reason.

## Scope

- Read the resolved model's tool capability and suppress the tool set (with an explicit, visible signal) when the model cannot call tools.
- Detect when the configured default model is deprecated/filtered from the registry and surface a clear message/alternative rather than failing opaquely.
- Fix the `default_agent()` fallback ordering so a capability-aware default (`build`) is honored consistently with config.

## Non-goals

- Catalog/provider work for re-enabling deprecated models.
- Broad onboarding UX.

## Done when

- A tool-incapable model produces explicit, visible non-agentic behavior instead of a broken agent request.
- A user configured on a filtered-out deprecated default model sees an actionable message rather than a silent failure.
- `config.default_agent`, then `build`, is the resolution order.

## Recommended verification

- `cargo check -p opencode-agent -p opencode-session -p opencode-server`
- Unit tests for default-agent ordering and capability gating helpers.

## Related Items

- `BUG-004` Coding sessions run as bare chat
- `START-012` Refresh provider and model catalog
- `START-008` Full parity deferred

## Notes

- Tool-capability gating inside `BUG-004` is the minimal seam; this card carries the fuller surfacing and default-agent ordering work.

# Wiki Index

The wiki contains architecture direction, version planning, and product-boundary documents for `scopemux-code`.

## Document roles

- `wiki/` explains intent, scope, version gates, and roadmap direction.
- `invariants/` contains normative final-system truths.
- `docs/` contains implementation and behavior documentation for the current codebase.
- `boards/` tracks work items, deferred items, and future product triage.

## Navigation

- `vision.md` defines the product intent and near-term posture.
- `product-boundary.md` defines what this product is and is not.
- `reference-strategy.md` explains how the TypeScript repo is used as a reference line.
- `agent-evaluation-strategy.md` defines how the Rust runtime is judged across V1 through V3.
- `scopemux-integration-plan.md` defines the deferred retrieval boundary, how `scopemux-core` can help this project, and the guardrails for integrating it.
- `agent-debugging-without-tui.md` defines the preferred non-TUI QA/debug path for agent-session runtime work.
- `cli-surface.md` is the canonical reference for CLI task-surface and TUI launch/detach/attach behavior, and maps the owning `CLI-*` board items.
- `advanced-coding-session-polling.md` defines the low-context waits on coding-session state, including the agreed first agent-tool slice and its follow-up cards.
- `coding-session-parity-audit.md` audits the Rust coding-session agentic path against the reference behavior (`BUG-004` evidence).
- `agent-modes-and-custom-agents.md` defines the agent/mode model, builtin agents, default resolution, custom-agent config, permission rulesets, and the disabled `general` builtin.
- `provider-side-visibility.md` inventories what a coding-session provider receives, classifies it by sensitivity, and documents exposure-limiting controls and per-provider payload differences.
- `v1.md` defines the first serious personal daily-driver target.
- `v1-runtime-loop.md` defines the concrete bounded-task execution model for V1.
- `v2.md` defines the next reliability and retrieval step.
- `v3.md` defines broader daily-use coverage.
- `future-versions.md` captures later capability areas and deferred surfaces.

## Normative boundary

- The wiki is planning and architecture guidance.
- Invariants are the authoritative project truths for the final desired system.
- Reference by path, not by inheritance: files in `$HOME/repos/opencode-modded` may inform this repo, but they do not become policy unless adopted here.

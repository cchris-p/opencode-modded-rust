---
id: "SCOPE-005"
title: "Activate the ScopeMux provider only for the local Qwen-via-Ollama model"
priority: "P1"
type: "feature"
area: "SCOPE"
spec: "invariants/providers.md"
status: "doing"
created: "2026-09-25"
---

# Activate the ScopeMux provider only for the local Qwen-via-Ollama model

Compile the native ScopeMux retrieval provider into the product binary, but
activate it only when the effective model is the local Qwen model served by
Ollama. Every other model keeps the generic repository-local provider as the
default and fallback.

## Why this exists

`SCOPE-002` connected `scopemux-core` behind the `START-025` retrieval-provider
boundary, but no build profile enabled the `native` feature, so the provider was
dormant. The product decision (2026-09-25, resolving the `H-002` `[HUMAN]` gate in
`$HOME/apps/scopemux-notes`) is to use the small local Qwen model as the
experimentation surface: scopemux's token-budgeted, structural context should let
a small local model run more efficiently, which is the integration's goal.

Scoping activation to the local model keeps the default daily-driver path
(`deepseek/deepseek-flash`) and every unsupported workspace on the generic
provider, preserving the V1 guardrails.

## Scope

- Runtime provider selection keyed on the active model: `ollama` + a model id
  starting with `qwen` selects the ScopeMux provider; everything else selects the
  generic provider.
- Thread the active model from prompt assembly into the retrieval boundary.
- Enable the `native` feature for the default/dev product build so the provider
  is actually compiled, with the pinned `scopemux-core` as a build prerequisite.
- Keep the generic provider as the default and fallback, and keep ScopeMux
  evidence-only with no runtime authority.

## Acceptance Criteria

- With the effective model `ollama/qwen*`, retrieval runs through the ScopeMux
  provider; for every other model (including the product default) it runs through
  the generic provider.
- A non-native build still selects the generic provider for every model.
- The default `opencode` binary compiles the native provider; the C core is a
  documented, scripted build prerequisite.
- No change to task state, lifecycle, verification, or review authority.
- `cargo fmt`, clippy, and the crate/session tests pass, plus the native provider
  test against the pinned core.

## Verification

- `cargo test -p opencode-scopemux` (gate + selection unit tests) and
  `cargo test -p opencode-scopemux --features native`.
- `cargo test -p opencode-session`.
- `SCOPEMUX_CORE_DIR=<core> cargo build -p opencode-cli` (default features include
  `scopemux-native`).
- Manual: run the TUI with the Qwen-via-Ollama model and confirm the scopemux
  provenance appears; switch to the default model and confirm generic.

## Related Items

- `SCOPE-002` Integrate scopemux-core behind the retrieval-provider boundary.
- `SCOPE-003` Product projection of task state into plan nodes.
- `START-025` Add retrieval-provider boundary for task context assembly.
- `WI-030` Rust grammar and reference resolution (upstream, `scopemux-core`).
- `H-002` ScopeMux integration program resume handoff (`$HOME/apps/scopemux-notes`).

## Implementation (2026-09-25)

Branch `feature/scopemux-qwen-local-activation`, base `development`.

- `opencode-scopemux`: `model_scope_enabled(provider_id, model_id)` and
  `provider_for(provider_id, model_id)`; the ScopeMux provider is returned only
  for `ollama` + a `qwen`-prefixed model id, otherwise the generic provider.
- `opencode-session`: `retrieval::retrieve` takes the active `ModelRef` and selects
  via `provider_for`; `create_user_message` passes `input.model`.
- Build: `opencode-session` exposes `scopemux-native` (`opencode-scopemux/native`);
  `opencode-cli` enables it by default, so the shipped binary compiles the provider.
- Pin: `scripts/fetch-scopemux-core.sh` advanced to `scopemux-core` `main`
  `230383ea2ee315c2632ad5742e7a0fb4a04e89fa` (headers unchanged by `PR #20-#22`).
- Invariants: `providers.md` and `integration-scope.md` record the activation
  scope; `AGENTS.md` records the build prerequisite and activation rule.
---
id: "START-027"
title: "Unify provider setup into one authoritative user path"
priority: "P1"
type: "feature"
area: "START"
spec: "wiki/v1.md"
status: "doing"
created: "2026-09-01"
---

# Unify provider setup into one authoritative user path

## Summary

Define and implement one authoritative provider setup path so users do not have to mentally reconcile shell presets, TUI settings, provider-specific auth flows, and special-case Ollama setup behavior.

## Why this exists

Current provider setup behavior is split across multiple surfaces and stories:

- `START-009` introduced a minimal `Settings > Provider` picker.
- `START-015` added OpenAI-specific auth behavior inside that settings flow.
- `START-019` added native Ollama support plus a local-model-first runtime path.
- local shell launch presets such as `opencode-use-ollama-*` and runtime defaults still influence the effective provider/model path outside the visible TUI setup flow.

This leaves the product without one authoritative answer to "how do I set up or change providers?" and creates avoidable confusion about where provider selection, model selection, auth, defaults, and Ollama host configuration are actually supposed to happen.

## Product Direction

- `Settings > Provider` is the single authoritative user-facing provider setup path for V1.
- Runtime defaults, config-file settings, and shell helpers may still exist, but they are secondary override mechanisms rather than competing primary setup flows.
- The TUI must surface enough provider state that a user can understand the current effective provider, model, auth state, and Ollama host path from that one area.
- Shell helpers such as `opencode-use-ollama-*` and launch aliases are advanced local conveniences, not the canonical product setup story.

## Scope

- Identify every current provider setup surface that can materially change runtime behavior.
- Decide which single user-facing setup path is authoritative for V1.
- Define how that authoritative path covers:
- provider selection
- model selection
- auth entry or status
- Ollama local-model configuration
- config-file and shell-driven overrides
- Minimize or remove conflicting parallel setup paths where practical.
- Create explicit follow-up items for any setup behavior that must temporarily remain split.

## Implementation Direction

- Keep the canonical setup entry point centered in `Settings > Provider` rather than inventing a separate onboarding wizard.
- Make the provider screen cover these concerns in one coherent flow:
- active provider selection
- active model selection
- provider auth status and entry points where relevant
- Ollama host/base URL visibility and edit path
- current effective source of truth when config overrides are active
- Preserve advanced config-file overrides, but make their precedence explicit in the product instead of leaving them implicit.
- Preserve shell helpers only as out-of-band developer conveniences; do not rely on them for the normal product setup path.
- Reconcile the current curated remote-provider surface with the explicit Ollama local-model path so the user does not experience them as two unrelated systems.

## Concrete outcomes required

- A user can answer "which provider am I using right now?" from the provider screen.
- A user can answer "which model am I using right now?" from the provider screen.
- A user can answer "am I authenticated correctly for this provider?" from the provider screen.
- A user can answer "where is Ollama pointing?" from the provider screen when Ollama is active or available.
- A user does not need shell-only setup knowledge to use the supported Ollama path.
- If config or environment overrides are winning, the product makes that obvious instead of silently behaving differently from the settings UI.

## Likely Touchpoints

- `crates/opencode-tui/src/components/settings.rs`
- `crates/opencode-tui/src/components/dialogs/provider.rs`
- `crates/opencode-tui/src/app/app.rs`
- `crates/opencode-tui/src/api.rs`
- `crates/opencode-server/src/routes.rs`
- `crates/opencode-provider/src/bootstrap.rs`
- `crates/opencode-config/src/loader.rs`
- `opencode.jsonc`

## Explicit non-goals

- Full upstream parity for every provider onboarding flow.
- Designing a separate first-run wizard unless implementation proves the settings-based path is insufficient.
- Broad generic support for every OpenAI-compatible host under one polished abstraction.
- Preserving hidden or shell-only setup behavior as equal alternatives to the canonical UI path.

## Done when

- The product has one clearly documented authoritative provider setup path.
- The TUI, config, runtime defaults, and shell helpers no longer feel like competing setup systems.
- The relationship between visible setup UX and advanced override paths is explicit instead of implied.
- Any intentionally retained non-authoritative override paths are documented as advanced workflows.

## Acceptance detail

- `Settings > Provider` is explicitly documented and implemented as the authoritative provider setup path.
- The provider screen shows the effective provider, effective model, and effective auth state.
- The provider screen exposes the explicit Ollama local-model path as part of the same setup story rather than a separate hidden mechanism.
- The runtime no longer depends on shell helper setup to make the supported Ollama path usable.
- If a config file or environment override is taking precedence over the settings selection, the user can see that clearly.
- Existing provider-specific setup work from `START-015` and `START-019` is either absorbed into the unified flow or explicitly linked as subordinate implementation detail rather than left as separate user-facing concepts.
- Any retained advanced override workflows are documented in repo-local docs and referenced from the board notes.

## Verification

- Open `Settings > Provider` and confirm it exposes one coherent flow for provider, model, auth, and Ollama-related setup.
- Verify OpenAI auth state is visible from the same surface that controls provider/model selection.
- Verify Ollama provider selection, model selection, and host/base URL are visible from the same surface without shell-only knowledge.
- Verify a user can switch between a remote provider and Ollama without needing to learn two different setup systems.
- Verify explicit config overrides still work, and that the UI makes their precedence visible instead of silently disagreeing with runtime behavior.
- Verify shell helpers remain optional conveniences and are not required for the normal supported provider path.

## Deliverables

- One authoritative provider-setup UX and precedence direction captured in this card.
- Implementation changes needed to align TUI, runtime defaults, config precedence, and provider-specific setup under that direction.
- Updated repo-local documentation for the final provider setup story.
- Follow-up cards only for genuinely deferred work, not for unresolved authority or ownership of the setup path.

## Related Items

- `START-009` Port provider setup from OpenCode
- `START-012` Refresh provider and model catalog
- `START-015` Mirror OpenAI auth configuration in settings
- `START-019` Add native Ollama support for the local-model-first V1 path

## Notes

- This should be handled before additional provider-surface polish so the product stops accumulating more setup variants without a clear authority model.
- The likely end state is not "support every setup path equally"; it is one primary path plus explicitly secondary override paths.

## Dev Notes

- `Settings > Provider` now persists the selected provider/model path through the config route instead of keeping it session-local only.
- The server now reports effective provider, effective model, auth source, and Ollama host source so the TUI can show the real winning setup path.
- `/connect` now routes back into `Settings > Provider` so the product no longer presents a competing provider-setup entry point.
- Added repo-local provider setup docs in `docs/provider-setup.md` and linked the workflow from `USER_GUIDE.md`.

## Verification Notes

- `cargo fmt --all`
- `cargo check -p opencode-server -p opencode-tui`

### PR Link

- https://github.com/cchris-p/opencode-modded-rust/pull/21

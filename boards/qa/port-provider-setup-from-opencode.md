---
id: "START-009"
title: "Port provider setup from OpenCode"
priority: "P1"
type: "feature"
area: "START"
spec: ""
status: "qa"
created: "2026-08-29"
---

# Port provider setup from OpenCode

## Summary

Add a minimal but visible provider-configuration surface in `Settings > Provider` that follows the current OpenCode direction for the V1 provider set without taking on full onboarding or auth parity.

## Why this exists

The current provider configuration surface feels incomplete compared with the reference product. V1 needs a recognizable reconfiguration path from settings, even if richer auth and provider-specific flows land separately.

## Scope

- Review the current provider configuration flow in `/Users/cchrisleepyles/repos/opencode-modded`.
- Match V1 provider scope to `START-012`: `openai`, `anthropic`, `deepseek`, and `openrouter`.
- Implement a minimal but visible `Settings > Provider` reconfiguration surface for those providers.
- Allow provider selection and model selection where the existing local wiring can support it safely.
- Keep the UI intentionally small; do not expand this story into a first-run onboarding flow.
- Defer editable OpenAI login and API key entry to a separate story.
- Defer richer provider-specific credential entry and setup variations to follow-up stories as needed.

## Non-goals

- First-run onboarding
- Full upstream parity for provider setup UX
- Implementing OpenAI auth login flow in this story
- Implementing a general credential-entry flow for every provider in V1

## Done when

- `Settings > Provider` exists as an obvious place to reconfigure providers in the TUI.
- The visible provider scope matches `START-012`: `openai`, `anthropic`, `deepseek`, and `openrouter`.
- The screen is minimal rather than ornamental, and does not pretend to support onboarding paths that are still deferred.
- Model selection is supported where current wiring allows it without pulling in the deferred auth work.
- The implementation documents intentional deviations from `/Users/cchrisleepyles/repos/opencode-modded`.
- Follow-up board items exist for deferred OpenAI auth work and any provider-specific credential gaps.

## Recommended verification

- Open the TUI settings flow and verify `Provider` is present as a clear reconfiguration path.
- Confirm the provider list shown there matches the V1 catalog from `START-012`.
- Verify any available model-selection path works from settings without requiring first-run onboarding.
- Confirm deferred credential and login actions are represented by linked follow-up stories rather than implied support.

## Related items

- `START-002` Freeze TS reference line
- `START-008` Full parity deferred
- `START-012` Refresh provider and model catalog
- `START-015` Mirror OpenAI auth configuration in settings

## Notes

- Mirror the reference flow selectively rather than treating full parity as required.
- This story is for settings-time reconfiguration, not first-run onboarding.

## Dev Notes

- Added a dedicated `Settings > Provider` route in the TUI with a minimal two-pane provider/model picker.
- Limited the visible provider surface in that screen to the current V1 set: `openai`, `anthropic`, `deepseek`, and `openrouter`.
- Reused the existing session-local model selection wiring instead of introducing credential persistence or first-run onboarding in this story.
- Credential entry and OpenAI login remain explicitly deferred to `START-015`, which is surfaced directly inside the settings screen.
- Intentional deviation from `/Users/cchrisleepyles/repos/opencode-modded`: this branch provides settings-time provider/model re-selection only, not the fuller upstream auth/setup flow.

## Verification

- `cargo check -p opencode-tui`

## PR Status

- Branch pushed: `bug/START-009-port-provider-setup-from-opencode`
- PR opened: `https://github.com/cchris-p/opencode-modded-rust/pull/2`
- Separate worktree: `/var/folders/r5/fk0c2ljn1zs436tyhk6mrt2r0000gp/T/opencode/start-009`

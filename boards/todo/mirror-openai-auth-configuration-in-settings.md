---
id: "START-015"
title: "Mirror OpenAI auth configuration in settings"
priority: "P1"
type: "feature"
area: "START"
spec: ""
status: "todo"
created: "2026-08-29"
---

# Mirror OpenAI auth configuration in settings

## Summary

Add a dedicated OpenAI settings flow that mirrors the reference product's auth configuration direction closely enough for V1, including API key entry and OpenAI login from `Settings > Provider`, without taking on first-run onboarding.

## Why this exists

`START-009` is intentionally limited to a minimal provider reconfiguration surface. OpenAI auth has extra complexity and should be tracked explicitly so API key entry and login support are not left as an implied dependency.

## Scope

- Review the OpenAI auth/configuration flow in `/Users/cchrisleepyles/repos/opencode-modded`.
- Mirror the settings-time OpenAI auth surface for V1 as closely as practical.
- Support direct OpenAI API key entry from the TUI settings flow.
- Support the existing OpenAI login/auth callback style already present in the Rust stack, centered on plugin-auth-backed code or token entry rather than inventing a new auth mechanism.
- Wire the resulting OpenAI auth state into the provider settings flow used by `START-009`.
- Reuse the existing persisted auth store and server auth routes instead of introducing a parallel credential system.
- Ensure the resulting OpenAI auth can satisfy the runtime path that expects either a provider API key or OpenAI-specific OAuth-backed fetch wiring.
- Keep the work focused on settings/provider reconfiguration, not first-run onboarding.

## Non-goals

- First-run onboarding
- Broad auth parity for non-OpenAI providers
- Full provider setup parity across every upstream provider
- A brand-new browser-only OAuth architecture if the existing plugin auth callback flow is sufficient

## Done when

- The TUI has a dedicated OpenAI auth configuration path from settings/provider.
- A user can fully enter or update an OpenAI API key from that path.
- A user can start and complete the current OpenAI login flow from settings/provider using the same underlying auth mechanism already supported by the Rust server/plugin stack.
- OpenAI auth written through the settings flow persists in the shared auth store used by the app.
- If login support still falls short of the reference product because of missing lower-level browser or well-known auth plumbing, that gap is documented explicitly and split into a follow-up item instead of being hidden.
- `START-009` can link to this story instead of owning OpenAI auth details directly.

## Recommended verification

- Compare the OpenAI settings flow against `/Users/cchrisleepyles/repos/opencode-modded` and document any intentional deviations.
- Verify API key entry persists and is reused by the OpenAI provider path.
- Verify persisted auth is visible through the same underlying auth state used by CLI/server auth commands.
- Verify the OpenAI runtime path succeeds when configured by API key.
- If login support is implemented, verify the end-to-end settings login flow stores usable OpenAI auth and that the runtime path can use it.

## Related items

- `START-009` Port provider setup from OpenCode
- `START-012` Refresh provider and model catalog
- `START-008` Full parity deferred

## Notes

- Prefer a close mirror of the reference product's OpenAI auth configuration over inventing a Rust-specific alternative UX.
- The local Rust stack already has persisted auth storage plus plugin-auth authorize/callback routes; this story should build on that path.
- In the current Rust codebase, the practical V1 mirror target for OpenAI login is the existing pasted code/token callback flow unless a richer browser-driven path is already achievable without major new plumbing.
- If login support depends on missing lower-level auth plumbing, capture that explicitly instead of silently downgrading the story.

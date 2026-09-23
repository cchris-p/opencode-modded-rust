---
id: "START-032"
title: "Match vanilla Codex authentication parity"
priority: "P1"
type: "feature"
area: "START"
spec: "docs/provider-setup.md"
status: "qa"
created: "2026-09-23"
---

# Match vanilla Codex authentication parity

## Summary

Gate `START-029` on matching vanilla OpenCode's current OpenAI/Codex authentication options 1:1 before declaring Codex 5.5 compatibility usable in the Rust product.

The desired provider connection flow should expose all supported vanilla OpenAI/Codex auth paths, especially the ChatGPT Plus/Pro browser and headless login paths, not only manual API key entry.

## Current Evidence - 2026-09-23

Vanilla/reference evidence was refreshed with `git -C "$HOME/repos/opencode-modded" fetch origin dev` before this refinement pass.

Vanilla OpenCode currently defines the OpenAI/Codex auth surface in `$HOME/repos/opencode-modded/packages/opencode/src/plugin/openai/codex.ts`:

- `CodexAuthPlugin` registers `auth.provider: "openai"`.
- Its `methods` array exposes exactly three user-facing methods: `ChatGPT Pro/Plus (browser)`, `ChatGPT Pro/Plus (headless)`, and `Manually enter API Key` (`codex.ts:436-553`).
- Browser login uses PKCE against `https://auth.openai.com`, starts a localhost callback server on port `1455`, returns `method: "auto"`, and stores OAuth `refresh`, `access`, `expires`, and `accountId` (`codex.ts:436-466`).
- Headless login uses OpenAI device authorization through `/api/accounts/deviceauth/usercode`, tells the user to open `https://auth.openai.com/codex/device` and enter the displayed user code, polls `/api/accounts/deviceauth/token`, exchanges the authorization code, returns `method: "auto"`, and stores OAuth `refresh`, `access`, `expires`, and `accountId` (`codex.ts:467-549`).
- API key entry remains present as a method with `type: "api"` (`codex.ts:550-553`).
- For OAuth auth, the plugin rewrites OpenAI responses/chat-completions requests to `https://chatgpt.com/backend-api/codex/responses`, injects `Authorization: Bearer <access>`, sends `ChatGPT-Account-Id` when available, refreshes expired access tokens, and sets `apiKey: OAUTH_DUMMY_KEY` so the OpenAI provider path can run without `OPENAI_API_KEY` (`codex.ts:325-435`).
- Vanilla's current server API also has integration routes for listing integrations and starting/completing OAuth attempts: `packages/protocol/src/groups/integration.ts:10-120` and `packages/server/src/handlers/integration.ts:19-103`. The older provider auth service shape still exists in `packages/opencode/src/provider/auth.ts`, including method prompts and method-index authorization, but current external API parity should be checked against the integration endpoints as well as the plugin hook.

Rust current state:

- Rust has provider auth endpoints at `/provider/auth`, `/provider/{id}/oauth/authorize`, and `/provider/{id}/oauth/callback` (`crates/opencode-server/src/routes.rs:3397-3640`).
- Rust `ProviderAuth` can list plugin auth methods and persist API or OAuth auth into `AuthManager` (`crates/opencode-server/src/oauth.rs:24-158`).
- Rust `Settings > Provider` displays only the first OpenAI auth method as `Login flow: <method.name>` and shows shortcuts `a API key   l Login   x Clear` (`crates/opencode-tui/src/components/settings.rs:568-624`).
- Pressing `l` always starts `openai` OAuth method index `0`; there is no method picker for browser vs headless (`crates/opencode-tui/src/app/app.rs:2798-2814`).
- The TUI always enters `OAuthCode` input mode for OAuth and requires non-empty input before calling callback (`crates/opencode-tui/src/app/app.rs:2699-2716`). That does not match vanilla's `method: "auto"` browser/headless methods, where callback completion may require no pasted code and headless completion should poll after the user enters the code in the browser/device page.
- Rust API structs collapse auth methods to `name` and `description` only (`crates/opencode-tui/src/api.rs:244-247`), so method type/prompt metadata from vanilla's richer auth method shape is not available to the TUI.

Implementation should therefore focus on exposing and completing the three vanilla OpenAI auth methods through the Rust TUI/provider setup flow, not on adding a Codex-only shell helper or changing defaults.

## Why this exists

`START-029` currently asks whether Codex 5.5 can work through the existing OpenAI provider path. That is incomplete unless the auth path matches vanilla OpenCode closely enough for the user's normal workflow.

The user specifically relies on ChatGPT Plus/Pro headless auth and does not normally use manual OpenAI API keys. Manual API key entry should remain supported if vanilla supports it, but it must not be the only path considered compatible.

## Gates

- Blocks completion of `START-029` Set up OpenAI Codex 5.5 compatibility.
- `START-029` can still investigate model ids and transport behavior, but cannot be marked done until this auth-parity gate is satisfied or an explicit intentional deviation is approved and documented.

## Required Vanilla Reference

- Fetch the latest vanilla/reference `dev` before comparing: `git -C "$HOME/repos/opencode-modded" fetch origin dev`.
- Compare against `$HOME/repos/opencode-modded` on `dev`, not a stale local checkout.
- Identify the vanilla OpenAI/Codex auth implementation paths for provider connection, stored credentials, browser login, headless login, and API key entry.
- Treat `packages/opencode/src/plugin/openai/codex.ts` as the primary auth method source for this item unless a fresher vanilla `dev` check shows the surface moved.
- Check whether the Rust implementation should continue using the existing `/provider/*/oauth/*` route shape or should add compatibility with vanilla's newer `/api/integration/*` route shape. The implementation can keep Rust's existing route shape if the user-facing behavior and persisted auth semantics match vanilla, but the choice must be documented in `docs/provider-setup.md`.

## Scope

- Audit vanilla OpenCode's OpenAI/Codex provider connection flow and enumerate every supported auth option.
- Confirm whether vanilla presents these options when connecting the OpenAI provider:
- ChatGPT Plus/Pro browser login.
- ChatGPT Plus/Pro headless login.
- Manual OpenAI API key entry.
- Map each vanilla option to the Rust product's current provider setup/auth routes and TUI settings surfaces.
- Implement the missing Rust provider setup UI needed to choose among all OpenAI auth methods instead of hardcoding method index `0`.
- Implement correct handling for OAuth `method: "auto"` completions so browser and headless methods can complete without requiring a pasted code in the TUI input box.
- Ensure headless auth clearly displays the vanilla URL and user code instructions and can complete/poll through the existing plugin callback path.
- Ensure browser auth clearly displays or opens the authorization URL and completes after the localhost callback without requiring manual code entry.
- Preserve and verify manual API key entry as the API-key path.
- Ensure OAuth-authenticated OpenAI/Codex requests use the plugin-provided custom fetch/auth behavior rather than falling back to `OPENAI_API_KEY`.
- Update `docs/provider-setup.md` with the final OpenAI/Codex auth options and any intentional route/API deviations from vanilla.
- Preserve API key entry as a supported option if vanilla supports it, but do not treat API key-only setup as sufficient for Codex compatibility.

## Implementation Direction

- Extend the Rust provider auth method data model sent to the TUI so it carries at least method index, label, type (`oauth` or `api`), and enough prompt/description data to render the vanilla OpenAI options accurately.
- In `Settings > Provider`, replace the single `l Login` shortcut with an OpenAI auth choice flow that lists `ChatGPT Pro/Plus (browser)`, `ChatGPT Pro/Plus (headless)`, and `Manually enter API Key` when those methods are reported by the plugin.
- For an API method, route to the existing API key input/save behavior.
- For an OAuth method with `method: "auto"`, call authorize with that method index, show the returned URL/instructions, and call callback with no code when the user confirms completion or when the method can be completed automatically. Do not require a non-empty input string for auto methods.
- For an OAuth method with `method: "code"`, preserve the current code-entry behavior.
- Keep `/provider/{id}/oauth/authorize` and `/provider/{id}/oauth/callback` unless implementation evidence shows vanilla integration-route compatibility is needed for the Rust plugin bridge. If kept, document it as an internal route-shape difference with equivalent user-facing behavior.
- Add targeted tests for method-list serialization and TUI auth-flow state selection where practical. At minimum, cover that OpenAI auth methods are not collapsed to the first method and that auto OAuth does not require a code value.

## Non-goals

- Changing the Rust product's default model/provider.
- Replacing the already established `Settings > Provider` as the authoritative setup surface.
- Speculative Codex transport changes unrelated to auth parity.
- Broad auth parity for every provider; this is specifically OpenAI/Codex auth parity needed by `START-029`.

## Acceptance Criteria

- Vanilla OpenAI/Codex auth options are documented from fresh `dev` evidence with code-path references.
- Rust exposes the same OpenAI/Codex auth option set from the provider connection/setup flow, or every deviation is explicitly approved and recorded.
- The OpenAI provider connection UI offers exactly the vanilla-supported options currently relevant to Codex: `ChatGPT Pro/Plus (browser)`, `ChatGPT Pro/Plus (headless)`, and `Manually enter API Key`.
- ChatGPT Plus/Pro headless login works end-to-end in Rust and stores auth where the OpenAI/Codex runtime path can use it.
- ChatGPT Plus/Pro browser login works end-to-end in Rust or is explicitly documented as intentionally unavailable with a tracked reason approved before `START-029` is completed.
- Manual API key entry remains available and verified, but is not the only accepted auth path.
- The Rust TUI no longer hardcodes OpenAI OAuth method index `0` as the only login path.
- OAuth `method: "auto"` flows can complete without forcing the user through the manual code-entry input path.
- OAuth-authenticated Codex/OpenAI runtime requests do not require `OPENAI_API_KEY` when a saved ChatGPT Plus/Pro OAuth credential exists.
- `START-029` links to this gate and treats it as a prerequisite for completion.
- `docs/provider-setup.md` documents the final OpenAI/Codex auth option set and any intentional deviations from vanilla.

## Recommended Verification

- Compare vanilla provider connect/auth UI and server/plugin auth paths from fresh `dev`.
- Run the Rust TUI and confirm `Settings > Provider` exposes all required OpenAI/Codex auth options.
- Complete ChatGPT Plus/Pro headless auth and verify the saved auth is visible through `GET /auth/openai` as OAuth auth.
- Start a Codex/OpenAI request using headless auth with `OPENAI_API_KEY` unset for the launched Rust process and confirm the runtime path uses the saved OAuth credential.
- Complete or smoke-test manual API key entry and verify `GET /auth/openai` reports API auth and the OpenAI runtime path still works.
- Complete ChatGPT Plus/Pro browser auth if implemented and verify persisted auth is reused.
- Run targeted server/TUI tests for auth method listing, selected method index, and auto-vs-code callback behavior.
- Run `ort-build` before live TUI verification so `ort` uses the latest binary.

## Related Items

- `START-029` Set up OpenAI Codex 5.5 compatibility
- `START-015` Mirror OpenAI auth configuration in settings
- `START-027` Unify provider setup into one authoritative user path
- `START-030` Match vanilla OpenAI model retrieval parity
- `START-031` Auto-refresh the models.dev catalog for standing OpenAI parity

## Ready To Refine Checklist

- [x] Fresh vanilla `dev` auth reference paths are identified.
- [x] Required auth options are confirmed from code, not memory.
- [x] Rust current auth surfaces are mapped against vanilla.
- [x] Missing paths are in this card's implementation scope.
- [x] `START-029` explicitly records this card as a completion gate.

## Ready For Implementation

This card is ready for implementation. The implementer should start by making the Rust TUI/provider auth surface method-aware, then add correct `auto` OAuth completion handling, then verify the ChatGPT Plus/Pro headless path because that is the user's primary workflow.

## Implementation Notes - 2026-09-23

Bundled plugin (`crates/opencode-plugin/builtin/codex-auth.ts`) now mirrors vanilla's three OpenAI auth methods:

- `ChatGPT Pro/Plus (browser)` (`type: "oauth"`, `method: "auto"`) with PKCE (`node:crypto`) and a localhost callback server on port `1455`.
- `ChatGPT Pro/Plus (headless)` (`type: "oauth"`, `method: "auto"`) using the OpenAI device authorization endpoints and polling, returning the vanilla `https://auth.openai.com/codex/device` URL plus user code instructions.
- `Manually enter API Key` (`type: "api"`).

For OAuth auth, `loader(getAuth)` returns the dummy key plus a custom fetch that strips the existing authorization header, refreshes expired access tokens, injects `Authorization` and `ChatGPT-Account-Id`, and rewrites `/v1/responses` and `/chat/completions` requests to `https://chatgpt.com/backend-api/codex/responses` (with the residency header when present).

Plugin host / bridge changes:

- `auth.load` now forwards the stored credential to `loader(getAuth)` (`crates/opencode-plugin/host/plugin-host.ts`, `subprocess/client.rs`, `subprocess/auth.rs`).
- Server bootstrap and `plugin_auth_load` pass the stored `AuthInfo`; the bootstrap loop no longer overwrites a stored OAuth credential with the plugin's placeholder key (`crates/opencode-server/src/server.rs`, `routes.rs`).

Rust TUI/provider changes:

- `/provider/auth` now returns `index` and `type` alongside `name`/`description` (`crates/opencode-server/src/routes.rs`).
- `ProviderAuthMethodInfo` carries `index` and `method_type` with `is_api`/`is_oauth` helpers (`crates/opencode-tui/src/api.rs`).
- `Settings > Provider` shows a method chooser for OpenAI when more than one method is reported, instead of hardcoding method index `0` (`settings.rs`, `app/app.rs`).
- Auto OAuth methods complete without a pasted code; `oauth_requires_code()` gates the input requirement, and Enter on an auto method calls the callback with `None` (`settings.rs`, `app/app.rs`).

Documentation: `docs/provider-setup.md` documents the three options, the route-shape deviation, and the in-session token-refresh limitation.

Tests: `crates/opencode-tui/src/components/settings.rs` covers method-list propagation, chooser navigation, auto-vs-code callback behavior, and payload deserialization. `cargo test -p opencode-plugin -p opencode-server -p opencode-tui` passes. Live ChatGPT headless/browser login still requires human QA with a real account.
## QA Handoff - 2026-09-23

- PR: https://github.com/cchris-p/opencode-modded-rust/pull/84 (base `development`).
- Branch `feature/START-032-codex-auth-parity` is checked out locally for verification.
- Human QA: run `ort-build` then `ort`, open `Settings > Provider`, select `openai`, press `l`, and confirm the chooser lists browser/headless/API key.
- Complete `ChatGPT Pro/Plus (headless)` against a real account, press Enter, and confirm `GET /auth/openai` reports `auth_type: "oauth"`.
- Run a Codex request with `OPENAI_API_KEY` unset and confirm the saved OAuth credential is used.
- Smoke-test manual API key entry and confirm `GET /auth/openai` reports `auth_type: "api"`.
- Browser login is implemented but only verify it if convenient; the headless path is the primary workflow.

## Merge Closeout - 2026-09-23

- Merged into `development` via PR #84 (merge commit `0382393`).
- Feature branch `feature/START-032-codex-auth-parity` deleted remotely and locally; local checkout is back on `development`.
- Remains in `qa`: no QA report is recorded yet. Promote to the completed lane only after live headless/browser login verification (per the QA Handoff section) or explicit user direction.

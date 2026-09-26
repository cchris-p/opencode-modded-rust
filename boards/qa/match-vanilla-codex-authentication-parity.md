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

## Reopened - 2026-09-26

Reopened from `qa` to `doing`. Live ChatGPT browser auth still fails after PR #84 merged, so the acceptance criteria ("ChatGPT Plus/Pro browser login works end-to-end") are not met.

### Reported failures (user, 2026-09-26)

1. Browser/Codex login, after entering the code in the browser, the TUI shows:

   `OpenAI auth failed: Failed to complete provider auth for openai: 400 Bad Request - {"error":{"message":"OAuth callback failed","type":"bad_request"}}`

2. After restarting, selecting `openai/gpt-5.5` and sending a prompt returns:

   `Provider error: Provider error: You have no credits remaining. Add credits to continue using the API at https://platform.openai.com/settings/organization/billing/.`

The user's read is that the provider is still using `OPENAI_API_KEY`, which is why browser OAuth needs to work.

### Evidence gathered

- The Rust auth store `~/Library/Application Support/opencode/data/auth.json` currently contains only `openai: {type: "api", key: "sk-proj-…"}` (164-char project key). Its mtime is 2026-09-23 04:14, predating the failed attempt, and there is no `type: "oauth"` entry. The browser OAuth flow did not persist anything.
- The visible error is generated in Rust, not by the plugin: `AuthError::OauthCallbackFailed` renders exactly `OAuth callback failed` (`crates/opencode-provider/src/auth.rs:177`), and `oauth_callback` maps any `AuthError` to `400 Bad Request` (`crates/opencode-server/src/routes.rs:4826-4829`).
- `ProviderAuth::callback` throws the real cause away with `map_err(|_| AuthError::OauthCallbackFailed)?` (`crates/opencode-server/src/oauth.rs:79-82`), returns the same error when the plugin result is not `type: "success"` (`oauth.rs:84-87`), and again when access/refresh are empty (`oauth.rs:123-125`). The plugin host's real string (`auth.callback failed: …`, `No pending auth callback`, `Token exchange failed: <status>`, `OAuth callback timeout`) never reaches the TUI.
- The `auth.callback` RPC is bounded by a fixed 30s subprocess timeout (`crates/opencode-plugin/src/subprocess/client.rs:181`, applied in `call()` at `client.rs:440-442`). Both OpenAI `method: "auto"` flows block inside this RPC until a human finishes: browser waits on the localhost:1455 callback (`codex-auth.ts:294-305`), headless polls the device endpoint at an ~8s interval (`codex-auth.ts:333-388`). A slow login exceeds 30s, becomes a timeout, and is collapsed into `OAuth callback failed`. The plugin's internal browser timeout is 5 minutes (`codex-auth.ts:265`), far longer than the RPC budget.
- Custom fetch (which rewrites `/v1/responses` to `https://chatgpt.com/backend-api/codex/responses` and injects the OAuth bearer) is registered only after a successful callback plus `auth.load` (`crates/opencode-server/src/routes.rs:4831-4851`, `crates/opencode-server/src/server.rs:86-110`). Because the callback failed, no proxy was registered and the OpenAI provider used its stored `sk-proj-…` API key (`bootstrap.rs:2854-2865`, `bootstrap.rs:2799-2810`). The `gpt-5.5` "no credits remaining" message is the standard API billing error, so failure 2 is a downstream symptom of failure 1, not a separate bug.
- The Rust product and vanilla OpenCode keep separate auth stores: `~/Library/Application Support/opencode/data/auth.json` (Rust) vs `~/.local/share/opencode/auth.json` (vanilla). Vanilla currently holds a valid `openai` OAuth credential (mtime 2026-09-24) that the Rust product does not read. Expected by design, but a source of "why is it still using the key" confusion.

### Root-cause assessment

- Confirmed: the Rust auth callback did not persist an OAuth credential, so the runtime used the stored API key and produced the "no credits" error.
- Primary hypothesis for the callback failure: the 30s `auth.callback` RPC timeout is shorter than the human-driven `auto` login, so the RPC times out and the real error is masked. This fits the absence of any persisted OAuth credential.
- Not-yet-excluded alternatives (cannot be distinguished while the underlying error is discarded): token exchange `400` from `https://auth.openai.com/oauth/token` (`codex-auth.ts:118-138`), the localhost:1455 redirect not reaching the plugin-host server, or `pendingAuthCallback` being cleared/overwritten between authorize and callback (a single global in `plugin-host.ts:168,499-533`).

### Planned Fixes

These are the implementation scope for the reopen. Fix 3 is guarded and only applies if the real error survives Fixes 1-2.

#### Fix 1 - Preserve and surface the real auth error

The TUI already prints whatever the server returns (`crates/opencode-tui/src/api.rs:776-781`, `crates/opencode-tui/src/app/app.rs:3004-3008`), and the server already serializes `AuthError` text (`crates/opencode-server/src/routes.rs:4826-4829`). The loss happens inside `ProviderAuth`.

- `crates/opencode-provider/src/auth.rs:177`: replace the unit `OauthCallbackFailed` with a detail-carrying variant, e.g. `OauthCallbackFailed { message: String }` (or add `OauthCallbackRejected(String)` / `OauthCallbackTimedOut`). Only `crates/opencode-server/src/oauth.rs` consumes it, so blast radius is small.
- `crates/opencode-server/src/oauth.rs:55,82`: stop using `.map_err(|_| ...)`; map `PluginAuthError` to the detail-carrying error, preserving the plugin message (`auth.callback failed: …`, `No pending auth callback`, `Token exchange failed: <status>`, `OAuth callback timeout`).
- `crates/opencode-server/src/oauth.rs:84-87`: when the plugin result type is not `success`, include the returned `type` (headless returns `{type: "failed"}`) in the error text.
- `crates/opencode-server/src/oauth.rs:123-125`: include a specific "empty access/refresh from plugin" reason.
- Tests: add server-side tests that a bridge error message and a non-`success` plugin result are preserved in the returned error; keep existing `opencode-provider` auth tests compiling against the changed variant.

#### Fix 2 - Give `auto` OAuth flows a human-scale timeout at both layers

Both 30s bounds are shorter than a real login and must be raised for the auth RPC only; the plugin's own browser budget is 5 minutes (`codex-auth.ts:265`) and headless polls until the device is approved.

- `crates/opencode-plugin/src/subprocess/client.rs`: add `call_with_timeout(method, params, timeout)`; keep the 30s default (`client.rs:181`) for normal hooks, and use a dedicated auth timeout (at least the plugin's 5-minute browser budget plus margin, e.g. 10 minutes) for `auth_authorize` (`client.rs:231-241`) and `auth_callback` (`client.rs:244-247`).
- `crates/opencode-tui/src/api.rs:405-406`: the TUI reqwest client is also fixed at 30s. Apply a matching per-request `.timeout(...)` override in `start_provider_oauth` (`api.rs:734-758`) and `complete_provider_oauth` (`api.rs:760-785`) instead of the client default.
- Document the TUI blocking concern: `complete_provider_oauth` runs synchronously on the event loop, so a multi-minute wait freezes input and prevents Esc-cancel. Minimal scope is a bounded long timeout; preferred is to await the callback off the event loop with a "waiting for browser/device" state and a cancel path. Record whichever is implemented.
- Tests: unit-test that `call_with_timeout` honors the override; add/adjust TUI API tests if a timeout override helper is introduced.

#### Fix 3 (guarded) - Only re-address transport if a real error recurs

- After Fixes 1-2, re-run the browser and headless flows with `OPENAI_API_KEY` unset for the launched `ort` process and plugin logging on.
- If the surfaced error is a token-exchange `400` (`codex-auth.ts:118-138`), a localhost:1455 redirect that never reaches the plugin host, or a cleared/overwritten `pendingAuthCallback` (`plugin-host.ts:168,499-533`), fix that specific path using the now-visible error.
- Do not change transport or Codex endpoint behavior speculatively while the error is still masked.

### Verification Plan

1. Rebuild with `ort-build`; launch `ort` from the intended workspace with `OPENAI_API_KEY` unset.
2. `Settings > Provider` -> `openai` -> `l` -> choose browser and headless in turn; confirm any failure now shows the real plugin message, not generic `OAuth callback failed`.
3. Confirm a successful callback writes an `oauth` entry to `~/Library/Application Support/opencode/data/auth.json` and `GET /auth/openai` reports `auth_type: "oauth"`.
4. Confirm the custom fetch proxy is active (request reaches `chatgpt.com/backend-api/codex/responses`) and that `openai/gpt-5.5` runs without the API-key "no credits remaining" error.
5. Run `cargo test -p opencode-plugin -p opencode-server -p opencode-tui` for the touched crates.

## Implementation Notes - 2026-09-26

Branch `bug/START-032-codex-auth-callback-failure`, PR: https://github.com/cchris-p/opencode-modded-rust/pull/116 (base `development`).

Fix 1 - surface the real auth error:

- `AuthError::OauthCallbackFailed` now carries a `String`, and a new `AuthError::OauthAuthorizeFailed(String)` covers the authorize path (`crates/opencode-provider/src/auth.rs:177-181`).
- `ProviderAuth::authorize`/`callback` no longer discard the `PluginAuthError`; the plugin/bridge message (`plugin RPC error (...)`, `plugin response timeout`, `auth.callback failed: …`, `No pending auth callback`) is now included in the 400 response (`crates/opencode-server/src/oauth.rs:52-55,79-85`).
- Non-`success` plugin results report the returned type or "no auth result type"; empty tokens report "plugin returned no access or refresh token".
- Extracted `parse_callback_auth` so these paths are unit testable without a live plugin host; 5 tests added in `crates/opencode-server/src/oauth.rs`.

Fix 2 - human-scale auth timeouts:

- Plugin RPC: added `call_with_timeout` and a dedicated `AUTH_FLOW_TIMEOUT = 330s` for `auth.authorize`/`auth.callback`; every other hook keeps the 30s default (`crates/opencode-plugin/src/subprocess/client.rs:54-60,247-257,443-465`).
- TUI HTTP: a matching `AUTH_FLOW_TIMEOUT = 360s` is applied to `start_provider_oauth`/`complete_provider_oauth` so the 30s client default no longer cuts off headless device polling (`crates/opencode-tui/src/api.rs:9-10,747,775`). The TUI budget is intentionally larger than the RPC budget so the server's detailed timeout error reaches the UI first.

Known limitation (tracked, not fixed here): `complete_provider_oauth` still runs synchronously on the TUI event loop, so pressing Enter before completing the browser/device step freezes input until the flow resolves or the 360s timeout elapses. The intended UX ("complete the flow, then press Enter") avoids it. Moving the `auto` callback off the event loop with a cancel path is the preferred follow-up.

Verification:

- `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo check -p opencode-provider -p opencode-plugin -p opencode-server -p opencode-tui` passed.
- `cargo test -p opencode-provider -p opencode-plugin -p opencode-server -p opencode-tui` passed: provider 103 + 7 integration, plugin 3, server 64 + 3 route, tui 156.
- `cargo fmt --all --check` passed.
- Disk prerequisite: the build volume was full; the shared worktree cache (`~/worktrees/opencode-modded-rust/.shared-target`) was removed to complete the test run.
- Live ChatGPT browser/headless login still requires human QA with a real account. With Fix 1 in place, a further failure now shows the actual plugin error instead of generic `OAuth callback failed`.

QA handoff:

- Run `ort-build` then `ort`, open `Settings > Provider`, select `openai`, press `l`, and try the browser and/or headless method.
- If auth fails, the toast/`GET /auth/openai` response should now include the real cause (token exchange status, timeout, no pending callback, etc.).
- On success, confirm an `oauth` entry appears in `~/Library/Application Support/opencode/data/auth.json` and `gpt-5.5` runs without the API-key "no credits" error.

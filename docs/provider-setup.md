## Provider Setup

`Settings > Provider` is the authoritative V1 setup path.

Use that screen to:

- choose the active provider
- choose the active model
- review the effective auth state
- review and edit the Ollama host when using local models

Behavior outside that screen is secondary:

- project or global config can still override provider behavior
- environment variables can still override provider behavior
- shell helpers such as local Ollama launch aliases are optional conveniences, not the primary setup flow

The provider screen shows the effective provider, effective model, selection source, auth source, and Ollama host source so overrides are visible instead of implicit. The selection source distinguishes the product default from a persisted manual selection and from config or environment overrides.

When you press `Enter` on a highlighted model in `Settings > Provider`, the selection is written to the project-local runtime config (`<workspace>/opencode.json`, which this repo gitignores) and becomes the default for new sessions and future runs. The same persistence applies when a model is chosen from the model-cycle dialog. Press `d` in `Settings > Provider` to clear the saved selection so the product default (`deepseek/deepseek-flash`) is effective again.

Manual selection is **project-level**: it is stored in the workspace's own `opencode.json` and applies to that workspace, not to every workspace on the machine. The product default only applies while no manual selection exists, or after a reset.

When Ollama is highlighted, press `u` to edit the Ollama host/base URL from the same screen.

## OpenAI / Codex authentication

Selecting the `openai` provider and pressing `l` opens an auth-method chooser when the plugin reports more than one method. The Rust product mirrors vanilla OpenCode's OpenAI/Codex auth surface:

- `ChatGPT Pro/Plus (browser)` — PKCE browser login. The TUI shows the authorization URL, the plugin starts a localhost callback server on port `1455`, and the login completes without pasting a code.
- `ChatGPT Pro/Plus (headless)` — OpenAI device authorization. The TUI shows the device URL plus the user code; complete it in the browser and press `Enter` to finish polling.
- `Manually enter API Key` — the existing API key input path. Press `a` as a shortcut.

OAuth methods report `method: "auto"`, so completing them does not require a pasted code. When a saved OAuth credential exists, OpenAI/Codex requests are routed through the plugin custom fetch, which rewrites responses/chat-completions traffic to the Codex backend and injects the ChatGPT access token and account id. This path does not require `OPENAI_API_KEY`.

Intentional deviations from vanilla OpenCode:

- The Rust product keeps its existing internal `/provider/{id}/oauth/authorize` and `/provider/{id}/oauth/callback` routes instead of vanilla's newer `/api/integration/*` endpoints. User-facing behavior and persisted auth semantics are equivalent.
- Refreshed OAuth access tokens are refreshed in the plugin host for the lifetime of the session; the refreshed token is not written back to the stored credential. The stored refresh token remains valid and is used on the next session.

Verify auth state with `GET /auth/openai`: an OAuth login reports `auth_type: "oauth"`, and an API key reports `auth_type: "api"`.

## Model catalog freshness

Provider and model listings are backed by the vanilla `models.dev` catalog (`https://models.opencode.ai/api.json`), cached at `dirs::cache_dir()/opencode/models.json`. The cache is not frozen after the first fetch:

- A cache older than `MODELS_DEV_TTL` (5 minutes, matching vanilla's `Duration.minutes(5)`) is refetched on the next CLI or server load without requiring manual file deletion.
- `opencode models --refresh` forces an immediate refetch of the catalog and then rebuilds the provider registry from the refreshed file.
- A running server schedules a catalog refresh every `MODELS_DEV_REFRESH_INTERVAL` (~60 minutes, matching vanilla's `Schedule.spaced("60 minutes")`) and rebuilds its provider registry, so long-running sessions pick up new models without a restart.
- A failed fetch is non-fatal: the product keeps serving the existing cached catalog rather than dropping models.

`scripts/compare-openai-model-parity.sh` seeds a freshly written cache, so the parity check remains deterministic and does not depend on the network at comparison time.


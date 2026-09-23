---
id: "BUG-034"
title: "Default DeepSeek model is missing from provider registry"
priority: "P1"
type: "bug"
area: "BUG"
spec: "invariants/providers.md"
status: "doing"
created: "2026-09-23"
---

# Default DeepSeek model is missing from provider registry

## Summary

The Rust product still resolves its effective default model to `deepseek/deepseek-v4-flash`, but the live provider registry for the `deepseek` provider no longer contains `deepseek-v4-flash`. A first prompt then fails before prompt acceptance with a 400 model-resolution error.

Observed on the live Rust `ort` server at `127.0.0.1:3187`:

- `/config/providers` reported `setup.effective_model: "deepseek/deepseek-v4-flash"`.
- The same response listed `deepseek` models including `deepseek-v4-pro` and `deepseek-flash`, but not `deepseek-v4-flash`.
- A TUI prompt failed with:
  `Failed to send prompt to http://127.0.0.1:3187/session/ses_fd6cf4ebaa6e4123a381579f83e66847/prompt: 400 Bad Request - {"error":{"message":"Model `deepseek-v4-flash` not found for provider `deepseek`","type":"bad_request"}}`
- Direct reproduction against a throwaway session returned the same server body:
  `{"error":{"message":"Model `deepseek-v4-flash` not found for provider `deepseek`","type":"bad_request"}}`.

## Scope

- Make the product-owned default model/provider resolve to a model that is present in the runtime provider registry.
- Preserve the intended product default behavior: global vanilla OpenCode config must not dictate this product's default.
- Keep provider listing, TUI model selection, and prompt resolution consistent for DeepSeek defaults.

## Non-goals

- Redesigning the full provider/model catalog refresh policy; `START-031` owns standing catalog refresh.
- Changing OpenAI/Codex catalog parity behavior.
- Replacing the TUI error presentation; `BUG-035` owns making prompt-send errors visible on Info.

## Acceptance Criteria

- Fresh `ort-build` + `ort` with no workspace override selects an effective DeepSeek model that exists in `/config/providers`.
- Sending the first prompt no longer fails with `Model `deepseek-v4-flash` not found for provider `deepseek``.
- `/config/providers` does not report an effective model absent from its provider's model list.
- Any intentionally changed product default is reflected in repo docs/invariants that name the default model.

## Verification

- Query `/config/providers` on a fresh Rust server and confirm `setup.effective_model` appears in the listed provider models.
- Send a first TUI prompt from Home and confirm the prompt is accepted rather than rejected at model resolution.
- Run relevant provider/config tests when Rust tooling is available.

## Related Items

- `BUG-010` Repo default model and provider ignored due to config precedence.
- `BUG-033` First prompt failure leaves an exportable empty new session.
- `START-031` Auto-refresh the models.dev catalog for standing OpenAI parity.
- `FEAT-013` Model capability gating and deprecated model surfacing.

## Investigation - 2026-09-23

Regression source: `b0ed403` (`fix(provider): restore OpenAI model listing parity (START-030)`, PR #76).

What changed:

- Before `b0ed403`, server provider bootstrap called `load_models_dev_cache()` and fell back to the bundled V1 catalog when `~/.cache/opencode/models.json` was absent or invalid.
- The bundled catalog contains `deepseek-v4-flash` and marks it active (`crates/opencode-provider/src/bootstrap.rs`). That matched the product-owned default `deepseek/deepseek-v4-flash`.
- `b0ed403` added `opencode_provider::ensure_models_dev_cache().await` to server startup and provider refresh (`crates/opencode-server/src/server.rs`) and CLI provider setup. That fetches `https://models.opencode.ai/api.json` on cache miss before bootstrapping.
- The canonical/cache DeepSeek entry marks `deepseek-v4-flash` as `status: "deprecated"`; current active DeepSeek models include `deepseek-v4-pro` and `deepseek-flash`.
- Provider bootstrap removes deprecated models unconditionally (`ProviderBootstrapState::init`, `model.status == "deprecated"`).

Net effect:

- START-030 fixed OpenAI catalog freshness by making the live canonical catalog available at startup, but it also made DeepSeek default resolution use canonical DeepSeek status data instead of the bundled fallback.
- The product default stayed `deepseek/deepseek-v4-flash`, but the runtime provider registry now filters that model out as deprecated.
- Prompt submission then fails before `accept_prompt` with `Model `deepseek-v4-flash` not found for provider `deepseek``.

This was not introduced by deleting the bundled DeepSeek model. The bundled fallback still has `deepseek-v4-flash`; the regression is the new startup cache fetch plus existing deprecated-model filtering.

## Implementation Notes - 2026-09-23

- Changed the Rust product default from `deepseek/deepseek-v4-flash` to `deepseek/deepseek-flash` in `crates/opencode-config/src/loader.rs`.
- Updated the repo workspace override in `opencode.jsonc` so local `ort`/`opencode` launches from this repo do not keep selecting the stale model.
- Updated the bundled DeepSeek fallback catalog and TUI fallback model list from `deepseek-v4-flash` to `deepseek-flash` so provider listing and selection stay consistent even before a remote model cache is available.
- Updated `AGENTS.md` and `invariants/providers.md` to name the new product-owned default.

Verification performed:

- `cargo fmt --all`
- `cargo test -p opencode-provider bundled_v1_catalog_includes_ollama_local_path`
- `cargo test -p opencode-config product_default_model_applies_without_workspace_config`
- `cargo test -p opencode-config workspace_config_overrides_product_default_model`
- `cargo test -p opencode-config model_only_env_content_from_standard_home_config_dir_does_not_replace_product_default`
- `cargo build -p opencode-cli`
- `./target/debug/opencode config` reported `Default model: deepseek/deepseek-flash`.
- Fresh `./target/debug/opencode serve --hostname 127.0.0.1 --port 3199` `/config/providers` check reported `effective_model=deepseek/deepseek-flash`, `present=True`, and DeepSeek models `deepseek-flash,deepseek-v4-pro`.
- `./target/debug/opencode run "Reply with exactly OK"` returned `Assistant: OK`, confirming the first prompt no longer fails at model resolution.

---
id: "START-019"
title: "Add native Ollama support for the local-model-first V1 path"
priority: "P1"
type: "feature"
area: "START"
spec: "wiki/v1.md"
status: "done"
created: "2026-08-29"
---

# Add native Ollama support for the local-model-first V1 path

## Summary

Add native Ollama support as the explicit local-model-first execution path for the Rust product's V1 runtime, so a user can run against a local or remote Ollama `/v1` endpoint without relying on the stock OpenCode shell wrapper behavior.

## Why this exists

`START-004` found that the current provider abstraction is reusable, but the visible product shape still reads as multi-provider breadth rather than a clearly local-model-first V1 runtime.

Current Rust implementation evidence shows that:

- the visible V1 provider surface is intentionally limited to `openai`, `anthropic`, `deepseek`, and `openrouter`
- there is no explicit native `ollama` provider entry in the Rust provider/catalog/settings surface
- the runtime already has generic OpenAI-compatible plumbing
- the current custom-provider fallback requires a non-empty API key, which blocks the user's existing Ollama setup pattern that works in stock OpenCode without an API key

This means Ollama is not part of the curated four-provider V1 breadth surface, but it is part of the V1 depth path: the required local-model execution route for serious daily-driver use.

## Scope

- Add explicit native `ollama` provider support in the Rust config/bootstrap/runtime flow.
- Support Ollama through its OpenAI-compatible `/v1` endpoint for both local and remote hosts.
- Allow `ollama/<model>` model references to resolve end-to-end through config loading, provider bootstrap, and runtime execution.
- Make the Rust implementation accept an Ollama configuration that does not require a real API key.
- Expose Ollama clearly enough in the product that the local-model path is intentional rather than hidden shell magic.
- Make Ollama the product default only when the user has not explicitly configured a provider or model.
- Ship `ollama/qwen3:30b` as the built-in default model for that path while allowing a normal config override.
- Default the Ollama endpoint to `http://127.0.0.1:11434/v1` when the user has not configured another host.
- Expose Ollama as an intentional selectable provider in the TUI/settings surface.
- Update provider-planning references so the distinction is clear: `openai`, `anthropic`, `deepseek`, and `openrouter` remain the curated V1 breadth surface, while `ollama` is the explicit V1 local-model path and should not be treated as an accidental out-of-scope provider.

## Non-goals

- Full provider parity with stock OpenCode.
- Broad onboarding or first-run setup UX.
- Supporting every OpenAI-compatible local endpoint under a polished generic custom-provider UX.
- Reworking the V1 curated remote provider list.
- Expanding this story into broader local-model evaluation, retrieval, or runtime-architecture work.

## Implementation expectations

- The provider layer either has a first-class `ollama` provider id or an equivalent native path that is intentionally named and documented as Ollama support.
- The config path supports a provider entry equivalent to the user's current stock OpenCode shell-generated config, including provider id `ollama`, configurable `baseURL`, any needed OpenAI-compatible transport metadata, and model ids like `ollama/qwen3:30b`.
- The runtime must not fail solely because no API key is present for Ollama.
- If the implementation uses a placeholder or optional API key internally, that behavior must be encapsulated in code and not pushed onto the end user as a manual workaround.
- Default means the following in this story: if no provider or model is explicitly configured by the user, runtime/provider selection should resolve to Ollama automatically instead of one of the remote curated providers.
- Default does not mean overriding an existing user-selected provider, user-selected model, or explicit config file setting.
- The built-in default model for that default path is `ollama/qwen3:30b`, but the user can override it through normal config.
- The built-in default endpoint for that default path is `http://127.0.0.1:11434/v1`, but the user can override it through normal config.
- The TUI/settings surface should reflect the intentional Ollama path as a visible selectable provider, not only a hidden runtime fallback.

## Done when

- A user can point Rust OpenCode at an Ollama `/v1` endpoint and run a session using an `ollama/<model>` model reference.
- The supported config path does not require the stock OpenCode `opencode-use-ollama-*` shell wrapper to manufacture compatibility.
- The provider bootstrap/runtime path accepts Ollama without requiring a real API key.
- A user with no explicit provider/model configuration gets Ollama by default.
- That default resolves to `ollama/qwen3:30b` and `http://127.0.0.1:11434/v1` unless overridden by normal config.
- Existing explicit provider or model configuration continues to win over the built-in Ollama default.
- The TUI/settings provider surface shows Ollama as an intentional supported option.
- The resulting implementation is documented in the board notes and clearly positioned relative to the curated four-provider V1 surface.
- Any intentionally deferred UI/catalog polish is captured as an explicit follow-up item rather than left implicit.

## Recommended verification

- Configure the app with an inline or file-based provider entry for `ollama` using a local endpoint such as `http://127.0.0.1:11434/v1`.
- Verify a model reference like `ollama/qwen3:30b` resolves through provider selection and runtime execution.
- Verify the session starts and streams correctly without a real API key configured for Ollama.
- Remove explicit provider/model configuration and verify the app defaults to `ollama/qwen3:30b`.
- Override the model or base URL in normal config and verify the override wins over the built-in Ollama defaults.
- Verify an explicitly configured non-Ollama provider still wins over the default path.
- Verify the implementation does not regress the existing curated V1 providers.
- Verify the TUI/settings surface shows Ollama as a supported provider without implying full onboarding support.

## Notes

- Use the existing OpenAI-compatible plumbing where practical; do not build a bespoke Ollama transport if the current HTTP compatibility route is sufficient.
- Prefer the smallest implementation that makes Ollama support native and explicit.
- Keep the distinction between V1 breadth and V1 depth clear: Ollama is not the fifth general-purpose catalog-expansion provider, it is the required local-model execution path.

## Dev Notes

- Added a native `ollama` provider bootstrap path on top of the existing OpenAI-compatible transport instead of inventing a separate Ollama HTTP client.
- Added a bundled Ollama provider/model entry so the local-model path is present even when the repo is using bundled fallback catalog data.
- Added a default Ollama base URL of `http://127.0.0.1:11434/v1`, with normalization for bare `OLLAMA_HOST` values and config-supplied host values.
- Allowed Ollama runtime creation without a user-supplied API key by encapsulating the compatibility placeholder key in code.
- Updated server and TUI provider selection so Ollama is visible and becomes the default only when no explicit provider/model is configured.

## Verification

- `cargo fmt`
- `cargo check -p opencode-provider -p opencode-server -p opencode-tui`
- `cargo test -p opencode-provider ollama -- --nocapture`

## PR

- https://github.com/cchris-p/opencode-modded-rust/pull/18

## Completion

- Merged into `development` on 2026-08-31.

## Related Items

- `START-004` Assess current Rust state
- `START-009` Port provider setup from OpenCode
- `START-012` Refresh provider and model catalog
- `START-015` Mirror OpenAI auth configuration in settings

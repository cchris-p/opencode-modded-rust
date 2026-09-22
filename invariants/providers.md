# Provider Invariants

- OpenRouter is not disabled as a provider unless an explicit board item implements and documents a temporary visibility exception.
- OpenRouter is part of the curated remote provider surface alongside OpenAI, Anthropic, and DeepSeek, but temporary removal from normal provider-selection lists is allowed when tracked by a board item and implemented without deleting the provider.
- The local-model path is Ollama; Ollama is not a replacement for OpenRouter or other curated remote providers.
- Provider setup must remain visible through `Settings > Provider`, including the effective provider, effective model, auth state, and Ollama host state.
- OpenRouter auth is keyed by `OPENROUTER_API_KEY`.
- OpenRouter may be selected explicitly by config, environment, or `Settings > Provider`.
- If OpenRouter is temporarily hidden from normal provider-selection lists, explicit `openrouter/...` configuration behavior must remain intentional and documented by the implementing board item.
- OpenRouter must not become the implicit product default unless an explicit project invariant changes that policy.
- The product-owned default model/provider is independent of vanilla OpenCode's global OpenRouter configuration.
- OpenAI model retrieval must match vanilla OpenCode's current `models.dev`-backed provider catalog for the in-scope OpenAI surface. The Rust product must not ship a stale, manually divergent OpenAI model list when vanilla would expose newer OpenAI or Codex-family models through provider/model listing.
- Any intentional OpenAI model exclusion, rename, status filter, or auth-scope limitation must be explicit in code and board/docs. Hidden divergence from vanilla model retrieval is not allowed.

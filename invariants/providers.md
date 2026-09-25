# Provider Invariants

- OpenRouter is not disabled as a provider unless an explicit board item implements and documents a temporary visibility exception.
- OpenRouter is part of the curated remote provider surface alongside OpenAI, Anthropic, and DeepSeek, but temporary removal from normal provider-selection lists is allowed when tracked by a board item and implemented without deleting the provider.
- The local-model path is Ollama; Ollama is not a replacement for OpenRouter or other curated remote providers.
- Provider setup must remain visible through `Settings > Provider`, including the effective provider, effective model, auth state, and Ollama host state.
- OpenRouter auth is keyed by `OPENROUTER_API_KEY`.
- OpenRouter may be selected explicitly by config, environment, or `Settings > Provider`.
- If OpenRouter is temporarily hidden from normal provider-selection lists, explicit `openrouter/...` configuration behavior must remain intentional and documented by the implementing board item.
- OpenRouter must not become the implicit product default unless an explicit project invariant changes that policy.
- The product-owned default model/provider is `deepseek/deepseek-flash` and is independent of vanilla OpenCode's global OpenRouter configuration.
- The ScopeMux structural retrieval provider is scoped to the local Qwen-via-Ollama model: it activates only when the effective provider is `ollama` and the effective model id starts with `qwen`. Every other model uses the generic repository-local provider, which remains the default and fallback.
- ScopeMux remains evidence-only and does not gain runtime authority over task state, lifecycle, verification, or review when active.
- OpenAI model retrieval must match vanilla OpenCode's current `models.dev`-backed provider catalog for the in-scope OpenAI surface. The Rust product must not ship a stale, manually divergent OpenAI model list when vanilla would expose newer OpenAI or Codex-family models through provider/model listing.
- "Match" means the same catalog source vanilla uses (`https://models.opencode.ai/api.json`), the same status filtering (drop deprecated; drop alpha unless experimental models are enabled), and the same `experimental.modes` expansion into `<model-id>-<mode>` entries. `scripts/compare-openai-model-parity.sh` is the parity check and must pass.
- Any intentional OpenAI model exclusion, rename, status filter, or auth-scope limitation must be explicit in code and board/docs. Hidden divergence from vanilla model retrieval is not allowed.
- The `models.dev` catalog cache must follow a freshness policy equivalent to vanilla OpenCode rather than freezing the first successfully parsed file. The Rust product uses vanilla's 5-minute TTL (`MODELS_DEV_TTL`), forces a refetch for `opencode models --refresh`, and schedules a ~60-minute server refresh (`MODELS_DEV_REFRESH_INTERVAL`). Fetch failures must keep serving the existing cache instead of emptying the catalog.

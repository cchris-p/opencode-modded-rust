---
id: "CLI-015"
title: "Native oco providers, models, and prompt shaping"
priority: "P1"
type: "feature"
area: "CLI"
spec: "docs/opencode-provider.md; docs/opencode-cli.md; ~/standards/handoffs/H-014-ort-native-oco-handoff.md"
status: "todo"
created: "2026-11-07"
updated: "2026-11-07"
predecessor: "CLI-014"
---

# Native oco providers, models, and prompt shaping

## Summary

Implement the `oco` provider layer and model listing on top of ORT's existing
`opencode-provider` crate, and finish the prompt-shaping options that don't
require a separate subcommand.

## Provider Requirements

Providers that must work (owner's actual usage):

- `ollama` (native `/api/chat` and OpenAI-compatible `/v1/chat/completions`).
- `openai` (including a custom `OCO_API_URL`).
- `anthropic`.
- `gemini`.
- OpenAI-compatible endpoints for DeepSeek/OpenRouter via `OCO_AI_PROVIDER=openai`
  and `OCO_API_URL` (the standards shell config uses this shape for DeepSeek).

Reuse `opencode-provider` where it fits:

- `Provider` trait: `chat(ChatRequest) -> ChatResponse`.
- `OpenAIProvider::new` / `OpenAIProvider::new_with_base_url`.
- `AnthropicProvider`, `GoogleProvider`, `DeepSeekProvider`.
- `create_registry_from_env` for key discovery, but construct explicit providers
  when `OCO_API_URL` or `OCO_MODEL` must win.

Rules:

- Honor `OCO_AI_PROVIDER`, `OCO_MODEL`, `OCO_API_URL`, `OCO_API_KEY`,
  `OCO_API_CUSTOM_HEADERS`.
- Keep an explicit connect/read timeout; surface HTTP status and body fragment
  on failure.
- Never read an API key from argv or log it.
- Map provider IDs the way upstream does (`deepseek` can be OpenAI-compatible;
  Ollama's base URL may already include `/api/chat` in legacy configs — accept
  both and normalize).

## `oco models`

- `oco models`, `oco models --refresh`, `oco models --provider <id>`.
- Query provider `/models` where supported; fall back to a static list.
- Cache in `~/.cache/ort/oco-models.json` with a 7-day TTL (match upstream's
  cache behavior in spirit).
- Never require network for `--help` or commit generation.

## Prompt Shaping

- `OCO_DESCRIPTION` (postface description).
- `OCO_EMOJI` and `--fgm` (short set vs full GitMoji set), including
  `OCO_EMOJI_POSITION_BEFORE_DESCRIPTION`.
- `OCO_OMIT_SCOPE`.
- `OCO_ONE_LINE_COMMIT`.
- `OCO_LANGUAGE` as a prompt instruction in v1.
- `OCO_TOKENS_MAX_OUTPUT` as the model's max output tokens.

## Acceptance Criteria

- [ ] Ollama, OpenAI, Anthropic, and Gemini each generate a commit message with a
      live or mock endpoint; tests use a local mock so they run offline.
- [ ] `OCO_API_URL` overrides the provider base URL for OpenAI-compatible
      endpoints.
- [ ] `oco models` lists models for at least Ollama and OpenAI, respects
      `--provider`, and caches.
- [ ] `--fgm`, `OCO_EMOJI`, `OCO_DESCRIPTION`, `OCO_ONE_LINE_COMMIT`, and
      `OCO_LANGUAGE` are covered by prompt snapshot tests.
- [ ] Errors include provider, model, URL host (not key), and HTTP status where
      applicable.

## Related

- `CLI-014` — core config/commit/prompt engine.
- `CLI-016` — hook and commitlint.
- `INFRA-016` — artifact packaging.
- Standards `H-014` / `STD-155`.

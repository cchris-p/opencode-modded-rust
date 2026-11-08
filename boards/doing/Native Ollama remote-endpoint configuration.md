---
id: "FEAT-070"
title: "Native Ollama remote-endpoint configuration"
priority: "P1"
type: "feature"
area: "FEAT"
spec: "docs/provider-setup.md"
status: "doing"
created: "2026-11-08"
updated: "2026-11-08"
---

# Native Ollama remote-endpoint configuration

## Summary

Move remote-Ollama endpoint and model selection out of the standards shell
wrapper (`ortq`, which probes the endpoint, exports `OLLAMA_HOST`, and pins
`--model` from shell state) and into ORT itself. The product must resolve one
answer at startup and never depend on shell state or an installed `ollama`
binary.

## Config contract (high to low precedence)

1. CLI `--model` (`ollama/<tag>`)
2. `OLLAMA_REMOTE_HOST` / `OLLAMA_REMOTE_MODEL`
3. `~/.config/opencode/ollama.json` (`{ "host": "...", "model": "..." }`)
4. `OLLAMA_HOST`
5. default `http://127.0.0.1:11434`

## Requirements

- Register `ollama` as an OpenAI-compatible provider at `<host>/v1` whenever a
  host is configured or the model is `ollama/<tag>`.
- When `--model` is absent and a configured model exists, default to
  `ollama/<model>` (and ensure that tag exists in the provider catalog).
- HTTP only: never require or invoke the `ollama` binary.
- On connection failure print the resolved URL, the config file consulted, and
  the exact reason.
- Unit tests for resolver precedence; integration test against a local stub HTTP
  server serving `/api/tags`, `/v1/models`, and a chat completion with no
  `ollama` on `PATH`.

## Implementation

- New resolver module `crates/opencode-provider/src/ollama.rs`:
  `OllamaResolution`, `OllamaEnv`, `OllamaConfigFile`, `resolve`/`resolve_with`,
  `probe`, `connection_failure_message`, and `debug ollama`.
- Provider bootstrap applies the resolved base URL to every Ollama model and
  injects a configured non-default tag into the catalog.
- CLI resolves once before any server child is spawned, exports `OLLAMA_HOST`
  and the default `OPENCODE_TUI_MODEL`, and probes when explicitly configured.
- Server `effective_provider_setup` uses the resolved host/model and reports the
  actual source.
- Tests: resolver precedence unit tests plus `tests/ollama_remote.rs` against a
  stub HTTP server.

## Evidence

- `cargo test -p opencode-provider` passes (including `ollama_remote`).
- `cargo test -p opencode-server --lib` and `cargo test -p opencode-cli` pass.
- `debug ollama` reports the resolved URL and source; `--probe` prints the
  required failure diagnostic for an unreachable host.

## Related Items

- `Repo default model and provider ignored due to config precedence` (done)
- `unify-provider-setup-into-one-authoritative-user-path` (done)
- `define-local-model-first-provider-path` (done)
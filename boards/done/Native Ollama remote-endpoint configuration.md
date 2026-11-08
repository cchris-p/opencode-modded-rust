---
id: "FEAT-070"
title: "Native Ollama remote-endpoint configuration"
priority: "P1"
type: "feature"
area: "FEAT"
spec: "docs/provider-setup.md"
status: "done"
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

- Commit: `b1b173fc676fd0cf3af4e23243f6b7096142e4e1`
- Artifact: `cchris-p/opencode-modded-rust@native-artifacts`
  `opencode-modded-rust-linux-x86_64.tar.gz` (`.rev` = `b1b173f...`).
- `cargo test -p opencode-provider` passes (149 lib tests, including resolver
  precedence; `tests/ollama_remote.rs` 2/2, including stub `/api/tags`,
  `/v1/models`, and chat completion with no `ollama` binary).
- `cargo test -p opencode-server --lib` (75) and `cargo test -p opencode-cli`
  (21) pass.
- `debug ollama` reports the resolved URL and source; `--probe` prints the
  required failure diagnostic for an unreachable host.
- Known unrelated workspace failures at this commit: `opencode-core`
  architecture-layer tests (`opencode-oco` uncategorized, pre-existing) and
  two `opencode-tui` prompt env-lock tests that pass when run serially.

## Closeout

- Delivered directly on `development` as `b1b173f` (no PR branch was created;
  direct-commit closeout path). Branch cleanup is not applicable.
- Owner explicitly directed completion on 2026-11-08 after reviewing the test
  and artifact evidence; promoted from `qa` to `done`.
- Release promotion is handled separately in the `development` -> `main`
  deploy PR, which includes this feature along with the rest of the pending
  development range.

## Related Items

- `Repo default model and provider ignored due to config precedence` (done)
- `unify-provider-setup-into-one-authoritative-user-path` (done)
- `define-local-model-first-provider-path` (done)
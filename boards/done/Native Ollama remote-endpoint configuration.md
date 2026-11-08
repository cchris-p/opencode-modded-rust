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

Endpoint *host* resolution (the `model` key is a provider detail, not a
default-model source):

1. CLI `--model ollama/<tag>` (selects an Ollama model explicitly)
2. `OLLAMA_REMOTE_HOST`
3. `~/.config/opencode/ollama.json` (`host`)
4. `OLLAMA_HOST`
5. default `http://127.0.0.1:11434`

## Requirements

- Register `ollama` as an OpenAI-compatible provider at `<host>/v1` whenever a
  host is configured or the model is `ollama/<tag>`.
- A bare launch must keep the normal default model
  (`deepseek/deepseek-flash` unless a workspace config or `--model` says
  otherwise). The presence of `ollama.json`, including its `model` key, must
  not change it.
- Only an explicit Ollama selection (`--model ollama/<tag>` or a workspace
  config naming it) uses the configured Ollama host/model. The standards `ortq`
  wrapper passes `--model` explicitly.
- HTTP only: never require or invoke the `ollama` binary.
- On connection failure print the resolved URL, the config file consulted, and
  the exact reason.
- Unit tests for resolver precedence plus a regression test that a configured
  `{ "host": ..., "model": "qwen3:30b" }` does not change the bare-run default;
  integration test against a local stub HTTP server serving `/api/tags`,
  `/v1/models`, and a chat completion with no `ollama` on `PATH`.

## Implementation

- New resolver module `crates/opencode-provider/src/ollama.rs`:
  `OllamaResolution`, `OllamaEnv`, `OllamaConfigFile`, `resolve`/`resolve_with`,
  `probe`, `connection_failure_message`, and `debug ollama`.
- Provider bootstrap applies the resolved base URL to every Ollama model and
  injects a configured non-default tag into the catalog.
- CLI resolves once before any server child is spawned, exports only
  `OLLAMA_HOST`, and probes when explicitly configured. It never exports
  `OPENCODE_TUI_MODEL` from `ollama.json`.
- Server `effective_provider_setup` never derives a default model from the
  Ollama config file; the model comes from `OPENCODE_TUI_MODEL`, workspace
  config, or the product default.
- Tests: resolver precedence unit tests, a bare-run default regression test in
  `opencode-cli`, plus `tests/ollama_remote.rs` against a stub HTTP server.

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
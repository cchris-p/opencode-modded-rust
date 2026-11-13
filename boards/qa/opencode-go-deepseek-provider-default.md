---
id: "FEAT-078"
title: "Support OpenCode Go DeepSeek and make it the product default"
priority: "P1"
type: "feature"
area: "FEAT"
spec: "invariants/providers.md"
status: "qa"
created: "2026-11-13"
updated: "2026-11-13"
---

# Support OpenCode Go DeepSeek And Make It The Product Default

## Summary

OpenCode Go (`opencode-go`) was listed from the models.dev catalog but every chat
request failed with `400 MissingSessionID`: OpenCode Go requires a stable
`x-opencode-session` header, and the generic OpenAI-compatible transport never
sent one. A second latent bug made `opencode*` requests hit a random native
endpoint (`/google/v1beta`, `/anthropic/v1`) instead of the provider-level
OpenAI-compatible base URL.

Make OpenCode Go DeepSeek Flash work as a first-class provider, then make it the
product default model.

## Changes

- `opencode-provider::headers`: session-header helper for every `opencode*`
  provider id (`x-opencode-session` + product User-Agent).
- `opencode-provider::openai`: provider-option headers are transport-only
  (never serialized into the JSON body) and are applied on both the legacy
  non-streaming and streaming `/chat/completions` paths.
- `opencode-provider::bootstrap`: preserve the provider-level `api` as
  `baseURL` so `opencode`/`opencode-go` use the gateway's OpenAI-compatible
  endpoint instead of the first model's native endpoint.
- Session request builders attach the header: `SessionPrompt`, `/stream`,
  `LlmProcessor` (compaction), session/message title generation, subtasks, and
  `AgentExecutor` (which now also honors the resolved config model on a bare
  CLI run).
- `DEFAULT_MODEL` is `opencode-go/deepseek-v4.1-flash`; `invariants/providers.md`,
  `AGENTS.md`, `docs/provider-setup.md`, and dependent wiki notes are updated.

## Verification

- `run -m opencode-go/deepseek-v4.1-flash "Reply with exactly: OK"` -> `OK`
  (previously `400 MissingSessionID`).
- Bare `run` in a workspace without `opencode.json{,c}` -> answered with
  `opencode-go/deepseek-v4.1-flash` via the product default.
- `/prompt` on a fresh local server recorded the assistant message as
  `opencode-go/deepseek-v4.1-flash`.
- `cargo test` passes for `opencode-provider` (157), `opencode-session` (185),
  `opencode-config` (66), `opencode-agent` (22), `opencode-server` (77), and the
  CLI default-model test.
- `cargo clippy --all-targets` on touched crates: no errors (pre-existing
  warnings only).

## Merge / Release Status

- Merged into `development` on 2026-11-13 (owner-directed local merge; no PR
  created by the agent). Tip after the release bump: `df7af79`.
- CI on `development` is green (fmt, check, clippy, test, native).
- Workspace version bumped to `0.1.4`; the main checkout built and answered a
  live request on the Go default.
- Awaiting owner post-merge QA on `development` (owner will test later).
- Production release still requires the owner to open and merge the
  `development -> main` release PR (`v0.1.4`) after required checks and review.
  Fleet deploy follows the `INFRA-012` release automation.

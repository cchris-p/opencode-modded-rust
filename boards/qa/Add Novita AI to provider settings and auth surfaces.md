---
id: "FEAT-067"
title: "Add Novita AI to provider settings and auth surfaces"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "qa"
created: "2026-10-07"
---

# Add Novita AI To Provider Settings And Auth Surfaces

## Summary

The `novita-ai` provider is present in the `models.dev` catalog and works through
the generic OpenAI-compatible path (headless `run -m novita-ai/deepseek/deepseek-r1-0528`
succeeds), but it does not appear in `Settings > Provider` and is absent from the
CLI credential-provider list. Add it to every provider/auth surface so it is
selectable, reports auth state, and accepts an API key.

## Context

- `V1_PROVIDER_IDS` in `crates/opencode-tui/src/components/settings.rs` gated the
  Settings provider list and omitted `novita-ai`.
- `AUTH_ENV_PROVIDERS` in `crates/opencode-cli/src/main.rs` (used by
  `opencode auth list` / `auth login`) omitted `novita-ai`.
- `provider_auth_status_from_runtime` and `effective_provider_setup` in
  `crates/opencode-server/src/routes.rs` mapped env keys and built the Settings
  auth map for only a fixed subset.
- The Settings API-key entry/clear flow in `crates/opencode-tui/src/app/app.rs`
  was hardcoded to `"openai"`, so a key entered for any other provider would be
  saved under `openai`.
- Auth for `novita-ai` is keyed by `NOVITA_API_KEY`, which is already managed in
  the standards OpenBao `env/shared` document and rendered to
  `~/.config/opencode/.env` on every registered machine.

## Scope

- Add `novita-ai` / `NOVITA_API_KEY` to the CLI credential-provider list.
- Add the `novita-ai` env mapping and Settings auth-map entry in the server.
- Add `novita-ai` to the TUI Settings provider set.
- Make the Settings API-key connect/clear actions target the selected provider
  instead of `openai`.

## Done When

- `opencode auth list` shows `novita-ai` with its env status.
- `Settings > Provider` lists NovitaAI and reflects `NOVITA_API_KEY` auth state.
- Entering/clearing an API key in Settings applies to the selected provider.
- `cargo check` and `cargo test -p opencode-tui --lib` pass.

## Related Items

- `anthropic-provider-tool-transport-parity` (provider surface parity)

## Related Docs

- `invariants/providers.md`

## Implementation Notes - 2026-10-07

- `settings.rs`: added `novita-ai` to `V1_PROVIDER_IDS`.
- `main.rs`: added `("novita-ai", "NOVITA_API_KEY")` to `AUTH_ENV_PROVIDERS`.
- `routes.rs`: added the `novita-ai` env mapping in
  `provider_auth_status_from_runtime` and the `novita-ai` entry in the
  `effective_provider_setup` auth map.
- `app.rs`: Settings API-key connect/clear now uses the selected provider id
  instead of the hardcoded `openai`.
- Verification: `cargo check -p opencode-tui -p opencode-server -p opencode-cli`
  clean; `cargo test -p opencode-tui --lib` 176 passed; rebuilt binary reports
  `novita-ai  NOVITA_API_KEY  set` in `opencode auth list`.
- The key itself is standards-managed: `NOVITA_API_KEY` is in OpenBao
  `env/shared` and rendered to `~/.config/opencode/.env`, so no per-machine
  Settings entry is required for propagation.
- Follow-up: the initial change was not sufficient because the server's
  `is_v1_catalog_provider` gate (used by `list_providers` and
  `get_config_providers`) also excluded `novita-ai`, so the TUI never received
  it. Added `novita-ai` there too.
- Verified against a running server: `GET /provider` and
  `GET /config/providers` both return `novita-ai` (and the latter's `setup.auth`
  map includes it).

## Live QA - 2026-10-08

Agent-run live check of the provider on the `odn` model (`novita-ai/deepseek/deepseek-r1-0528`),
driven headlessly through the built server (`target/debug/opencode serve`, commit `f30920f`).

- Surfaces (this card's scope): PASS - `opencode auth list` shows `novita-ai  NOVITA_API_KEY  set`,
  the provider is selectable, and `GET /provider` / `GET /config/providers` include it.
- A tool-execution failure surfaced during this QA (the model leaks native DeepSeek tool tokens in
  content, so tools do not execute). That is a separate runtime defect tracked by `BUG-060`, not this
  card's scope.
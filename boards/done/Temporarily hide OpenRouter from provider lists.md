---
id: "BUG-032"
title: "Temporarily hide OpenRouter from provider lists"
priority: "P1"
type: "bug"
area: "BUG"
spec: ""
status: "done"
created: "2026-09-22"
---

# Temporarily hide OpenRouter from provider lists

## Summary

Temporarily remove OpenRouter from normal provider/model selection lists while keeping the provider implementation available for explicit configuration and later re-enable work.

## Why this exists

OpenRouter is currently implemented and documented as supported, but it should not appear in day-to-day provider lists while its current UX/support state is being reconsidered. This is a temporary visibility change, not a permanent provider removal.

## Scope

- Hide `openrouter` from user-facing provider/model lists in the TUI and CLI.
- Hide `openrouter` from the `Settings > Provider` curated provider list and provider auth/status summary.
- Keep explicit `openrouter/...` config handling intact unless implementation evidence shows that visible-list hiding requires a narrower runtime guard.
- Do not delete the OpenRouter provider module, catalog entries, auth key mapping, transform logic, or tests unless a follow-up explicitly changes the provider invariant.

## Done when

- Normal provider selection surfaces no longer list OpenRouter.
- Existing explicit OpenRouter configuration either still works or fails with a clear, intentional message documented in implementation notes.
- `invariants/providers.md` remains accurate for the temporary behavior.
- The implementation notes identify what needs to be reverted or changed to re-enable OpenRouter visibility.

## Likely touchpoints

- `crates/opencode-tui/src/components/settings.rs`
- `crates/opencode-server/src/routes.rs`
- `crates/opencode-cli/src/main.rs`
- `crates/opencode-tui/src/components/dialogs/model_select.rs`

## Recommended verification

- Run the TUI provider settings path and confirm OpenRouter is absent.
- Run CLI provider/model listing paths and confirm OpenRouter is absent.
- Confirm an explicit non-OpenRouter provider still works.
- Confirm explicit OpenRouter config behavior is documented.

## Implementation Notes

### Design

Visibility is controlled by one documented switch in `crates/opencode-provider/src/provider.rs`:

- `TEMPORARILY_HIDDEN_PROVIDER_IDS = &["openrouter"]`
- `is_provider_temporarily_hidden(provider_id)`

Provider implementation, catalog entries, auth key mapping (`OPENROUTER_API_KEY`), transforms, and tests are all left intact. Only list/selection surfaces consult the switch.

### Re-enable OpenRouter visibility

Remove `"openrouter"` from `TEMPORARILY_HIDDEN_PROVIDER_IDS`, then move/reopen this card as the tracking item. No other revert is required; every hidden surface below derives from that constant.

### Surfaces changed

- TUI `Settings > Provider`: `filtered_providers` (`crates/opencode-tui/src/components/settings.rs`) now skips temporarily hidden providers.
- TUI model-select fallback list (`crates/opencode-tui/src/components/dialogs/model_select.rs`) no longer ships hardcoded OpenRouter entries.
- Server `GET /provider` and `GET /config/providers`: `is_v1_catalog_provider` (`crates/opencode-server/src/routes.rs`) returns false for hidden providers.
- Server provider setup auth/status summary (`ProviderSetupInfo.auth`, `crates/opencode-server/src/routes.rs`) omits OpenRouter.
- CLI `opencode models` (`list_models`), interactive `/models` and `/providers`, and `opencode auth list`/login/logout prompt lists skip OpenRouter.

### Explicit OpenRouter behavior (documented)

- `openrouter/...` model requests still route normally; only the listing surfaces changed.
- `provider_env_var("openrouter")` still resolves `OPENROUTER_API_KEY`, so `opencode auth login openrouter --token ...` continues to work even though OpenRouter is absent from the displayed `auth list`.
- The effective-provider line in `Settings > Provider` still reports `openrouter` when it is explicitly configured.

### Board hygiene

The pulled card claimed `BUG-031`, which collided with `dialog-input-cursor-and-export-option-keys` (already `BUG-031`). This card was renumbered to `BUG-032`.

### Verification

- `cargo fmt --all -- --check` clean.
- `cargo check -p opencode-provider -p opencode-tui -p opencode-cli -p opencode-server` clean.
- `cargo test -p opencode-provider -p opencode-tui -p opencode-server -p opencode-cli` all passing (0 failures).

## PR

- https://github.com/cchris-p/opencode-modded-rust/pull/74 (`development` base)

## Completion

- Merged into `development` via merge commit `5bf0821` (2026-09-22).
- PR branch `feature/BUG-032-hide-openrouter-provider-lists` deleted remotely and locally.
- Completed on explicit user direction ("closeout") on 2026-09-22.

## Related items

- `START-012` Refresh provider and model catalog
- `START-027` Unify provider setup into one authoritative user path
- `FEAT-015` Preserve manually selected model and provider across sessions
- `invariants/providers.md`
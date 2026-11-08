---
id: "START-030"
title: "Match vanilla OpenAI model retrieval parity"
priority: "P1"
type: "feature"
area: "START"
spec: "invariants/providers.md"
status: "done"
created: "2026-09-22"
---

# Match vanilla OpenAI model retrieval parity

## Summary

Make the Rust product's OpenAI model list match vanilla OpenCode's current `models.dev`-backed model retrieval behavior for OpenAI, so `Settings > Provider`, provider APIs, and CLI model listing expose the same in-scope OpenAI/Codex-family models instead of a stale or manually divergent catalog.

## Why this exists

`START-029` identified that Codex 5.5 setup may be blocked by model catalog visibility rather than transport. Current Rust evidence shows two divergent OpenAI catalog sources:

- `crates/opencode-provider/src/bootstrap.rs` has a bundled V1 fallback containing newer OpenAI entries such as `gpt-5.3-codex`, `gpt-5-mini`, `gpt-5-nano`, and `o4-mini`.
- `crates/opencode-provider/src/openai.rs` still constructs an older hardcoded runtime model list with `gpt-4o`, `gpt-4o-mini`, `gpt-4-turbo`, `o1-preview`, and `o1-mini`.

Vanilla OpenCode does not rely on that kind of manually divergent OpenAI list for normal provider/model listing. It loads the `models.dev` catalog through `ModelsDev.Service`, transforms providers through `Provider.fromModelsDevProvider`, and exposes the resulting filtered provider list through provider APIs and model CLI paths.

This mismatch makes OpenAI/Codex setup confusing: a model may be usable by exact id but absent from the visible model list, or different surfaces may disagree about which OpenAI models exist.

## Required Product Invariant

`invariants/providers.md` now requires OpenAI model retrieval to match vanilla OpenCode's current `models.dev`-backed provider catalog for the in-scope OpenAI surface. Hidden divergence from vanilla model retrieval is not allowed.

## Current Evidence

- Reference repo was refreshed from `origin dev` on 2026-09-22 before writing this card.
- Vanilla `packages/opencode/src/provider/provider.ts` reads `modelsDevSvc.get()` and builds the provider catalog with `fromModelsDevProvider`.
- Vanilla `packages/opencode/src/server/routes/instance/httpapi/handlers/provider.ts` lists providers from `ModelsDev.Service`, filtered by enabled/disabled providers, and overlays connected providers.
- Vanilla `packages/opencode/src/cli/cmd/models.ts` lists models from `Provider.Service.list()` and supports refreshing the models cache.
- Vanilla `packages/opencode/src/plugin/openai/codex.ts` has Codex-specific allowed/disallowed model IDs including `gpt-5.5`, `gpt-5.3-codex-spark`, `gpt-5.4`, and `gpt-5.4-mini`, with `gpt-5.5-pro` disallowed for that plugin path.
- Rust `crates/opencode-provider/src/bootstrap.rs` contains the visible provider/model catalog fallback used by provider setup and listing.
- Rust `crates/opencode-provider/src/openai.rs` contains a separate stale hardcoded list returned by the `OpenAIProvider` runtime implementation.

## Scope

- Audit all Rust code paths that surface OpenAI models to users or APIs.
- Decide the single authoritative OpenAI catalog source for the Rust product, aligned with vanilla's `models.dev` retrieval semantics.
- Remove, bypass, or regenerate the stale hardcoded OpenAI model list in `openai.rs` so it cannot disagree with provider/model listing.
- Ensure `Settings > Provider`, `/config/providers` or equivalent provider API output, and CLI model listing all use the same OpenAI model set.
- Preserve existing runtime request behavior for exact model ids; this item is about retrieval/listing parity, not speculative transport changes.
- Document any intentional deviation from vanilla OpenAI model retrieval, including auth scope limits, disabled models, experimental filters, or Codex-plugin-only model constraints.

## Non-goals

- Implementing Codex 5.5 transport/auth itself; that remains `START-029` unless parity work proves the fix is only catalog retrieval.
- Adding every vanilla provider to the Rust V1 surface.
- Changing the product-owned default provider/model.
- Redesigning `Settings > Provider` UX beyond making its OpenAI list accurate.
- Speculative OpenAI Responses request-option changes.

## Implementation Direction

Prefer the smallest approach that makes all Rust OpenAI model listing surfaces agree with vanilla:

- First trace where `OpenAIProvider::models()` is used versus where `bootstrap.rs`/models data is used.
- If `OpenAIProvider::models()` is only a legacy/runtime fallback, make it consume catalog-derived models or stop using it for user-facing listing.
- If `bootstrap.rs` is the source of user-facing provider data, refresh its OpenAI source from the same generated `models.dev` snapshot conventions already represented in Rust, and add tests that catch drift against expected OpenAI/Codex ids.
- Keep enabled/disabled provider filtering intact.
- Keep exact model-id execution working even when a model is absent from fallback data, but do not use that as a substitute for correct listing.

## Likely Touchpoints

- `invariants/providers.md`
- `boards/refinement/setup-opencode-codex-55-compatibility.md`
- `crates/opencode-provider/src/bootstrap.rs`
- `crates/opencode-provider/src/openai.rs`
- `crates/opencode-provider/src/models.rs`
- `crates/opencode-server/src/routes.rs`
- `crates/opencode-cli/src/main.rs` or model-listing command code
- `crates/opencode-tui/src/components/dialogs/provider.rs`
- `docs/provider-setup.md`

## Acceptance Criteria

- There is one clear authoritative Rust path for OpenAI model retrieval/listing.
- The user-facing OpenAI model list matches vanilla OpenCode's current `models.dev`-backed OpenAI catalog for the in-scope surface, or every intentional exclusion is explicitly documented.
- Stale `openai.rs` hardcoded model data cannot override or contradict the provider/model catalog shown to users.
- `Settings > Provider` shows the parity OpenAI model set.
- The server/provider API model listing shows the same OpenAI model set as the TUI.
- The CLI model listing shows the same OpenAI model set as the TUI/server.
- Codex-family models relevant to `START-029`, including Codex 5.5 if present in vanilla's current catalog or plugin-allowed path, are either visible or explicitly documented as intentionally excluded with the reason.
- Regression coverage fails if the Rust OpenAI list falls back to the old `gpt-4o`/`o1`-only hardcoded set.

## Recommended Verification

- Compare vanilla OpenAI model IDs from `$HOME/repos/opencode-modded` `dev` after `git -C "$HOME/repos/opencode-modded" fetch origin dev`.
- Run the Rust provider/model listing path and confirm OpenAI model IDs match the selected vanilla source.
- Run the TUI and confirm `Settings > Provider` shows the same OpenAI list.
- Run the CLI model listing for OpenAI and confirm it matches the TUI/server list.
- Run targeted tests for provider bootstrap/model listing, likely `cargo test -p opencode-provider openai` plus any server/TUI tests that cover provider listing.
- If code changes affect public docs, update `docs/provider-setup.md` and re-run relevant doc-linked smoke checks.

## Related Items

- `START-029` Set up OpenAI Codex 5.5 compatibility
- `START-012` Refresh provider and model catalog
- `START-015` Mirror OpenAI auth configuration in settings
- `START-027` Unify provider setup into one authoritative user path
- `FEAT-013` Model capability gating and deprecated model surfacing

## Ready To Implement Checklist

- [x] Bug/feature surface is reproduced from code evidence: Rust OpenAI lists diverge across `bootstrap.rs` and `openai.rs`.
- [x] Reference behavior source is identified: vanilla `ModelsDev.Service` and provider listing on `dev`.
- [x] Product invariant has been updated in `invariants/providers.md`.
- [x] Related Codex setup work is linked through `START-029`.
- [x] Acceptance criteria specify exact user-facing surfaces to verify.

## Dev Notes - 2026-09-22

- Refreshed the bundled OpenAI fallback catalog in `crates/opencode-provider/src/bootstrap.rs` with current reference OpenAI/Codex-family entries including `gpt-5.3-codex-spark`, `gpt-5.4`, `gpt-5.4-mini`, `gpt-5.5`, and `gpt-5.5-pro`.
- Kept the bootstrap/models.dev-derived provider state as the authoritative listing source for wrapped runtime providers; `OpenAIProvider` still handles transport for exact model ids.
- Updated fallback env registration to wrap concrete providers with catalog-derived provider state, preventing fallback registration from exposing the stale hardcoded `openai.rs` model list.
- Added regression coverage that fails if wrapped OpenAI listings omit the current catalog models or fall back to the old `o1-preview` hardcoded set.
- PR: https://github.com/cchris-p/opencode-modded-rust/pull/73
- Verification: `git -C "$HOME/repos/opencode-modded" fetch origin dev`; reference paths still use `ModelsDev.Service`/`fromModelsDevProvider`; `cargo test -p opencode-provider openai`; `cargo test -p opencode-provider bootstrap`.

## Closeout - 2026-09-22

- PR #73 merged into `development` at `74a1135fa7c38deaa56854b5716db4fe7a7c7404`.
- Branch cleanup completed for `feature/START-030-openai-model-parity`.
- Item remains in `qa` pending post-merge QA report or explicit completion direction.

## QA Report - 2026-09-22

### Reported symptom

- User could not see "Codex 5.5" (the OpenAI `gpt-5.5` entry) in the product's model surfaces after the START-030 merge.

### Root cause (the parity claim was not reproducible)

- `crates/opencode-provider/src/models.rs` typed `ModelInfo.experimental` as `Option<bool>`. The canonical catalog (`https://models.opencode.ai/api.json`) emits an object for this field, e.g. `gpt-5.5` has `experimental.modes.fast`.
- Strict `serde` deserialization therefore failed for the whole `openai`, `anthropic`, and `github-copilot` providers. `load_models_dev_cache`'s per-provider fallback silently drops providers that fail to parse, so OpenAI disappeared from every listing surface entirely.
- The pre-existing tests (`wrapped_openai_provider_lists_catalog_models`, `bundled_v1_catalog_includes_ollama_local_path`) only validated the hand-written bundled fallback in `bootstrap.rs`, never the models.dev parse path, so they passed while the real runtime list was empty.
- Consequently the original "parity" assertion had no path to detect divergence and did not hold.

### Fix

- Typed the experimental schema (`ModelExperimental` / `ModelExperimentalMode` / `ModelExperimentalModeProvider`) instead of `Option<bool>`.
- `from_models_dev_provider` now expands each `experimental.modes` entry into an addressable `<model-id>-<mode>` model, matching vanilla `fromModelsDevProvider`: derived name, merged mode cost, mode provider body options (`serviceTier`, `reasoningMode`), mode headers, and base `api.id`.
- Removed the undocumented `OpenAI` custom-loader blacklist (`whisper`/`tts`/`dall-e`/`embedding`/`moderation`); vanilla does not blacklist these, and only the embedding entries exist in the current catalog.
- Added `ensure_models_dev_cache()` and call it from server startup/refresh and CLI provider setup, so parity no longer depends on a pre-existing cache file; on miss the canonical catalog is fetched before provider bootstrapping.
- Aligned the fetch source to vanilla's endpoint (`https://models.opencode.ai`).
- `AliasedProvider` now routes derived mode model ids to their base API model id at request time, so listed mode variants do not emit invalid model ids.

### How parity is determined

- `scripts/compare-openai-model-parity.sh` is the parity check. It fetches the canonical catalog, derives the expected OpenAI list with vanilla's rules (drop `deprecated`; drop `alpha` unless experimental models are enabled; expand `experimental.modes` into `<id>-<mode>`), runs the Rust CLI against a temporary cache seeded with that exact catalog, and diffs both sorted sets. Non-zero exit on divergence.

### Verification evidence

- `scripts/compare-openai-model-parity.sh` -> `OpenAI model list matches vanilla models.dev catalog (49 models).`
- `opencode models openai` -> 49 ids including `gpt-5.5`, `gpt-5.5-fast`, `gpt-5.5-pro`, `gpt-5.4`, `gpt-5.3-codex`, `gpt-5.3-codex-spark`, and the three `text-embedding-*` entries.
- Server `GET /provider` -> same 49 OpenAI ids (TUI `Settings > Provider` reads this endpoint).
- Fresh-cache check: with `~/.cache/opencode/models.json` removed, the CLI fetched the canonical catalog and still listed 49 OpenAI ids.
- `cargo test -p opencode-provider` -> 96 + 7 passed; new test `models_dev_openai_experimental_modes_and_embeddings_are_listed`.
- `cargo check --workspace` clean.

### Documented follow-up

- Catalog freshness/auto-refresh is tracked by `START-031` "Auto-refresh the models.dev catalog for standing OpenAI parity". Without it this card's match is point-in-time only.
- Mode-variant execution options (`serviceTier` / `reasoningMode`) are listed with correct metadata, and derived ids route to the base model, but the Rust request path does not yet translate the mode provider body into request options. That is transport work outside this item's retrieval/listing scope and is not yet carded.

## Completion - 2026-09-22

- QA report recorded above; verification evidence reproduced and passing.
- Marked `done` by explicit user direction.
- Follow-up filed: `START-031` Auto-refresh the models.dev catalog for standing OpenAI parity.
- Code fix delivered in PR #76 (`feature/START-030-openai-model-parity-fix`), merged into `development` at `835bdb89c925989f15e046e6add5ec9d8ed8256a`.
- Branch cleanup completed for `feature/START-030-openai-model-parity-fix` (remote and local deleted).




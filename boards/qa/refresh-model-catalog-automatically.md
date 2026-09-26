---
id: "START-031"
title: "Auto-refresh the models.dev catalog for standing OpenAI parity"
priority: "P1"
type: "feature"
area: "START"
spec: "invariants/providers.md"
status: "qa"
created: "2026-09-22"
---

# Auto-refresh the models.dev catalog for standing OpenAI parity

## Summary

Add a real catalog refresh policy to the Rust product so provider/model listings keep matching vanilla OpenCode's current `models.dev`-backed catalog over time, instead of freezing whatever catalog was present when the cache file was first written.

## Why this exists

`START-030` fixed OpenAI model retrieval so the list matches vanilla's catalog at the moment of verification (49 OpenAI models, `scripts/compare-openai-model-parity.sh` passing). That match is currently point-in-time only:

- `ModelsRegistry::load()` reads the on-disk cache with no TTL and returns it forever once it parses.
- `ensure_models_dev_cache()` only fetches when the cache file is missing or unparseable; it never refetches a stale cache.
- `opencode models --refresh` prints a note and does not refresh.
- Server `refresh_providers()` only re-reads the same cache; it is triggered by config/auth changes, not by catalog staleness or time.

Vanilla refreshes: it applies a 5-minute TTL to its cached catalog, schedules a refresh roughly every 60 minutes, and `opencode models --refresh` forces a refetch. Without an equivalent, new upstream OpenAI/Codex-family models will not appear, and the START-030 invariant ("match vanilla's current catalog") will silently decay. The catalog content changed during the START-030 session, so this is observed, not hypothetical.

## Current Evidence

- `crates/opencode-provider/src/models.rs` `ModelsRegistry::load` reads `dirs::cache_dir()/opencode/models.json` with no mtime or TTL check.
- `crates/opencode-provider/src/models.rs` `ensure_models_dev_cache` calls `ModelsRegistry::get`, which only fetches on cache read/parse failure.
- `crates/opencode-cli/src/main.rs` `list_models` handles `--refresh` by printing a parity note only.
- Vanilla `packages/core/src/models-dev.ts` uses `Duration.minutes(5)` freshness, a `Schedule.spaced("60 minutes")` refresh, and a forced refresh path.
- `scripts/compare-openai-model-parity.sh` verifies the list at a point in time; it does not prove the product refreshes.

## Scope

- Define a freshness policy for the models.dev catalog cache (mirror vanilla's TTL unless a documented reason says otherwise).
- Make `ensure_models_dev_cache` refetch when the cache is stale rather than only when missing.
- Make `opencode models --refresh` force a catalog refetch and refresh the provider registry.
- Refresh the provider registry after a successful catalog refresh so long-running sessions pick up new models.
- Optional: schedule a periodic refresh in the server (vanilla uses ~60 minutes).
- Keep failures non-fatal: on fetch failure, keep using the existing cache and log/ignore.

## Non-goals

- Changing the catalog source or the OpenAI parity rules established by `START-030`.
- Wiring mode-variant execution options (`serviceTier` / `reasoningMode`); that remains the documented follow-up from `START-030`.
- Adding new providers to the V1 surface.
- Provider auth or transport changes.

## Acceptance Criteria

- After the initial fetch, a stale cache (older than the designated TTL) is refetched on normal CLI/server startup without requiring manual file deletion.
- `opencode models --refresh` actually fetches the latest catalog and the subsequent listing reflects any upstream changes.
- A running server picks up catalog updates after the refresh policy runs, without a restart.
- Fetch failures degrade gracefully to the existing cache.
- The delegated refresh reuses the same parse/transform path so `scripts/compare-openai-model-parity.sh` still passes after a refresh.
- The freshness policy and any intentional deviation from vanilla's TTL/schedule are documented in code and repo docs.

## Recommended Verification

- Build with `ort-build`.
- Seed an old cache (or set a test TTL), run `opencode models openai`, and confirm a refetch occurs and the catalog updates.
- Run `opencode models --refresh` and confirm a real network fetch plus updated listing.
- Run `scripts/compare-openai-model-parity.sh` after a refresh and confirm it still passes.
- Add focused tests for staleness detection and refresh behavior using an injectable clock or temp cache directory.
- `cargo test -p opencode-provider` and `cargo check --workspace`.

## Likely Touchpoints

- `crates/opencode-provider/src/models.rs`
- `crates/opencode-cli/src/main.rs`
- `crates/opencode-server/src/server.rs`
- `crates/opencode-provider/src/bootstrap.rs`
- `invariants/providers.md`
- `docs/provider-setup.md`

## Dev Notes

Implemented the freshness policy in `crates/opencode-provider/src/models.rs`:

- Added `MODELS_DEV_TTL` (5 minutes, mirroring vanilla's `Duration.minutes(5)`) and
  `cache_mtime_is_fresh`, a pure mtime-age predicate (future mtimes count as fresh so clock
  skew never forces a refetch loop).
- `ModelsRegistry` now carries the TTL and source URL. `load()` serves a fresh parsed cache,
  but refetches when the cache is stale; on fetch failure it falls back to the existing parsed
  cache instead of emptying the catalog.
- `fetch()` now returns `Option<ModelsData>` so failure is distinguishable from an empty
  catalog, and `refresh(force: bool) -> bool` re-fetches only when stale or when forced.
- Added `refresh_models_dev_cache()` for the forced CLI path; `ensure_models_dev_cache()` is
  now TTL-aware and used by bootstrap.

Wiring:

- `crates/opencode-cli/src/main.rs` `list_models` calls `refresh_models_dev_cache()` on
  `--refresh` and reports success/failure instead of the old parity-note stub, then rebuilds
  the registry from the refreshed cache via `setup_providers`.
- `crates/opencode-server/src/server.rs` adds `MODELS_DEV_REFRESH_INTERVAL` (60 minutes,
  mirroring vanilla's `Schedule.spaced("60 minutes")`) and `spawn_models_dev_refresh`, spawned
  by `run_server`/`run_server_with_state`; it calls `refresh_providers()` (TTL-aware refetch +
  registry rebuild) so long-running servers pick up new models without a restart.

Docs/invariants: added a freshness invariant to `invariants/providers.md` and a "Model catalog
freshness" section to `docs/provider-setup.md`.

Verification performed:

- `cargo test -p opencode-provider` (all tests pass, including new staleness/refresh-failure
  tests in `models::tests`).
- `cargo check --workspace` (with `SCOPEMUX_SKIP_NATIVE_BUILD=1`) passes.
- `scripts/compare-openai-model-parity.sh` passes: "OpenAI model list matches vanilla models.dev
  catalog (55 models)."
- Manual stale-cache check: seeded a 2-hour-old cache; `opencode models openai` refetched it
  (mtime advanced, stale marker gone).
- Manual `--refresh` check: seeded a fresh bogus cache; `opencode models --refresh openai`
  printed "Model catalog refreshed from models.dev." and replaced the cache.

Known limits: atomic temp-write/rename and cross-process file locking from vanilla were not
added; the existing direct write remains. The scheduled server refresh runs only in
`run_server`/`run_server_with_state`, which is the single product server path.

## Related Items

- `START-030` Match vanilla OpenAI model retrieval parity
- `START-012` Refresh provider and model catalog
- `START-029` Set up OpenAI Codex 5.5 compatibility

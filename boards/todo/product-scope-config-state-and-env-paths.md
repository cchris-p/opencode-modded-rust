---
id: "START-035"
title: "Product-scope config, state, and env paths (Layer 2)"
priority: "P3"
type: "refactor"
area: "START"
spec: "invariants/reference-boundary.md"
status: "todo"
predecessors: "START-033"
created: "2026-10-14"
updated: "2026-10-14"
---

# Product-scope config, state, and env paths (Layer 2)

## Summary

`START-033` decoupled the product's user-facing command identity (Layer 1) but explicitly deferred
**Layer 2**: the config/state/data directories and `OPENCODE_*` environment variable names are still
shared with vanilla OpenCode. This card records and scopes that deferred work so the product can own
its runtime state without colliding with vanilla.

This is a follow-up to `START-034`/`CLI-013` (handoff `H-015`); Layer 1 is done. Layer 3 (crate/
package/repo name renames) remains a non-goal here.

## Why this exists

- The product currently reads/writes state under bare `opencode` paths shared with vanilla:
  - Config dir: `~/.config/opencode/*` (standards-managed config, `AGENTS.md`, `tui.json`, `opencode.json` live here).
  - Data dir / DB: `dirs::data_local_dir()/opencode/opencode.db` (`crates/opencode-storage/src/database.rs`);
    traces: `dirs::data_local_dir()/opencode/traces/server.log`.
  - State dir: `dirs::state_dir()/opencode/kv.json` (`crates/opencode-tui/src/context/app_context.rs`).
- Env vars: `OPENCODE_SERVER_LOG`, `OPENCODE_RUN_TIMEOUT_MS`, `OPENCODE_STREAM_BUDGET_MS`, plus launcher
  pass-through vars `OPENCODE_CONFIG_CONTENT`, `OPENCODE_CONFIG_DIR`, `OPENCODE_TUI_CONFIG`, and
  `OPENCODE_MODEL_*` presets.
- Because `opencode` on `PATH` is the pinned vanilla build, shared paths/env can cause cross-product
  interference (config bleed, DB lock contention, surprising defaults).

## Scope

- Decide and implement product-scoped config, data, and state paths (e.g. an `opencode-modded-rust`
  subdir) or an explicit documented decision to keep sharing vanilla's paths for continuity.
- Decide and implement `OPENCODE_*` env var scoping (product-scoped names) or keep them with a
  documented compatibility rationale.
- Define migration/compat policy: read-old/write-new, one-time migration, or accept fresh state.

## Non-goals

- Layer 1 command/binary/clap identity (`START-033`, done).
- Layer 3 crate/package/repo name renames.
- The ScopeMux integration, provider ids, mdns domain, event ids, and the model system-prompt
  identity (kept as compatibility/implementation surfaces).

## Risks to weigh

- State fragmentation and lost sessions if migration is done wrong.
- Breaking the standards-managed config under `~/.config/opencode/`.
- macOS vs vanilla DB path mismatch and hidden coupling in launchers, plugins, and tests.

## Done when

- Paths and env vars are either product-scoped or explicitly documented as intentionally shared, with
  a migration/compat decision recorded.
- Standards launcher and managed config continue to work; `opencode` (vanilla) is untouched.
- Behavior is verified on the TUI + headless CLI paths.

## Related Items

- `START-033` Decouple the product command identity from opencode (Layer 1) — predecessor; deferred this.
- `START-034` Scrub scopemux-code product-name references.
- `CLI-013` Reach the Rust CLI subcommands through the launcher.
- `invariants/reference-boundary.md` — canonical product name / compatibility boundary.

## Notes

- Created 2026-10-14 as the deferred Layer 2 follow-up recorded in `START-033` during `H-015`.

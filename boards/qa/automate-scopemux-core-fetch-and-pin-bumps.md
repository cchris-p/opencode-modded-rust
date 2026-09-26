---
id: "INFRA-002"
title: "Automate scopemux-core fetch, drift detection, and pin bumps for native builds"
priority: "P2"
type: "chore"
area: "INFRA"
spec: ""
status: "qa"
created: "2026-09-26"
---

# Automate scopemux-core fetch, drift detection, and pin bumps for native builds

## Summary

`ort-build` failed on a machine whose `third_party/scopemux-core` was absent: the
`scopemux-native` feature is on by default, so
`crates/opencode-scopemux/build.rs` panics when the core source is missing. The
pin could also drift silently: the fetch script exited early whenever
`third_party/.git` existed, so bumping `PINNED_REV` never refreshed an existing
checkout.

## Why this exists

The core source is a manual build prerequisite that depends on machine state
(fresh clone, worktree, deleted checkout), and the pinned revision is advanced by
hand. Both fail in ways that only surface at build time. This card makes the
prerequisite self-healing and the pin observable.

## Scope

- `ort-build` ensures the core before invoking cargo.
- Fetch script is pin-aware (`--check`, `--force`, auto-refresh on drift).
- A bump helper that advances the pin and verifies the native provider.
- `build.rs` copies queries into `OUT_DIR`, so the binary no longer depends on
  the source checkout at runtime.
- Docs updated.

## Non-goals

- Auto-advancing the pin on a schedule (upstream sync stays selective).
- Changing how the product resolves the core beyond the existing env/fallback.
- CI (tracked separately as `INFRA-003`).

## Implementation (2026-09-26)

- `~/standards/opencode-config` (`main`): new `opencode-rust-ensure-scopemux-core`
  preflight, called from `opencode-rust-build` after the existing toolchain/disk
  preflights. It honors `SCOPEMUX_CORE_DIR` and `SCOPEMUX_SKIP_NATIVE_BUILD`,
  auto-runs `scripts/fetch-scopemux-core.sh` when `third_party/scopemux-core` is
  missing, and warns when the checkout `HEAD` differs from `PINNED_REV`
  (`OPENCODE_RUST_REFRESH_CORE=1` refreshes). Committed on `main` as `783e871`.
- `scripts/fetch-scopemux-core.sh`: pin-aware. `--check` verifies the checkout
  matches the pin (non-zero when missing/stale) without mutating; `--force`
  refreshes; the default path now refreshes on pin drift instead of exiting
  early.
- `scripts/bump-scopemux-pin.sh`: updates `PINNED_REV`, refreshes the checkout,
  confirms with `--check`, and runs `cargo test -p opencode-scopemux --features native`.
- `crates/opencode-scopemux/build.rs`: copies `<core>/queries` into
  `OUT_DIR/queries` and bakes that path as `SCOPEMUX_QUERIES_DIR`, so the compiled
  binary does not need the source checkout to remain in place.
- `AGENTS.md`: "Local Launchers" bullet rewritten to describe the automatic
  preflight, drift warning, `--check`, and bump helper.

## Verification

- `bash -n` on both scripts.
- `scripts/fetch-scopemux-core.sh --check` returns 1 (missing) and 1 (stale) and
  exits 2 on an unknown argument; `--check` never mutates.
- Launcher preflight tested in isolation: skip, explicit-valid, explicit-invalid,
  drift-warn, and missing-dest cases.
- `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo check -p opencode-scopemux --features native`
  compiles `build.rs` and `lib.rs`.
- Full native build against `$HOME/apps/scopemux-core` (core `656f22f`) succeeded;
  the new `OUT_DIR/.../queries` tree contains all six language query sets and
  `libparser_core.a` plus the tree-sitter libs were built.

## Follow-up fix (2026-09-26)

Verifying on `development` surfaced a second, pre-existing bug: the CMake build
tree was fixed at `OUT_DIR/scopemux-core-build`, so switching the core source path
(a local `SCOPEMUX_CORE_DIR` versus `third_party/scopemux-core`) reused a CMake
cache generated for a different source and failed with "does not match the source
... used to generate cache". The build directory is now scoped by a hash of the
resolved source path (`scopemux-core-build-<hash>`), so each source keeps its own
cache. Verified by building with both sources in sequence with no `cargo clean`.

## Open items / QA focus

- End-to-end `ort-build` verified on `development`: the launcher auto-fetched
  `third_party/scopemux-core`, the CLI built, and `opencode version` printed
  `OpenCode 0.1.0`. Disk was ~14 GB free afterward, so future builds may trip the
  `INFRA-001` disk warning.
- Duplicate-symbol warning observed on macOS: linking the native provider warns
  about duplicate `_ts_*` symbols between the crate's vendored tree-sitter and the
  `tree-sitter` Rust crate. The link succeeds on macOS (ld64 warning); confirm
  whether Linux `ld` treats it as an error before relying on native builds there.
- Candidate follow-up: advance `PINNED_REV` from `e93df08` to core `main`
  (`656f22f`) (22 commits ahead; includes the `WI-028` cleanup double-free fix).
  Pin-bump only after native tests pass at that revision.

## PR

- [#122](https://github.com/cchris-p/opencode-modded-rust/pull/122) into `development` (launcher preflight, pin-aware fetch, bump helper, query copy).
- Source-scoped CMake build dir: [#123](https://github.com/cchris-p/opencode-modded-rust/pull/123).
- Launcher change committed separately in `~/standards` `main` as `783e871` (local, not pushed).

## Closeout

- `#122` merged into `development` as `c8a4246`; `#123` merged as `f2e50b6`. Feature branches deleted remotely and locally.
- Verified on `development`: `ort-build` auto-fetched the core and produced a working binary (`opencode version` → `OpenCode 0.1.0`).
- Remains in `qa` pending a recorded QA report or explicit completion.

## Related Items

- `INFRA-001` build and disk hygiene (shared launcher preflight pattern).
- `SCOPE-004` consumes `WI-034`/`WI-035`/`WI-036`, which require a pin newer than
  the current `e93df08`.

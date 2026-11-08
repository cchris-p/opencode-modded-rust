---
id: "INFRA-001"
title: "Contain build and data cache disk growth across worktrees"
priority: "P2"
type: "chore"
area: "INFRA"
spec: ""
status: "qa"
created: "2026-09-23"
---

# Contain build and data cache disk growth across worktrees

## Summary

Disk reached 100% (127 Mi free) during normal work in
`$HOME/worktrees/opencode-modded-rust/<name>`, which blocked a worktree build
outright (`No space left on device` while linking). The Rust product is a large
workspace and every worktree compiles its own `target/` because no shared
build cache is configured. This card tracks reclaiming space and preventing a
repeat.

## Why this exists

Worktrees are the repo's normal branch workflow (`ort-build`/`ort` under
`$HOME/worktrees/opencode-modded-rust/`). Each worktree resolves
`CARGO_TARGET_DIR` to its own `target/`, so a full workspace build is duplicated
per worktree. There is no `~/.cargo/config.toml`, no repo `.cargo/config.toml`,
and no `RUSTC_WRAPPER`/sccache, so nothing shares or dedupes artifacts. A full
build in one worktree plus the main checkout's `target/` can exhaust the disk and
stall unrelated work.

## Evidence

Measured `2026-09-23` after a partial worktree build:

- `/Users/cchrisleepyles/repos/opencode-modded-rust/target` — 37 GB.
- `/Users/cchrisleepyles/worktrees/opencode-modded-rust/grep-freeze-and-session-inspect/target`
  — 3.1 GB after only `cargo build -p opencode-cli` (a full workspace build/test
  is much larger).
- `~/.local/share/opencode/opencode.db` (vanilla OpenCode, not the Rust product)
  — 2.0 GB.
- Root filesystem hit `100%` capacity (127 Mi free); after the user cleared ~20 GB
  it recovered to ~27 GB free (94% used).

## Plan (approved 2026-09-23)

Chosen strategy: **one shared `CARGO_TARGET_DIR` for all git worktrees**, leaving
the main checkout's `target/` untouched.

- Cargo discovers `.cargo/config.toml` in parent directories of the build
  directory. Write a managed config at
  `$OPENCODE_WORKTREE_ROOT/opencode-modded-rust/.cargo/config.toml` (outside any
  git repo) that sets `[build] target-dir` to a single shared directory
  `$OPENCODE_WORKTREE_ROOT/opencode-modded-rust/.shared-target`. Every worktree
  beneath that root inherits it; the main checkout at
  `$HOME/repos/opencode-modded-rust` does not.
- `scripts/setup-worktree-build-cache.sh` creates the managed config and shared
  target dir idempotently. `scripts/clean-build-caches.sh` reports and reclaims
  space from duplicated per-worktree `target/` dirs (and optionally the shared
  dir and the main checkout).
- `ort-build` gains a disk-space preflight that warns below a threshold and
  points at the cleanup command.
- Document the cache and cleanup command in `AGENTS.md` under "Local Launchers".

## Scope / non-goals

- In scope: shared worktree target dir, cleanup command, `ort-build` preflight,
  docs.
- Non-goals: sccache/`RUSTC_WRAPPER`, changing `[profile.dev]` debug info (may
  be revisited separately), and deleting the vanilla
  `~/.local/share/opencode/opencode.db` (a human decision; see Deferred).

## Done when

- A worktree build no longer duplicates tens of GB, or a documented shared-cache
  setup makes the duplication cheap.
- A documented cleanup command restores comfortable disk headroom.
- `ort-build` warns before a build can fill the disk.

## Dev Notes

- Added `scripts/setup-worktree-build-cache.sh`: idempotently writes a managed
  `.cargo/config.toml` at `$OPENCODE_WORKTREE_ROOT/opencode-modded-rust/` with
  `[build] target-dir` set to the shared `.shared-target` dir.
- Added `scripts/clean-build-caches.sh`: reports duplicated per-worktree
  `target/` dirs (dry run by default), with `--yes` to delete, `--shared` for the
  shared cache, and `--include-main` for the main checkout.
- Documented both under `AGENTS.md` → "Local Launchers".
- Verified with `cargo clean --dry-run -v` that a worktree resolves its target to
  `.shared-target` while the main checkout still resolves to its own `target/`.
- Verified cleanup behavior against throwaway worktrees (duplicate detection and
  `--yes` deletion).
- Added `opencode-rust-disk-preflight` to `~/standards/opencode-config` so
  `ort-build` warns below `OPENCODE_RUST_MIN_FREE_GB` (default 15) and points at
  the cleanup script. Committed separately in `~/standards` on `development`
  as `2a533c3` (local only, not pushed).

### Deferred

- `[profile.dev]` debug-info shrink: not changed; revisit separately.
- Removing the 2.0 GB vanilla `~/.local/share/opencode/opencode.db` is a human
  decision and was not performed.

## Closeout

- Merged into `development` as PR #102 (merge commit `bc5fcc1`) on 2026-09-23.
- Feature branch `feature/INFRA-001-worktree-build-cache` deleted remotely and
  locally; temporary worktrees removed.
- Remains in `qa` pending a recorded QA report or explicit completion.

## Related Items

- `GATE-004` reference pin (worktree conventions).

---
id: "INFRA-001"
title: "Contain build and data cache disk growth across worktrees"
priority: "P2"
type: "chore"
area: "INFRA"
spec: ""
status: "todo"
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

## Next steps

- Decide a cache strategy and record it: a shared `CARGO_TARGET_DIR` for all
  worktrees (repo or `~/.cargo/config.toml`, with a launcher env override), or
  sccache via `RUSTC_WRAPPER`.
- If shared `target-dir` is chosen, document it in `AGENTS.md` under "Local
  Launchers" so `ort-build` and manual `cargo` runs use it.
- Shrink debug artifacts: evaluate `[profile.dev] debug = 1` /
  `split-debuginfo = "unpacked"`, or prune with `cargo clean` /
  `cargo sweep --time` after merges.
- Add a disk-space preflight to `ort-build` that warns below a threshold and
  points at the cleanup command.
- Review the 2.0 GB vanilla `~/.local/share/opencode/opencode.db` (separate from
  the Rust product DB at `~/Library/Application Support/opencode/opencode.db`)
  and archive/remove it if the vanilla line is no longer used locally.

## Done when

- A worktree build no longer duplicates tens of GB, or a documented shared-cache
  setup makes the duplication cheap.
- A documented cleanup command restores comfortable disk headroom.
- `ort-build` warns before a build can fill the disk.

## Related Items

- `GATE-004` reference pin (worktree conventions).

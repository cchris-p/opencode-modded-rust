---
id: "INFRA-004"
title: "Align checkout/setup paths with the standards-managed submodule and trim disk footprint"
priority: "P2"
type: "chore"
area: "INFRA"
spec: ""
status: "qa"
created: "2026-10-08"
---

# Align checkout/setup paths with the standards-managed submodule and trim disk footprint

## Summary

The product's canonical checkout is now the standards-managed submodule
`~/standards/vendor/opencode-modded-rust` (branch `development`), driven by
`OPENCODE_RUST_REPO` in `~/standards/opencode-config`. The legacy standalone
clone `~/repos/opencode-modded-rust` was removed on 2026-10-08. This card tracks
finding and landing the remaining space/setup improvements that follow from that
move, then cleaning up the leftovers.

## Why this exists

`STD-021` in `~/standards` retired standalone `~/repos/<managed>` clones across
registered machines, but this operator machine kept its own
`~/repos/opencode-modded-rust` until 2026-10-08. That clone alone held ~22 GB
(21 GB was `target/`). With it gone, disk recovered from ~14 GiB to ~36 GiB free,
but several setup paths and space sinks still assume the old location.

## Evidence (measured 2026-10-08, operator machine)

- Removed `~/repos/opencode-modded-rust` (clean: `development`/`main` both at
  `0d2f2db`, matching `origin`; no unpushed commits). Free disk went
  ~14 GiB → ~36 GiB.
- Remaining repo space sinks:
  - `~/standards/vendor/opencode-modded-rust/target` — ~4.8 GB.
  - `~/worktrees/opencode-modded-rust/.shared-target` — ~6.3 GB.
  - `~/standards/vendor/opencode-modded-rust/third_party` — ~505 MB.
  - `~/.local/share/opencode/opencode.db` (vanilla OpenCode, not the Rust
    product) — ~3.1 GB; `~/Library/Application Support/opencode/opencode.db`
    (Rust product) — ~32 MB.
- Setup paths still hardcode the removed clone:
  - `AGENTS.md:69`, `AGENTS.md:74`, `AGENTS.md:77`.
  - `scripts/scopemux-qa-server.sh:17,28`.
  - `boards/qa/dev-build-cache-and-disk-hygiene.md` (INFRA-001) evidence text.

## Scope

- Repoint or make path-agnostic every actionable reference to
  `$HOME/repos/opencode-modded-rust`: prefer `$OPENCODE_RUST_REPO` (default
  `~/standards/vendor/opencode-modded-rust`) over a hardcoded home path, so the
  submodule and any intentional dev worktree both resolve. At minimum update
  `AGENTS.md` and `scripts/scopemux-qa-server.sh`.
- Evaluate the untracked `scripts/create-worktree.sh` helper (preserved outside
  the repo; see below). It wraps the existing `setup-worktree-build-cache.sh`
  and `install-git-hooks.sh`; decide whether to land it, fold it into the docs,
  or drop it.
- Investigate further space reductions: recurring `target/` growth in the vendor
  checkout and `.shared-target`, whether `third_party/scopemux-core` should be
  reclaimed between builds, and whether `scripts/clean-build-caches.sh` should
  also cover the vendor checkout's own `target/`.
- Confirm `ort`/`ort-build` build and run from the vendor checkout on this
  machine after the removal.

## Cleanup after

- Once paths are repointed, delete the stale references and verify no alias,
  export, `.pth`, or doc still names `~/repos/opencode-modded-rust`.
- Run `scripts/clean-build-caches.sh` (dry run, then `--yes` / `--include-main`
  as appropriate) and record before/after free-space numbers.
- Decide the fate of the preserved legacy clone artifacts (see below) and remove
  them once no longer needed.

## Preserved legacy artifacts (not in this repo)

Before removing the clone, its only local-only state was preserved under
`~/.local/share/legacy-clone-preserve/opencode-modded-rust-2026-10-08/`:
untracked `create-worktree.sh`, four stash patches (see `stash-list.txt`), and
provenance files. Most notable is `stash-3.patch` (provider-dialog selection
styling plus `AGENTS.md` provider-auth notes) which is not in `HEAD`. Review,
apply, or discard; then delete the preserve directory.

## Implementation Notes - 2026-10-08 (OIZG-REM-034)

Path alignment:

- `AGENTS.md` "Local Launchers": the `ort-build`, PID-targeting, and
  worktree-cache bullets now reference `$OPENCODE_RUST_REPO` (default
  `$HOME/standards/vendor/opencode-modded-rust`) instead of the removed
  `$HOME/repos/opencode-modded-rust`.
- `scripts/scopemux-qa-server.sh`: usage comment and the `REPO` fallback now
  default to the vendor path.

Setup bug found and fixed:

- `scripts/clean-build-caches.sh` and `scripts/setup-worktree-build-cache.sh`
  derived the repo name from `dirname(git-common-dir)`. Under the submodule that
  dir is `~/standards/.git/modules/vendor`, so the name resolved to `vendor` and
  the scripts pointed at `~/worktrees/vendor/.shared-target` instead of
  `~/worktrees/opencode-modded-rust/.shared-target`. Both now resolve the main
  checkout via `core.worktree` when present (submodule), else
  `dirname(git-common-dir)` (normal checkout or its linked worktree).
- Verified from the submodule main checkout, a submodule linked worktree, and a
  normal checkout: all resolve to `opencode-modded-rust` and the correct shared
  target. Regenerating `~/worktrees/opencode-modded-rust/.cargo/config.toml`
  produced identical content.

Verification:

- `bash -n scripts/scopemux-qa-server.sh`; `sh -n` on both cache scripts.
- `OPENCODE_RUST_REPO` resolves to the vendor submodule; `ort-build` is present;
  `target/debug/opencode --help` exits 0.
- `scripts/clean-build-caches.sh` dry run: no duplicated per-worktree `target/`
  dirs; shared worktree target `~/worktrees/opencode-modded-rust/.shared-target`
  is ~6.3 GB; main checkout `target/` ~4.8 GB; `third_party` ~0.5 GB. No deletion
  performed (document-only per this card).

## PR

- https://github.com/cchris-p/opencode-modded-rust/pull/137 — merged into
  `development` (merge commit `dbbb60f`) on 2026-10-08; branch
  `chore/INFRA-004-align-setup-paths` deleted remotely and locally.

## Closeout - 2026-10-08

Merged into `development` via PR #137 (`dbbb60f`); PR branch deleted remotely
and locally; local `development` fast-forwarded to the merge. Remains in `qa`
until a QA report is recorded or the operator explicitly completes it. The
`create-worktree.sh` / preserved-stash decisions are intentionally deferred to
`INFRA-005`.

## Done when

- No actionable file or script in this repo names `~/repos/opencode-modded-rust`
  (historical transcripts and closed-card evidence may remain).
- `ort`/`ort-build` work from the vendor checkout with no stale-path errors.
- The `create-worktree.sh` and preserved-stash decisions are recorded and the
  preserve directory is either emptied of pending items or removed.
- A cleanup pass with `clean-build-caches.sh` is recorded with before/after
  free-space numbers.

## Related Items

- `INFRA-001` contain build and data cache disk growth across worktrees
  (shared worktree cache + `clean-build-caches.sh`; this card extends it to the
  submodule checkout and stale paths).
- `STD-021` in `~/standards` — clean up standalone clones superseded by managed
  submodules.

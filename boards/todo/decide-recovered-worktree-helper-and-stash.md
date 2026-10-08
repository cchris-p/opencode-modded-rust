---
id: "INFRA-005"
title: "Decide the fate of a recovered worktree helper and an abandoned provider-dialog stash"
priority: "P3"
type: "chore"
area: "INFRA"
spec: ""
status: "todo"
created: "2026-10-08"
---

# Decide the fate of a recovered worktree helper and an abandoned provider-dialog stash

## Summary

Removing the legacy standalone clone `~/repos/opencode-modded-rust` on host
**OIZG-REM-034** (macOS operator machine) surfaced two pieces of local-only state
that exist nowhere on a remote. Both were preserved instead of discarded and now
need an explicit decision. This card is host-scoped: the preserved copies live on
`OIZG-REM-034` only, at
`~/.local/share/legacy-clone-preserve/opencode-modded-rust-2026-10-08/`.

Split out from `INFRA-004`, which owns the checkout/path alignment.

## Item 1 — `scripts/create-worktree.sh` (untracked helper)

A complete, untracked helper script that was never committed. It creates a git
worktree at `$OPENCODE_WORKTREE_ROOT/<repo>/<name>` (optionally branching from a
base ref), then calls the existing `scripts/setup-worktree-build-cache.sh` and
`scripts/install-git-hooks.sh`, and copies the local (gitignored) `opencode.json`
into the worktree.

- Preserved copy: `create-worktree.sh` in the preserve directory.
- Decision needed: land it in this repo, fold its behavior into the existing
  setup scripts/docs, or drop it.
- Open question (raised by the operator): is this actually tied to an existing
  setup-script item rather than being new work? Candidates to check before
  deciding: `INFRA-001` (worktree build cache), `INFRA-002` (setup preflight
  automation), and the standards-side `ort-setup` / `opencode-config` launcher
  flow referenced by `STD-021`. Confirm whether it duplicates or complements
  those before landing.

## Item 2 — `stash-3.patch` (abandoned provider-dialog work)

A stashed change from before a `development` QA merge, not present in `HEAD`:

- `crates/opencode-tui/src/components/dialogs/provider.rs` — provider-dialog
  list selection styling (bold/reverse selected row with a marker) and a switch
  to `render_stateful_widget`.
- `AGENTS.md` — provider-auth notes (`auth.json` path, `OPENAI_API_KEY` loader,
  stale-server caveat).

- Preserved copy: `stashes/stash-3.patch` in the preserve directory.
- Decision needed: apply and evaluate against current provider-dialog code (it
  predates later UI changes, so a clean apply is unlikely), or discard as
  superseded.

## Other preserved stashes (likely disposable)

- `stash-0.patch` — `BUG-039` board-lane move; marked superseded by a branch card.
- `stash-1-include-untracked.patch` — a large one-off session transcript
  (`execute-board-item-to-pr-on-bug-023.md`); disposable log.
- `stash-2.patch` — edits to `boards/todo/copy-cline-style-cli-task-send-conventions.md`,
  a card that no longer exists in `HEAD`; superseded.

Confirm and discard these unless one is found to carry unique value.

## Scope / non-goals

- In scope: decide and record each item's outcome (land, fold, or discard).
- Non-goals: the checkout/path repointing and disk-cache work, which belong to
  `INFRA-004`.

## Done when

- Each preserved item is either landed in the repo or explicitly discarded, with
  the decision recorded here.
- The `OIZG-REM-034` preserve directory is removed once nothing is pending.

## Related Items

- `INFRA-004` align checkout/setup paths with the standards-managed submodule
  and trim disk footprint (source of these recovered artifacts).
- `INFRA-001` contain build and data cache disk growth across worktrees.
- `INFRA-002` automate scopemux-core fetch, drift detection, and pin bumps.
- `STD-021` in `~/standards` — clean up standalone clones superseded by managed
  submodules (legacy-clone removal; references the standards `ort-setup` flow).

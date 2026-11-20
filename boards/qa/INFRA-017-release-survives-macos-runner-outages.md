---
id: "INFRA-017"
title: "Make main releases survive hosted macOS runner outages"
priority: "P1"
type: "chore"
area: "INFRA"
spec: "invariants/release-deployment.md"
status: "qa"
created: "2026-11-20"
updated: "2026-11-20"
---

# Make main releases survive hosted macOS runner outages

## Summary

Every release so far (`v0.1.0`, `v0.1.3`, `v0.1.4`, `v0.1.5`) needed a human to rebuild the
macOS assets by hand because the hosted `macOS` runner was queued past the job timeout and
then cancelled, which in turn cancelled the Linux job (`fail-fast: true`) and skipped
`publish`. Separately, the agent `GH_TOKEN` has no `actions:write`, so the failed run could not
even be re-run from the CLI.

This is the tracking card for the gap that was noted on `INFRA-011` and in
`docs/release-deployment.md` without an owner.

## Root Cause

1. `release.yml` build matrix is `fail-fast: true`; a cancelled macOS job cancels the Linux
   job, so nothing is publishable.
2. There is no automatic retry. Hosted macOS capacity is transient, so a delayed retry of the
   same run would usually succeed.
3. `publish` fails hard when the release tag already exists, so a naive full rerun is not
   idempotent.
4. The agent token lacks `actions:write`, so manual `gh run rerun` is unavailable.

## Fix

- Add `release-retry.yml`: triggered by `workflow_run` on `Release` completion, retries a
  failed or cancelled `main` release up to 3 attempts, after a delay that lets macOS capacity
  recover. It uses the run's built-in `GITHUB_TOKEN` with `permissions: actions: write`, so no
  personal token permission is required for the automatic path.
- Make the `publish` step idempotent: if the tag and a matching release already exist, exit 0;
  upload missing assets with `--clobber`; create with `--verify-tag` when the tag exists but
  the release does not. This makes a full rerun always safe.
- Set `fail-fast: false` so a macOS cancellation leaves the Linux job and its diagnosis intact.
- Document the automatic retry and the optional `actions:write` grant for manual reruns.

## Acceptance Criteria

- A `Release` run that ends `failure`/`cancelled` on `main` is retried automatically at least
  once, bounded so it cannot loop.
- A retry of a partially published release does not fail on an existing tag.
- A cancelled macOS job no longer cancels the Linux build.
- No personal token permission is required for the automatic recovery path.

## Verification

- YAML parses (`ruby -ryaml`).
- Retry decision logic unit-checked with a fake run payload where possible.
- End-to-end proof: the next real `main` release, including one deliberately allowed to hit a
  macOS queue timeout if one occurs; watch the retry workflow.

## Related Items

- `INFRA-011` automated versioned release.
- `INFRA-012` automatic fleet deploy (consumes the release; retries do not change the tag).
- `INFRA-014` verification and rollback.

## Implementation (2026-11-20)

- `.github/workflows/release-retry.yml` (new): `workflow_run` on `Release` completion; when
  the run ended `failure`/`cancelled` on `main`, it waits 10 minutes for hosted capacity and
  re-runs the run. Bounded to attempt 3, skips runs whose `publish` job already succeeded, and
  uses its own `GITHUB_TOKEN` with `permissions: actions: write`.
- `.github/workflows/release.yml`: `fail-fast: false` on the build matrix; the publish step is
  idempotent (existing release for the same commit is verified and missing assets uploaded;
  a tag without a release is created with `--verify-tag`; a tag for a different commit still
  fails with a clear error).
- `docs/release-deployment.md` documents the retry, the attempt bound, the idempotent
  publish, and the `workflow_run` one-release lead time.
- `AGENTS.md` records the retry rule.

Verification:

- Both workflow files parse as YAML.
- Every `run:` block passes `bash -n`.
- The publish-decision predicate is exercised with true/false sample job payloads.

Remaining: prove it on a real `main` release (the retry workflow only activates once this
file is on `main`).

## Merge Closeout — 2026-11-20

- PR #207 merged into `development` as merge commit `a059d09` with all required checks green
  (`fmt`, `check`, `clippy`, `test`, `native (ubuntu-22.04)`); PR state `CLEAN`.
- Branch `feature/INFRA-016-release-retry` deleted remotely and locally; worktree removed and
  `origin` pruned.
- Card stays in `qa`: the retry workflow only fires once it is on `main`, so proof is the next
  real release. Manual asset recovery remains the documented last resort until then.

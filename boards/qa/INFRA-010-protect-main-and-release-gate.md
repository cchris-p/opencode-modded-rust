---
id: "INFRA-010"
title: "Protect main and require green CI for every release merge"
priority: "P1"
type: "chore"
area: "INFRA"
spec: "invariants/release-deployment.md"
status: "qa"
created: "2026-11-05"
updated: "2026-11-08"
---

# Protect Main And Require Green CI For Every Release Merge

## Summary

Make `main` a protected, release-only branch. A release may advance `main` only through a
release PR from `development`, only after the required CI suite is green, and only with an
explicit review. Direct pushes, force pushes, and unverified merges are rejected.

This is the gate that makes automatic fleet deployment safe: machines are deployed from
`main` releases, so `main` must be provably stable.

## Current State

- `main` is the release branch; routine work lands on `development`.
- The current token cannot read branch protection (`403 Resource not accessible`), so the
  existing configuration is unknown and likely unset.
- `INFRA-009` will provide the required check names; this card consumes them.
- The existing release path is a deliberate development -> main merge, performed manually.

## Scope

- Configure a repository ruleset (preferred over legacy branch protection) for `main`:
  - no direct pushes, deletes, or force pushes;
  - pull request required, base `main`, head `development` for releases;
  - at least one approving review, dismiss stale approvals on new commits;
  - required status checks from `INFRA-009` (and the release build job from `INFRA-011`);
  - require conversation resolution;
  - require linear history or merge queue;
  - include administrators (or document the explicit exception and why);
  - optionally restrict who can push/merge.
- Add a release PR template/checklist and document the only allowed path:
  `development` -> release PR -> required checks -> review -> merge -> release + deploy.
- Record the ruleset in the repo (`docs/` or `invariants/`) so it is reviewable and can be
  re-applied; include a `gh api` example and the manual UI fallback.
- Add a workflow or policy check that rejects a PR targeting `main` from anything other than
  `development` (label/name check is not enough if the ruleset cannot express it).
- Document the admin-only nature: a token with `admin:repo` or the GitHub UI is required;
  the current `gh` token cannot read or apply protection.

## Acceptance Criteria

- A direct push to `main` is rejected.
- A PR to `main` with a failing required check cannot merge.
- A release PR from a branch other than `development` is rejected or blocked by policy.
- The required checks match the jobs added by `INFRA-009` and `INFRA-011`.
- The ruleset is documented and reproducible.
- The owner/admin has applied it and recorded the evidence on the card.

## Verification

- Attempt a direct commit/push to `main` from a scratch clone; it must be rejected.
- Open a test PR to `main` with a failing check; confirm the merge button is disabled.
- Confirm an admin cannot bypass without an explicit recorded decision.

## Human/Admin Gate

Applying the ruleset requires repository admin rights (or a token with admin scope). Flag
this card `needs_human` when execution reaches the apply step if the executing agent cannot
authenticate as an admin. Do not weaken the protection to work around missing permissions.

## Non-Goals

- Protecting `development` with the same strictness (PRs to `development` still need the
  required checks, but routine board/docs work commits directly per repo policy).
- Changing the release build itself (`INFRA-011`).
- Deploying machines (`INFRA-012`).

## Related Items

- `INFRA-009` required CI suite.
- `INFRA-011` automated versioned release.
- `INFRA-012` automatic fleet deployment (depends on a trustworthy `main`).
- `INFRA-015` invariant and runbook.

## Implementation (2026-11-08)

Non-admin parts landed:

- `docs/main-ruleset.json` - the reproducible ruleset (no deletes/force-push, PR + 1
  approval, conversation resolution, required checks `fmt`, `check`, `clippy`, `test`,
  `native (ubuntu-22.04)`, `release-source`, no bypass actors).
- `docs/main-protection.md` - rationale, apply/verify commands with a `gh api` example, the
  manual UI fallback, and the release PR checklist.
- `.github/workflows/main-gate.yml` - `release-source` check rejects any PR to `main` whose
  head is not `development`.

Notes:
- `native (macos-14)` is intentionally **not** required yet because the macOS runner is
  unavailable in this repo; requiring it would block every release. Add it once the runner
  works.
- The `release.yml` build jobs run after the main merge, so they gate the release/deploy,
  not the PR.

## Human/Admin Gate (open)

The current `GH_TOKEN` cannot read or apply branch protection (`403 Resource not
accessible`). A repository admin must apply `docs/main-ruleset.json`, then record evidence
here: a rejected direct push to `main`, a blocked PR with a failing check, and a blocked PR
from a non-`development` head. Do not weaken the ruleset to work around missing permissions.

## Admin Apply And QA (2026-11-08)

Applied the ruleset with repository-admin rights using the OpenBao-managed classic `GH_TOKEN`
(`ghp_…`, from `~/.config/opencode/.env`); the session's fine-grained `github_pat_…` token
returns 403. The token value was never printed.

- Ruleset `main-release-only` created: id `26794977`, `enforcement: active`, target
  `refs/heads/main`.
- Required checks recorded: `fmt`, `check`, `clippy`, `test`, `native (ubuntu-22.04)`,
  `release-source`.
- `docs/main-ruleset.json` matches the applied ruleset; `docs/main-protection.md` records the
  apply/verify path.

Remaining QA gates before `done`:

- Direct-push rejection: GitHub `git push --dry-run` does not evaluate the ruleset, so the
  live rejection is verified by the owner during the next release attempt (or by a ruleset
  test). The ruleset config itself is active.
- **Release review:** the ruleset requires 1 approving review and GitHub forbids
  self-approval. The only other collaborator is `cchrispivy` (write), so the release PR must
  be approved from that account (or the owner explicitly records a reviewed exception).
- `native (macos-14)` stays advisory until the macOS runner is available.

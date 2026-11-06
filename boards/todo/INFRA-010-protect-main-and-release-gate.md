---
id: "INFRA-010"
title: "Protect main and require green CI for every release merge"
priority: "P1"
type: "chore"
area: "INFRA"
spec: "invariants/release-deployment.md"
status: "todo"
created: "2026-11-05"
updated: "2026-11-05"
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

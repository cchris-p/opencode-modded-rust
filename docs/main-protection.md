# `main` Protection

`main` is protected and release-only (`invariants/release-deployment.md`). This document is
the reviewable, re-appliable record of the GitHub ruleset. Applying it requires a repository
admin token (or the GitHub UI); the default fine-grained `GH_TOKEN` cannot read or apply
branch protection (`403 Resource not accessible by personal token`).

## Applied Ruleset

Applied 2026-11-08 as ruleset id `26794977` (`main-release-only`, enforcement `active`,
target `refs/heads/main`). Source of truth: `docs/main-ruleset.json`.

- block deletes and force pushes on `main`;
- require a pull request with at least one approving review; dismiss stale approvals on new
  pushes; require conversation resolution;
- require the status checks `fmt`, `check`, `clippy`, `test`, `native (ubuntu-22.04)`, and
  `release-source`;
- require branches to be up to date before merge (`strict_required_status_checks_policy`);
- no bypass actors (administrators are included).

### Why `native (macos-14)` is not required yet

The macOS native job is currently cancelled/unavailable in this repository (the same
pre-existing `native-artifacts` runner failure). Requiring it would block every release.
Add it to the required list once the macOS runner is available; until then it remains an
advisory check.

### Why the release build job is not a PR check

The `release.yml` build jobs run on a push to `main`, after the release PR merges, so they
cannot gate the PR itself. They gate the release/deploy (a failing build fails the release).

## Apply

```bash
# Create (returns an id)
gh api --method POST repos/:owner/:repo/rulesets --input docs/main-ruleset.json

# Or update an existing ruleset
gh api --method PUT repos/:owner/:repo/rulesets/<id> --input docs/main-ruleset.json

# Verify
gh api repos/:owner/:repo/rulesets
```

Manual UI fallback: Settings -> Rules -> Rulesets -> New branch ruleset, target `main`, and
add the rules above.

## Release PR Checklist

Use for every PR from `development` to `main`:

- [ ] `development` is green and current; the merge commit is the intended release tip.
- [ ] Required checks are green (`fmt`, `check`, `clippy`, `test`, `native (ubuntu-22.04)`,
      `release-source`).
- [ ] At least one approving review; conversations resolved.
- [ ] `[workspace.package] version` is bumped if `v<version>` already exists.
- [ ] The owner explicitly confirmed this specific release (a bare "update main" is not
      confirmation).
- [ ] After merge: confirm the `Release` workflow published the `v<version>` release and its
      manifest, then confirm the fleet deploy/verification result.

## Review Requirement

A release PR needs one approving review, and GitHub does not allow an author to approve their
own PR. The other collaborator is `cchrispivy` (write); the release PR must be approved from
that account, or the owner records an explicit reviewed exception. Do not silently reduce the
approval count to work around this.

## Verification

- A direct push to `main` is rejected.
- A PR to `main` with a failing required check cannot merge.
- A PR to `main` whose head is not `development` fails `release-source`.

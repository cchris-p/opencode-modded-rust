# Release And Fleet Deployment Invariants

These are binding truths for how `main` becomes a release and how a release reaches the
fleet. The operating steps live in `docs/release-deployment.md`. The work that implements
these rules is tracked by `INFRA-009` through `INFRA-015`.

## Definitions

- **Build**: artifacts produced from a commit (native bundle, CLI tarball, checksums,
  `.rev`, manifest).
- **Release**: an immutable, versioned publication of a `main` build (tag + assets +
  manifest + release notes).
- **Preview**: a rolling, non-versioned publication of a `development` build. It is not a
  release and is never deployed to the fleet automatically.
- **Deploy**: installing a specific release revision on a machine and verifying it.
- **Active registered machine**: a machine present in `standards-machines` whose
  `standards-machine-status` is `active`.
- **Complete**: the release is published and every active registered machine has the
  release revision installed and has passed post-deploy verification.

## Rules

1. **`main` is protected and release-only.** No direct pushes, deletes, or force pushes.
   `main` advances only through a release PR whose head is `development`.
2. **A release PR requires green required checks and an explicit review.** A failing or
   missing required check blocks the merge. Administrators are not silently exempt.
3. **Every merge to `main` produces an immutable versioned release.** The release includes
   the native provider bundle, CLI binaries for Linux and macOS, checksums, `.rev` sidecars,
   a machine-readable manifest (commit, scopemux pin, query-set key, asset names,
   checksums), and release notes. It is never overwritten in place.
4. **Only `main` releases deploy to the fleet.** `development` and preview builds refresh
   the rolling preview channel only; they never auto-deploy to registered machines.
5. **Deployment is part of release completion, not a manual follow-up.** Every successful
   `main` release is installed on every active registered machine.
6. **A release is not complete until every active machine verifies it.** A failing machine
   fails the deploy and the release is marked not fully deployed; the previous revision is
   restored on affected machines.
7. **Inactive machines are reported and skipped intentionally.** `pending`, `disabled`, and
   `retired` machines are listed with their lifecycle state; they are never silently
   dropped from the deploy report.
8. **Deployment uses the registered-machine SSH/OpenBao model with least privilege.**
   Mesh keys, OpenBao tokens, and other fleet credentials are never exposed to
   GitHub-hosted runners. The deploy control plane runs on a designated control host that
   already holds the fleet credentials.
9. **Deploys are serialized.** Overlapping releases cannot deploy concurrently; a new
   release waits for or aborts the previous rollout.
10. **The stable channel is the default.** `ort-install` and `opencode-config` follow the
    latest stable `main` release by default. Preview is explicit opt-in per machine, and a
    stable-configured machine never follows the rolling preview tag.
11. **Exact revisions remain installable.** `ort-install --rev <sha>` is available to the
    deploy driver and to operators for pins and rollback.
12. **Verification is read-only and bounded.** It never mutates task state, session data, or
    user files.

## Human And Admin Gates

- Applying the `main` ruleset requires repository admin rights (`INFRA-010`).
- Choosing the deploy control host and fleet credential path requires owner approval
  (`INFRA-012`).
- Advancing `main` from `development` is a production release and always requires the
  owner's explicit confirmation for that specific release. A bare "update main" is not
  sufficient.

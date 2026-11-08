---
id: "INFRA-015"
title: "Capture the main-release fleet deployment invariant and runbook"
priority: "P1"
type: "docs"
area: "INFRA"
spec: "invariants/release-deployment.md"
status: "doing"
created: "2026-11-05"
updated: "2026-11-08"
---

# Capture The Main-Release Fleet Deployment Invariant And Runbook

## Summary

Make the release and deployment policy an explicit project invariant and a runbook, so the
automation in `INFRA-009`..`INFRA-014` has a durable standard to conform to.

## Invariant To Capture

1. `main` is protected and release-only: no direct pushes, required green CI, review, and a
   `development` -> `main` release PR.
2. Every merge to `main` produces an immutable versioned release (native bundle + CLI
   assets + checksums + provenance).
3. Every successful `main` release is automatically deployed to every **active registered
   machine**; deployment is part of release completion, not a manual follow-up.
4. A release is not complete until every active machine verifies the new revision. A
   failing machine fails the release and triggers rollback to the previous revision.
5. `development`/preview builds never auto-deploy to the fleet. The rolling
   `native-artifacts` tag is preview-only.
6. Inactive machines (`pending`, `disabled`, `retired`) are reported and skipped
   intentionally, never silently dropped.
7. Deployment uses the registered-machine SSH/OpenBao model and least-privilege
   credentials; mesh keys and OpenBao tokens are never exposed to GitHub-hosted runners.

## Current State

- No such invariant exists.
- The current rolling tag and auto-update behaviour effectively deliver development builds
  to machines, which contradicts the intended policy.
- Branch protection is unset/unknown and no test workflow exists.

## Scope

- Add `invariants/release-deployment.md` with the rules above and the exact definitions of
  "build", "release", "deploy", "active registered machine", and "complete".
- Update `AGENTS.md` with the short operational rule and pointers.
- Update `README.md` with the stable/preview channels and the main-release deployment path.
- Add a runbook (for example `docs/release-deployment.md`) covering:
  - how to cut a stable release;
  - how to read the release manifest;
  - how to pause a rollout;
  - how to roll back;
  - how to add/disable a machine and how lifecycle state is honored;
  - the admin steps for the `main` ruleset.
- Update the `standards` repo docs/AGENTS where the fleet fan-out and `ort-install`
  channels are described.
- Cross-reference the invariant from `INFRA-009`..`INFRA-014`.

## Acceptance Criteria

- The invariant file exists and states the rules unambiguously.
- `AGENTS.md` and `README.md` point at it.
- A runbook moves a release from `development` to a verified fleet deploy without
  undocumented manual steps.
- The standards repo documentation matches the channel/deploy behavior.
- The invariant is referenced by every implementation card in the program.

## Verification

- Read the invariant and runbook against `INFRA-009`..`INFRA-014`; every rule maps to an
  enforcement point.
- Confirm a new agent can follow the runbook and identify the human/admin gates.

## Non-Goals

- Implementing CI, protection, release, deploy, or verification.
- Changing the current product behavior before the implementation cards land.

## Related Items

- `INFRA-009` required CI.
- `INFRA-010` protect main.
- `INFRA-011` versioned release.
- `INFRA-012` automatic fleet deploy.
- `INFRA-013` stable channel.
- `INFRA-014` verification and rollback.

## Implementation (2026-11-08)

- Added `invariants/release-deployment.md` with the definitions (build, release,
  preview, deploy, active registered machine, complete), the twelve binding rules,
  and the human/admin gates.
- Added `docs/release-deployment.md` runbook: channels, cut a release, read the
  manifest, deploy (canary then fan-out), verify, pause, roll back, add/disable a
  machine, and the `main` ruleset admin steps. Manual interim steps are marked
  where `INFRA-009`..`INFRA-014` have not landed yet.
- Updated `AGENTS.md`, `README.md`, and `invariants/README.md` with the rule and
  pointers.
- Cross-referenced `invariants/release-deployment.md` from `INFRA-009`..`INFRA-014`.

Remain in `doing` until `INFRA-009`..`INFRA-014` land, then refine the invariant
and runbook to match the shipped automation and move to QA/done.

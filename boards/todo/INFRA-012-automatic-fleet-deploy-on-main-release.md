---
id: "INFRA-012"
title: "Automatically deploy every main release to all registered machines"
priority: "P1"
type: "feature"
area: "INFRA"
spec: "invariants/release-deployment.md"
status: "todo"
created: "2026-11-05"
updated: "2026-11-05"
---

# Automatically Deploy Every Main Release To All Registered Machines

## Summary

Make fleet deployment part of release completion, not a manual follow-up. Every successful
`main` release is automatically installed on **all active registered machines**, and the
release is not considered complete until every machine reports the new revision and passes
verification.

Interpretation locked with the owner: only `main` releases deploy. `development`/preview
builds refresh the preview channel but never auto-deploy to the fleet.

## Current State

- `standards/standards-machines` is the inventory; lifecycle state (`active`, `pending`,
  `disabled`, `retired`) is read from OpenBao with `standards/standards-machine-status`.
- `standards/utilities/ort-install` installs a pinned release revision into
  `~/.local/share/ort/bin/<rev>/` and maintains `current`.
- `standards/opencode-config` auto-installs/auto-updates `ort` at launch, currently against
  the rolling `native-artifacts` tag.
- There is no programmatic fan-out that installs a specific release on every machine and
  verifies it.
- Deployment requires the registered-machine SSH/OpenBao access model; there is no
  GitHub-hosted path to the mesh today.

## Scope

- Choose the deploy control plane and record the decision:
  - recommended: a self-hosted GitHub Actions runner or a release-triggered systemd job on a
    designated control host (`matrillobase1`) that already has the SSH mesh and OpenBao
    access; or
  - a `standards` deploy driver invoked by the release workflow over SSH.
  - Do not expose mesh keys or OpenBao tokens to GitHub-hosted runners.
- Add a deploy driver (likely `~/standards/utilities/standards-deploy-ort` or a
  `scripts/` script in this repo) that:
  - reads the release manifest from `INFRA-011`;
  - enumerates `standards-machines`, filters by `standards-machine-status` (`active` only;
    report and skip `pending`/`disabled`/`retired` explicitly);
  - deploys a canary machine first, verifies it, then fans out to the rest;
  - installs the pinned revision (`ort-install --rev <sha>` or a stable-channel equivalent);
  - runs the `INFRA-014` verification on each machine;
  - records a per-machine report (name, previous rev, new rev, status, duration, error);
  - fails the release/deploy if **any** active machine fails; retries transient SSH/network
    failures once;
  - serializes deploys and refuses overlapping releases;
  - supports an explicit rollback to the previous revision.
- Trigger the deploy automatically after the stable release is published. Do not require a
  human to run it. A human may still pause a rollout through an explicit release gate.
- Never deploy development/preview builds. A `native-artifacts` preview refresh must not
  invoke the fleet deploy.
- Update `standards` docs/AGENTS and this repo's invariant/runbook with the operating model.

## Acceptance Criteria

- A successful `main` release automatically installs that release on every active
  registered machine.
- No active machine is skipped silently; skipped inactive machines are listed with reasons.
- Any machine failure fails the deploy job and the release is marked not fully deployed.
- A canary machine is verified before the fan-out.
- Overlapping releases cannot run concurrently.
- Development/preview builds never trigger fleet deployment.
- A rollback restores the previous revision on every affected machine.
- The deploy report is retained as a build artifact and summarized on the release.

## Verification

- In a controlled test, publish a throwaway release and confirm the deploy driver installs
  it on the canary and then the fleet (or a scoped subset via an explicit dry-run mode).
- Break one machine's access and confirm the deploy fails, reports that machine, and does
  not report success.
- Confirm a development build does not trigger the deploy.
- Confirm `ort-doctor` on a sample machine shows the expected release revision.

## Human/Admin Gate

The control-host runner, SSH mesh credentials, and OpenBao access are security-sensitive.
Record the control host, credential path, and least-privilege scope before enabling
automatic deployment. If the release workflow cannot reach the mesh, stop at the decision
step rather than weakening key handling.

## Non-Goals

- Client auto-update policy (`INFRA-013`).
- Post-deploy checks themselves (`INFRA-014`).
- Deploying to unregistered/supplementary hosts or NAS appliances.
- Cross-machine map data sharing.

## Related Items

- `INFRA-007` published CLI and `ort-install`.
- `INFRA-011` release manifest.
- `INFRA-013` stable channel.
- `INFRA-014` verification and rollback.
- `INFRA-015` invariant and runbook.

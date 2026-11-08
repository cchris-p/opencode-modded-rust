---
id: "INFRA-014"
title: "Verify every fleet deployment and roll back on failure"
priority: "P1"
type: "feature"
area: "INFRA"
spec: "invariants/release-deployment.md"
status: "qa"
created: "2026-11-05"
updated: "2026-11-08"
---

# Verify Every Fleet Deployment And Roll Back On Failure

## Summary

Every machine that receives a release must prove it is running the release correctly.
Add a post-deploy verification pass and an automatic rollback path so a bad release or a
bad machine does not leave the fleet in a mixed, silent state.

## Current State

- `INFRA-008` adds `opencode-modded-rust debug scopemux-queries`, which resolves and
  materializes the embedded query set.
- `ort-install` verifies the download checksum and installs per revision; `ort-doctor`
  reports the installed revision.
- There is no fleet-wide post-deploy verification or rollback driver today.

## Scope

- Add a verification driver (a `standards` utility or a script in this repo) that, for each
  machine:
  - confirms the installed revision equals the release manifest revision;
  - confirms the binary runs (`opencode-modded-rust version`);
  - confirms `debug scopemux-queries` resolves a directory containing the expected
    language query files (for example `rust/docstrings.scm`);
  - optionally runs a bounded, read-only smoke task and checks `provider=scopemux` when the
    local Qwen profile is active;
  - records a machine-readable result and exit status.
- Integrate it into the `INFRA-012` deploy driver:
  - verify the canary before fan-out;
  - verify every machine after install;
  - aggregate results into a deploy report artifact and a release summary;
  - fail the deploy/release if any active machine fails.
- Rollback:
  - on failure, reinstall the previous revision on the affected machines (or the whole
    canary stage) automatically;
  - mark the release as not fully deployed and surface the failing machines;
  - keep the previous revision/asset available until the release is confirmed.
- Keep verification read-only and bounded; no task-state or source mutation.
- Add a dry-run/report-only mode for audit.

## Acceptance Criteria

- Every deployment produces a per-machine verification report.
- The canary must pass before the fleet fan-out begins.
- Any failed machine fails the deploy and is rolled back or clearly blocked with the previous
  revision restored.
- The release summary lists installed revisions, successes, failures, and skipped inactive
  machines.
- `debug scopemux-queries` proves the portable query bundle works on each machine.
- No verification step mutates task state or user data.

## Verification

- Run the verifier against a healthy machine; it reports the expected revision and query
  files.
- Point it at a machine with a mismatched revision and confirm it fails.
- Simulate a failed install on the canary and confirm the rollback restores the prior
  revision.
- Confirm the report is attached to the release/deploy artifact and summarized.

## Non-Goals

- The deploy fan-out itself (`INFRA-012`).
- Full end-to-end model/tool evaluation on every machine (that remains a separate QA pass).
- Monitoring/alerting integration (candidate follow-up with Zabbix/Healthchecks).

## Related Items

- `INFRA-008` portable query bundle and `debug scopemux-queries`.
- `INFRA-012` automatic fleet deploy.
- `INFRA-015` invariant and runbook.

## Implementation (2026-11-08)

Implemented in `~/standards` (commit `d34ed93` on `development`):

- `utilities/ort-verify`: read-only. Checks the installed revision (`bin/current`),
  `opencode-modded-rust version`, and `debug scopemux-queries` resolving
  `rust/docstrings.scm`. Emits human or tab-separated output; non-zero on failure.
- Integrated into `utilities/standards-deploy-ort`: verifies the canary before fan-out,
  verifies every active machine after install, rolls back failed machines to their previous
  revision, and records a per-machine report (TSV).

Local check on `matrillobase1`: `ort-verify` correctly fails the currently installed
pre-INFRA-008 binary (no `debug scopemux-queries`), which is expected until the first release
with the embedded query set is deployed.

Remaining QA gate: run the verifier against the fleet after the first v0.1.0 deploy and
confirm every active machine passes.

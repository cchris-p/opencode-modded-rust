# Where We Left Off - 2026-11-08 (H-019 in progress)

## H-019 Main-Release Fleet Deployment Program

Merged into `development`:

- `INFRA-015` - `invariants/release-deployment.md` + `docs/release-deployment.md` runbook,
  AGENTS/README pointers.
- `INFRA-009` - `.github/workflows/ci.yml` (fmt/check/clippy/test/native + portable query
  smoke); repaired two stale architecture-guard tests. Card in `qa`.
- `INFRA-011` - `.github/workflows/release.yml` immutable `v<version>` release + manifest.
  Card in `qa`.
- `INFRA-010` - `docs/main-ruleset.json`, `docs/main-protection.md`,
  `.github/workflows/main-gate.yml` (`release-source`). Ruleset `main-release-only`
  (id `26794977`) applied to `main` with the OpenBao classic `GH_TOKEN`. Card in `qa`.

Implemented in `~/standards` (commit `d34ed93`):

- `INFRA-013` stable ORT channel (stable default, preview opt-in) - card in `qa`.
- `INFRA-014` `utilities/ort-verify` + verification integrated in the deploy driver - `qa`.
- `INFRA-012` `utilities/standards-deploy-ort` (canary, verify, rollback, inactive report,
  serialized) - card in `doing`.

## Remaining

1. Release PR `development -> main` for `v0.1.0`. The ruleset requires 1 approving review and
   GitHub forbids self-approval: approve from `cchrispivy` (or record a reviewed exception).
2. After the release publishes, run on `matrillobase1`:
   `~/standards/utilities/standards-deploy-ort --rev v0.1.0` (dry-run first).
3. Confirm every active machine passes `ort-verify`; `webappssub2` is `disabled` and is
   reported/skipped.
4. Wire a release-triggered control-plane job on `matrillobase1` (systemd unit/timer or a
   self-hosted runner). Hosted runners must never receive fleet credentials.

## Known Issues

- `native (macos-14)` CI fails at ~0s: the macOS runner is unavailable in this repo
  (pre-existing `native-artifacts` failure). Linux is authoritative for now.
- `git push --dry-run` does not evaluate the main ruleset, so the live direct-push rejection
  is verified during the next release attempt (or by a ruleset test).

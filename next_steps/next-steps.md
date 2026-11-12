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

## Released And Deployed (2026-11-08)

- The approval requirement was removed from the main ruleset (`required_approving_review_count:
  0`); `main` still requires a PR + required checks + conversation resolution.
- Release PR #175 merged (`eeb608f`); `v0.1.0` published
  (https://github.com/cchris-p/opencode-modded-rust/releases/tag/v0.1.0) with Linux and
  macOS CLI assets, native bundles, checksums, and `release-manifest.json`.
- macOS asset was built on `j8rsxy` because the GitHub macOS runner is unavailable; the
  automated `publish` job was skipped (see INFRA-011).
- Fleet deploy via `standards-deploy-ort --rev eeb608f...`: 8 active machines verified at
  `0.1.0` (canary `jasper` first), `webappssub2` skipped (`disabled`).
- Report: `~/.local/share/ort/deploy-reports/standards-deploy-ort-20261108T121056Z.tsv`.

## Remaining

1. `moldavite` (offline ~18d) and `xp-farmer` (offline ~1h) could not be deployed; bring them
   online and re-run the deploy, or mark them `pending`/`disabled`.
2. Enable macOS Actions minutes or install a self-hosted macOS runner on `j8rsxy`, so future
   releases do not require the manual macOS build.
3. Wire a release-triggered control-plane job on `matrillobase1` (systemd unit/timer or a
   self-hosted runner). Hosted runners must never receive fleet credentials.

## Known Issues

- `git push --dry-run` does not evaluate the main ruleset, so the live direct-push rejection
  is verified during the next release attempt (or by a ruleset test).

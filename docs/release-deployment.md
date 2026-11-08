# Release And Fleet Deployment Runbook

This runbook moves a verified `development` tip to a stable release and deploys it to the
fleet. It implements `invariants/release-deployment.md`. Automation is provided by
`INFRA-009` (CI), `INFRA-011` (versioned release), `INFRA-012` (fleet deploy), `INFRA-013`
(channels), and `INFRA-014` (verification/rollback). Until a card lands, use the documented
manual interim step and record it on the release.

## Channels

- **stable**: the latest immutable `main` release (`v<version>`). Default for
  `ort-install` and `opencode-config`.
- **preview**: the rolling `native-artifacts` tag, refreshed on `development` pushes.
  Explicit opt-in only (`ort-install --channel preview` /
  `OPENCODE_RUST_CHANNEL=preview`). Never auto-deployed to the fleet.

## Cut A Stable Release

1. Confirm `development` is green and current: the required CI suite passes, and
   `git -C <checkout> log --oneline -1 origin/development` matches the intended tip.
2. Open the release PR `development -> main`. Title it with the version.
3. Wait for the required checks (fmt, check, clippy, test, native, smoke) and an approving
   review. Do not merge on a red or missing check.
4. **Owner gate**: the owner must explicitly confirm this specific release. "Update main"
   alone is not confirmation.
5. Merge the release PR. The release workflow (`.github/workflows/release.yml`, `INFRA-011`)
   builds and publishes the immutable `v<version>` release with Linux + macOS assets,
   checksums, `.rev` sidecars, manifest, and release notes.
6. Confirm the release manifest and all assets are present before deployment starts.

Manual interim (no release workflow yet): build with
`scripts/package-native-artifacts.sh` and `scripts/publish-cli-artifact.sh`, then verify the
uploaded assets and record the commit SHA, scopemux pin, and query-set key on the card.

## Read The Release Manifest

The manifest (release asset, e.g. `release-manifest.json`) records:

- `tag`, `commit`, `version`;
- `scopemux_pin`, `queries_key`;
- per-asset `name`, `sha256`, and platform.

Verify an asset before install:

```bash
sha256sum -c <asset>.sha256
cat <asset>.rev   # must equal the manifest commit
```

## Deploy

`INFRA-012` triggers deployment automatically after the stable release publishes.

1. The deploy driver reads the release manifest.
2. It enumerates `standards-machines`, filters to `active` via
   `standards-machine-status`, and lists skipped machines with their lifecycle state.
3. It installs the pinned revision (`ort-install --rev <sha> --channel stable`) on a
   **canary** machine first and runs `INFRA-014` verification.
4. Only after the canary passes does it fan out to the remaining active machines,
   serially, one release at a time.
5. It records a per-machine report (`name`, `previous_rev`, `new_rev`, `status`,
   `duration`, `error`) as a build artifact and summarizes it on the release.
6. Any active-machine failure fails the deploy and marks the release not fully deployed.

Manual interim: run the deploy driver on the control host with `--dry-run` first, then for
real; never expose fleet credentials to a GitHub-hosted runner.

## Verify

For each machine, `INFRA-014` verifies:

```bash
opencode-modded-rust version
opencode-modded-rust debug scopemux-queries   # resolves a dir containing rust/docstrings.scm
```

and confirms the installed revision equals the manifest commit. Optional bounded
`provider=scopemux` smoke only where the local Qwen profile is active.

## Pause A Rollout

- Set the rollout pause flag the deploy driver honors (recorded in the release/deploy run),
  or stop the driver before fan-out. A paused rollout is reported, not silently abandoned.
- A human may pause at any time; the canary/fan-out state and per-machine report remain the
  source of truth.

## Roll Back

1. Identify the failing machines in the deploy report.
2. Reinstall the previous revision from its still-retained release assets:
   `ort-install --rev <previous_sha> --channel stable`.
3. Re-run `INFRA-014` verification on each affected machine.
4. Mark the release not fully deployed; surface the failing machines on the release and the
   owning board card. Retain the previous revision/asset until the release is confirmed.

## Add Or Disable A Machine

- Add: register in `standards-machines`, record lifecycle `active`, and confirm SSH/OpenBao
  access. The next release deploys to it automatically.
- Disable/retire: set lifecycle to `disabled`/`retired` via `standards-machine-status`. The
  next deploy reports and skips it; it is never silently dropped.
- No release is required to change lifecycle state.

## Admin: `main` Ruleset (`INFRA-010`)

Requires a token with admin scope or the GitHub UI; the default `gh` token cannot read or
apply protection.

Repository ruleset for `main`:

- block direct pushes, deletes, and force pushes;
- require a pull request whose base is `main` and whose head is `development`;
- require at least one approving review; dismiss stale approvals on new commits;
- require the `INFRA-009` check set (and the `INFRA-011` release build job);
- require conversation resolution;
- include administrators (or record an explicit, reviewed exception);
- optionally restrict who may push/merge.

Re-apply example (adjust names to the actual checks):

```bash
gh api --method PUT repos/:owner/:repo/rulesets/<id> --input docs/main-ruleset.json
```

See `docs/main-protection.md` and `docs/main-ruleset.json` for the exact ruleset and a
release PR checklist. A direct push to `main`, a PR with a failing required check, and a PR
whose head is not `development` must all be rejected.

## Human Gates Summary

- Release PR merge: explicit owner confirmation for that release (`INFRA-010`).
- Ruleset application: repository admin (`INFRA-010`).
- Deploy control host + fleet credentials: owner decision (`INFRA-012`).

---
id: "H-021"
title: "Production / Fleet Deployment Handoff (manual interim while INFRA-012 auto-trigger is unfinished)"
status: "open"
created: "2026-11-13"
updated: "2026-11-17"
owner: ""
target: "opencode-modded-rust; standards (ort-install / standards-deploy-ort)"
blocked_reason: ""
needs_human: "Owner gate: every main release and every fleet rollout needs explicit owner confirmation for that release."
items: ["BUG-075", "INFRA-012", "INFRA-014", "INFRA-011", "INFRA-013"]
---

# Production / Fleet Deployment Handoff

> **Status update (2026-11-17):** the payload shipped. `BUG-075` released in `v0.1.4`
> (`main` `cc9befa`), and `v0.1.5` (`main` `4292f41`) has been installed fleet-wide through
> the in-product `/update` path, which the owner accepted as the QA pass. What remains of this
> handoff is only: (a) `INFRA-012`'s release-triggered control plane (owner setup), and
> (b) `moldavite` and `xp-farmer`, which are still Tailscale-offline and need the current
> revision. Do not re-run the release steps below; they have already happened.

This document replaces the generic "ssh to prod-server-1 / ./scripts/deploy.sh" draft. That
draft does not describe this repository. There is **no** `scripts/deploy.sh`, **no**
`docs/deployment.md`, **no** `prod-server-1`, **no** `/opt/opencode/prev`, and **no**
`opencode.service` unit. Deployment here is a `main` release installed onto registered
machines with `ort-install`, driven by `standards-deploy-ort`.

Authoritative sources (read these before acting):

- `docs/release-deployment.md` — the operating runbook.
- `invariants/release-deployment.md` — the non-negotiable rules.
- `boards/doing/INFRA-012-automatic-fleet-deploy-on-main-release.md` — driver state.
- `boards/qa/INFRA-014-post-deploy-fleet-verification-rollback.md` — verification/rollback.

## Goal

Stop ORT from killing healthy long sessions, then get that fix onto the fleet.

The payload is **`BUG-075`** (`boards/doing/BUG-075-session-timeout-vanilla-parity.md`): ORT
added three non-vanilla bounds and this fix makes them vanilla-parity.

- `OPENCODE_RUN_TIMEOUT_MS` default `1800000` (30 min) -> **`0` (disabled)**; opt-in backstop.
- `OPENCODE_STREAM_BUDGET_MS` default `900000` (15 min) -> **`0` (disabled)**; opt-in.
- `OPENCODE_STREAM_IDLE_TIMEOUT_MS` (new) default **`300000` (5 min)**, matching vanilla's
  provider chunk timeout; `0` disables.
- `opencode run --attach` client wait no longer aborts at 30 minutes.

The installed release `v0.1.3` still carries the caps, so the fleet keeps timing out until
`0.1.4` (or the next version) ships this fix. Deployment is therefore the second half of the
job, not a separate project.

## Current State (2026-11-13)

- **`BUG-075` is in flight and uncommitted** at handoff time. Working-tree changes:
  `crates/opencode-provider/src/stream.rs`, `crates/opencode-session/src/prompt.rs`,
  `crates/opencode-server/src/routes.rs`, `crates/opencode-cli/src/main.rs`,
  `README.md`, `AGENTS.md`, `invariants/coding-session-behavior.md`; the card
  `boards/doing/BUG-075-session-timeout-vanilla-parity.md` is untracked. It was owned by
  another live session (`agent-1`, pid 3094102) — do not discard or rewrite it.
- `BUG-075` edits `invariants/coding-session-behavior.md`, changing the "bounded wall-clock"
  invariant to "idle bound by default, wall-clock caps opt-in". That is policy-level and
  needs **owner sign-off** before it lands.
- Current release: **`v0.1.3`**, `main` at `e2334a3`. Release assets published
  (Linux + macOS, checksums, `.rev` sidecars, manifest).
- `FEAT-077` ("show selected model in models menu") is **already merged and released**
  (PR #191 → `1852d38`). The draft's
  `git merge feat/show-selected-model-in-models-menu` step is obsolete; that branch does
  not exist.
- Fleet deploy of `v0.1.3` was run manually and verified on **8/10** active machines:
  `jasper`, `webapps`, `webappssub1`, `matrillobase1`, `matrillo`, `matrillosub1`,
  `matrillosub2`, `j8rsxy`.
- `webappssub2` is `disabled`/skipped. `moldavite` and `xp-farmer` were Tailscale-offline
  and still need the `v0.1.3` revision — bring them online, then re-run the deploy for the
  missing machines.
- `INFRA-012` is still `doing`: the deploy driver exists
  (`~/standards/utilities/standards-deploy-ort`), but the automatic release→deploy trigger
  (release-triggered systemd job / self-hosted runner on `matrillobase1`) is **not wired**.
  Until it is, a human runs the driver on `matrillobase1`. GitHub-hosted runners must never
  receive fleet credentials.

## Prerequisites

1. Work on the dev host `matrillobase1` (SSH mesh + OpenBao). Do not run fleet deploy from
   another host.
2. Confirm the worktree state before any release merge:
   - `git status --porcelain` must be clean for the release commit.
   - At handoff time the worktree is **dirty with `BUG-075`** (the intended payload) plus an
     unrelated `boards/todo/integrate-sketchvlm.md`. Verify and commit `BUG-075` first; do not
     discard it, and do not fold in unrelated files.
3. Confirm the target release commit and that its manifest/assets exist.
4. Owner confirmation for this specific `development -> main` release. "Update main" alone
   is not confirmation.

## Next Steps (ordered)

1. **Land `BUG-075`.** Run its verification on the dirty tree
   (`cargo check` on the four crates, `cargo test -p opencode-provider --lib`,
   `cargo test -p opencode-session --lib`), get owner sign-off on the invariant change,
   then commit it and the card on `development`. Only `BUG-075`-related files.
2. **Release.** Bump to the next version (`0.1.4`), open `development -> main` with green
   required checks and an approving review, merge on explicit owner confirmation. Confirm
   the `v<version>` manifest and assets.
3. **Deploy.** `standards-deploy-ort --rev <sha> --dry-run`, then for real, on
   `matrillobase1`. Canary first, then serial fan-out; keep the per-machine TSV report.
4. **Verify.** Per machine: `opencode-modded-rust version` and
   `opencode-modded-rust debug scopemux-queries`; `.rev` equals the manifest commit.
5. **Close the fleet gap.** `moldavite` and `xp-farmer` were Tailscale-offline for `v0.1.3`;
   bring them online and deploy the pinned revision. `webappssub2` is `disabled`/skipped.
6. **Record** results on `BUG-075`, `INFRA-012`, `INFRA-014`, and the release card. The
   automatic release→deploy trigger for `INFRA-012` remains owner setup.

## Procedure (real)

### 0. Confirm state

```bash
git fetch origin
git status --porcelain
git log --oneline -1 origin/development
python3 ~/standards/opencode/scripts/agent_roles.py list
```

Do not proceed if `origin/development` is not the intended tip, or if the worktree contains
someone else's in-flight work you would accidentally include.

### 1. Release to `main` (only if the release has not already happened)

Releases move `development -> main` via a release PR with green required checks
(fmt, check, clippy, test, native, smoke) and an approving review. Never push to `main`.
The release workflow (`.github/workflows/release.yml`, `INFRA-011`) builds and publishes
the immutable `v<version>` release.

Do **not** run `cargo build --release` as a deployment step. Local release packaging (only
for the manual interim where the workflow is unavailable) is:

```bash
scripts/package-native-artifacts.sh
scripts/publish-cli-artifact.sh
```

and you must verify the uploaded assets and record the commit SHA, scopemux pin, and
query-set key on the release card.

### 2. Verify the release manifest

```bash
sha256sum -c <asset>.sha256
cat <asset>.rev     # must equal the manifest commit
```

### 3. Deploy the pinned revision from the control host

Dry run first, then for real (canary first, then serial fan-out):

```bash
~/standards/utilities/standards-deploy-ort --rev <release_commit_sha> --dry-run
~/standards/utilities/standards-deploy-ort --rev <release_commit_sha>
```

The driver:

- reads `standards-machines`, filters with `standards-machine-status` (`active` only),
- reports skipped `pending`/`disabled`/`retired` machines with their state,
- installs into `~/.local/share/ort/bin/<rev>/` and updates `current`,
- verifies the canary before fan-out, retries transient SSH failures once,
- rolls a failed machine back to its previous revision and fails the deploy,
- uses `flock` so overlapping releases cannot run,
- writes a TSV per-machine report (`name`, `previous_rev`, `new_rev`, `status`,
  `duration`, `error`) — retain it as the deploy artifact.

### 4. Post-deploy verification (`INFRA-014`)

For each deployed machine:

```bash
opencode-modded-rust version
opencode-modded-rust debug scopemux-queries    # resolves a dir containing rust/docstrings.scm
```

Confirm the installed revision equals the manifest commit (`.rev`). Do not claim success
from a successful install alone; the per-machine report plus these checks are the evidence.

### 5. Record results

Update `INFRA-012`/`INFRA-014` (and `FEAT-077` if it is the released item) with: release
tag, commit, per-machine result, skipped machines and reasons, and any remaining offline
machines.

## Rollback

Rollback pins a **revision SHA**, not a semver string (the draft's `v1.2.3` does not exist;
current is `v0.1.3`).

```bash
~/standards/utilities/ort-install --rev <previous_commit_sha> --channel stable
opencode-modded-rust version
opencode-modded-rust debug scopemux-queries
```

Then mark the release not fully deployed, name the failing machines on the release and the
owning board card, and retain the previous revision/assets until the release is confirmed.

## Human Gates

- Release PR merge: explicit owner confirmation for that release (`INFRA-010`).
- Control host / fleet credentials: owner decision (`INFRA-012`).
- A canary failure or any active-machine failure is a stop-and-report, not a retry-forever.

## Corrections vs. the Drafted Handoff

| Draft claim | Reality |
|---|---|
| `git merge feat/show-selected-model-in-models-menu` | Branch gone; merged as PR #191 (`1852d38`) and released in `v0.1.3`. |
| `cargo build --release` | Not the release path; CI release or `package-native-artifacts.sh` + `publish-cli-artifact.sh`. |
| `./scripts/deploy.sh production` | No such script; use `standards-deploy-ort`. |
| `ssh user@prod-server-1 "journalctl -u opencode.service ..."` | No prod servers or `opencode.service`; fleet = registered machines. |
| `./opencode-cli models list` | Binary is `opencode-modded-rust`; no such `opencode-cli models list` verification. |
| Rollback `./scripts/rollback.sh v1.2.3` | No such script/tag; `ort-install --rev <sha> --channel stable`. |
| Docs at `/docs/deployment.md` | `docs/release-deployment.md`. |
| Prereq: worktree clean | Currently dirty with another session's BUG-075 work — preserve, do not release. |

## Out Of Scope

The "next steps" trading schedule pasted alongside this handoff (NZDJPY/EURCAD/EURGBP/
GBPCAD, EUR CPI, BoE, NFP, PWRH-002-RSCH, TJB-047) belongs to the `aa-studies` /
`trade-journal` work, not this repository. Do not act on it here.

## Related Items

- `INFRA-012` — automatic fleet deploy (driver landed; trigger pending).
- `INFRA-014` — verification and rollback.
- `INFRA-011` — versioned release + manifest.
- `INFRA-013` — stable vs preview channels.
- `handoffs/H-019-main-release-fleet-deployment-program-handoff.md` — program handoff.

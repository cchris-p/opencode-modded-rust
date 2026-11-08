---
id: "H-019"
title: "INFRA-009/010/011/012/013/014/015 Main-Release Fleet Deployment Program — Program Handoff"
status: "open"
created: "2026-11-05"
updated: "2026-11-05"
owner: ""
target: "opencode-modded-rust; standards (fleet deploy tooling and channel docs)"
blocked_reason: ""
needs_human: ""
items: ["INFRA-015", "INFRA-009", "INFRA-011", "INFRA-010", "INFRA-013", "INFRA-014", "INFRA-012"]
---

# INFRA-009/010/011/012/013/014/015 Main-Release Fleet Deployment Program — Program Handoff

## Current State

The native ScopeMux bundle and the `ort` CLI are built and published by
`.github/workflows/native-artifacts.yml`, and `INFRA-008` makes the shipped binary
self-contained (embedded Tree-sitter query set). The distribution path works, but the
release and deployment path is still manual and the safety gates are missing:

- There is **no PR/push test workflow**; only the native-artifacts workflow exists.
- `main` protection is unset/unknown. The current `gh` token cannot read branch protection
  (`403 Resource not accessible by personal access token`).
- A `main` merge does not produce a versioned release; the workflow does not trigger on
  `main` at all.
- The rolling `native-artifacts` tag (refreshed on `development` pushes) is the default
  `ort-install` channel, so clients can auto-update to development builds.
- There is no fleet deploy fan-out, no per-machine post-deploy verification, and no
  automatic rollback.

**Locked interpretation:** "every build deploys all machines" means **every `main` release
deploys all active registered machines**. `development`/preview builds refresh the preview
channel only and never auto-deploy to the fleet. `main` must be green and protected before a
release can deploy.

This program is separate from `H-018` (remaining ScopeMux capabilities) and `INFRA-008`
(portable query bundle). It may land while `H-018` is paused.

## The 7 Steps, In Order

| # | Board ID | Work | Repo(s) | Kind |
|---|---|---|---|---|
| 1 | `INFRA-015` | Invariant + release/deploy runbook + channel docs | product + standards | docs PR / direct |
| 2 | `INFRA-009` | Required CI test suite (fmt/check/clippy/test/native/smoke) | product | PR |
| 3 | `INFRA-011` | Automated versioned release on `main` + manifest | product | PR |
| 4 | `INFRA-010` | Protect `main`, require checks, release-only merges | GitHub settings + docs | admin + docs PR |
| 5 | `INFRA-013` | Stable default ORT channel, preview opt-in | standards + product docs | PR |
| 6 | `INFRA-014` | Post-deploy verification, canary, rollback | standards (+ product script) | PR |
| 7 | `INFRA-012` | Automatic fleet deploy of every main release | standards + product workflow | PR |

Dependency order (strict): `INFRA-015` -> `INFRA-009` -> `INFRA-011` -> `INFRA-010` ->
`INFRA-013` -> `INFRA-014` -> `INFRA-012`.

- `INFRA-010` needs the check names produced by `INFRA-009`/`INFRA-011`.
- `INFRA-014` provides the verification the deploy driver (`INFRA-012`) consumes.
- `INFRA-013` must exist before the deploy driver can pin stable revisions safely.
- `INFRA-015` is written first as the target policy; refine it at the end to match what
  actually landed.

## Operating Model — One PR at a Time, Human-Gated

Repeat this cycle for each step. Do not start step N+1 until step N's PR(s) are merged.

1. **Gate check:** confirm the previous PR is merged, `development` is current, and the
   previous step's verification is recorded on its card.
2. **Branch:** create the step branch from `origin/development` in a dedicated worktree for
   product PRs. Standards changes are made in the `~/standards` checkout and follow that
   repo's workflow.
3. **Board:** move the card `todo` -> `doing` (direct on `development` for board/docs work).
4. **Implement + verify:** implement, run the per-step verification, and record exact
   commands/results on the card.
5. **Open PR(s):** target `development`; never `main`. One PR per repo per step when a step
   spans both. Keep the card out of `done`.
6. **STOP (human):** ask the owner to review and explicitly approve each merge. Never merge
   on tests alone.
7. **After merge:** move the card to `qa`, pull `development`, run post-merge QA, then
   promote to `done` only with recorded evidence.
8. **Admin steps:** `INFRA-010` and the control-host/credential decisions require the owner.
   Flag `needs_human`, record the exact requested action, and pause.

Board/docs work commits directly on `development`. Do not push `main`.

## Shared Decisions — Lock Early

- **Deployment trigger:** only a successful stable release from `main`. Development/preview
  builds never deploy.
- **Release completeness:** a release is not "done" until every active registered machine
  verifies the new revision; any failure blocks completion and rolls back.
- **Inventory:** `standards/standards-machines` is the source of truth; lifecycle is read
  from OpenBao via `standards-machine-status`. Active machines deploy; `pending`,
  `disabled`, and `retired` are reported and skipped, never silently dropped.
- **Channels:** `stable` (latest `main` release, default) and `preview` (rolling
  `native-artifacts`, opt-in). No cross-channel auto-update.
- **Credentials:** use the registered-machine SSH mesh and OpenBao. Never expose mesh keys
  or OpenBao tokens to GitHub-hosted runners. The deploy control plane must be a
  mesh-capable host (recommended: `matrillobase1`) or a self-hosted runner with the same
  least-privilege access.
- **No disruptive host actions:** deployment installs a binary/cache entry; it must not
  reboot, restart, or disrupt other services on a machine (`invariants/disruptive-host-actions.md`).
- **Fail closed:** missing checksum, missing asset, unknown revision, or unverifiable
  machine means the release/deploy fails, not a warning.
- **`INFRA-008` contract:** the shipped binary carries its own query set; verification uses
  `debug scopemux-queries`, not a checkout or native cache.
- **Keep it boring:** reuse the existing `ort-install`, `standards-sync`, and
  `native-artifacts` patterns instead of building a second distribution system.

## Per-Item Runbooks

### Step 1 — `INFRA-015` invariant and runbook

New `invariants/release-deployment.md`; update `AGENTS.md`, `README.md`,
`docs/release-deployment.md` (new), and the `standards` docs/AGENTS where fleet/channel
behavior is described.

Key content:
- `main` is protected and release-only.
- Every `main` merge produces an immutable versioned release.
- Every successful release deploys all active registered machines.
- A release is complete only when all active machines verify; failure rolls back.
- Preview builds never deploy.
- Inactive machines are skipped explicitly.
- Credentials stay in the mesh/OpenBao, never GitHub-hosted runners.

Verify: every rule from the invariant maps to one of `INFRA-009`..`INFRA-014`.

### Step 2 — `INFRA-009` required CI

Add `.github/workflows/ci.yml`:
- `fmt`: `cargo fmt --all --check`.
- `check`: `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo check --workspace`.
- `clippy`: `cargo clippy --workspace --all-targets`.
- `test`: `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test --workspace`.
- `native`: prebuilt bundle, `cargo test -p opencode-scopemux --features native`, and the
  INFRA-008 clean-host `debug scopemux-queries` smoke.
- Linux `ubuntu-22.04` + macOS `macos-14`; caching; timeouts; superseded-run cancellation.

Touchpoints: `.github/workflows/ci.yml`, `README.md`, possibly test quarantine markers.

Quarantine policy for the two documented pre-existing native failures
(`native_provider_reports_plan_reconciliation_evidence` SIGSEGV,
`native_delta_slice_returns_projected_entries`): fix them, or `#[ignore = "SCOPE-003: <reason>"]`
with a tracked card. Do not silently exclude them.

Verify with throwaway branches that break fmt, a test, and the query smoke independently.

### Step 3 — `INFRA-011` automated versioned release

Extend `native-artifacts.yml` or add `.github/workflows/release.yml` triggered by `main`:
- build native bundle + CLI on Linux and macOS;
- publish an immutable versioned tag (`v<version>` preferred) with tarballs, `.sha256`,
  `.rev`, a `manifest.json` (tag, commit, scopemux pin, query key, assets, checksums), and
  release notes;
- keep the rolling `native-artifacts` tag as preview;
- fail on any missing platform asset/checksum;
- run the clean-host smoke on the built binary.

Touchpoints: `.github/workflows/native-artifacts.yml` / new release workflow,
`scripts/package-native-artifacts.sh`, `scripts/publish-cli-artifact.sh`,
`scripts/ensure-native-artifacts.sh`, `README.md`.

Verify with publishing disabled/dry-run; extract the asset on a clean host and run
`version` + `debug scopemux-queries`.

### Step 4 — `INFRA-010` protect main

Configure a GitHub ruleset for `main`:
- no direct push/delete/force-push;
- PR required, head `development`;
- one approving review, dismiss stale approvals;
- required checks from `INFRA-009` and `INFRA-011`;
- conversation resolution and linear history/merge queue;
- include administrators or record the explicit exception;
- optionally restrict merge actors.

Record the ruleset in `docs/release-deployment.md` with a `gh api` example and the UI
fallback. Add a policy check rejecting a `main` PR whose head is not `development` if the
ruleset cannot express it.

Human gate: applying this requires repository admin. The current token returns `403` on
protection reads. Do not weaken protection to work around missing permissions.

Verify: direct push rejected; failing required check blocks merge; non-`development` head
blocked.

### Step 5 — `INFRA-013` stable channel

Standards repo:
- `~/standards/utilities/ort-install`: default `stable`; `--channel stable|preview`;
  preserve `--tag`/`--rev`; resolve the latest stable release and verify checksum.
- `~/standards/opencode-config`: `OPENCODE_RUST_CHANNEL` (default `stable`) used by
  auto-install and `_opencode_rust_maybe_auto_update`; preview opt-in; `ort-doctor` shows
  channel/rev.
- `~/standards/docs/registered-machines.md` / AGENTS: channel rules.
Product: `README.md` channel documentation.

Verify on a scratch `ORT_BIN_ROOT`: default resolves stable; `--channel preview` follows the
rolling tag; a rolling-tag change does not move a stable-configured machine.

### Step 6 — `INFRA-014` verification and rollback

Standards repo (preferred) or product `scripts/`:
- per-machine check: installed rev == manifest rev; `version` runs;
  `debug scopemux-queries` resolves and contains e.g. `rust/docstrings.scm`;
- optional bounded read-only provider smoke when the local Qwen profile is active;
- machine-readable report; canary-first; aggregate summary;
- rollback to the previous revision on failure; keep prior assets until confirmed;
- dry-run/report-only mode.

Verify: healthy machine passes; mismatched revision fails; simulated canary failure rolls
back.

### Step 7 — `INFRA-012` automatic fleet deploy

Decision first (record on the card): deploy control plane.
- Recommended: control host `matrillobase1` with a self-hosted GitHub Actions runner (or a
  release-triggered systemd job) because it already has the SSH mesh and OpenBao access.
- Alternative: a `standards` deploy driver invoked by the release workflow.
- Do not use GitHub-hosted runners with mesh keys.

Then:
- deploy driver reads the `INFRA-011` manifest;
- enumerates `standards-machines`, filters with `standards-machine-status` (active only,
  explicit skip report for others);
- canary deploy + verify, then fan-out;
- install exact revision (`ort-install --rev <sha>`);
- verify every machine (`INFRA-014`);
- fail the release/deploy on any active-machine failure; retry transient SSH once;
- serialize deploys, refuse overlaps;
- rollback on failure;
- never trigger from preview/development builds.

Verify: controlled test release reaches canary then fleet; one broken machine fails the run
and keeps the previous revision; a development build does not trigger deploy.

## Verification Standards (every step)

- Paste exact commands and results into the card and PR body.
- `cargo fmt --all --check` and the new CI suite must pass.
- Native behavior verified with the prebuilt bundle and the clean-host query smoke.
- Deployment changes are verified on a canary and, where practical, a scoped subset before
  the whole fleet.
- Never report success for a machine that did not verify.
- No secrets, keys, tokens, or machine-private data in logs, PRs, or committed files.

## PR Description Template

```markdown
## Summary
<what and why, which board item and handoff step>

## Acceptance Criteria
- [x] <AC from the card>

## Code / Config Touchpoints
- <file:line or file list, per repo>

## Verification
- <exact commands run + results>

## Human/Admin Gates
- <ruleset apply, control-host approval, credential scope, if any>

## Known Limitations
- ...

## Board Card
<INFRA-XXX>; step N of H-019
```

## Human/Admin Checklist

- [ ] Approve the `main` ruleset (admin token or UI) and confirm the required check names.
- [ ] Approve the deploy control plane (host/runner) and its least-privilege credentials.
- [ ] Confirm no mesh/OpenBao credentials reach GitHub-hosted runners.
- [ ] Approve enabling automatic fleet deployment after a successful canary/dry run.
- [ ] Confirm rollback behavior and the previous-release retention window.
- [ ] Confirm deployment does not reboot or disrupt services on any machine.
- [ ] Confirm the stable/preview channel defaults are what the owner wants.

## Out of Scope / Future Work

- Release signing/notarization beyond the existing macOS quarantine handling.
- Monitoring/alerting integration for deploys (candidate for Zabbix/Healthchecks).
- Deploying to unregistered/supplementary hosts, NAS appliances, or Windows-native hosts.
- Changing the ScopeMux activation policy or the `H-018` capability work.
- Automatically fixing a red `main`; the invariant is to block, not to auto-repair.

## Session Kickoff Prompt

Send this to a new session started in `~/apps/opencode-modded-rust`:

```text
Execute handoff H-019 (handoffs/H-019-main-release-fleet-deployment-program-handoff.md)
using the handoff-to-pr skill. Implement the 7 steps in order, one at a time, and update
the cards and this handoff as you go.

Scope: INFRA-015, INFRA-009, INFRA-011, INFRA-010, INFRA-013, INFRA-014, INFRA-012.
Product PRs target development; never push main. Standards changes are made in the
~/standards checkout and follow that repo's workflow.

Locked policy: only main releases deploy to the fleet; development/preview builds never
deploy. Every main release must deploy all active registered machines and verify them.
Main must be protected and green.

Stop for human approval before: applying the main ruleset, choosing the deploy control
plane/credentials, and enabling automatic fleet deployment. Do not merge any PR without
the owner's explicit approval.
```

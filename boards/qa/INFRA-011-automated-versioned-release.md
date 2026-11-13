---
id: "INFRA-011"
title: "Automate a complete versioned release on every main merge"
priority: "P1"
type: "chore"
area: "INFRA"
spec: "README.md"
status: "qa"
created: "2026-11-05"
updated: "2026-11-08"
---

# Automate A Complete Versioned Release On Every Main Merge

See invariant: `invariants/release-deployment.md`.

## Summary

Make a `main` merge produce a complete, immutable release with no manual build or upload
step: native provider bundle, CLI binaries for Linux and macOS, checksums, revision
provenance, release notes, and a stable release tag. The rolling `native-artifacts` tag is
demoted to a development preview channel.

This is "the new bundle is automated with every build": the native bundle and CLI are built
and published by CI, not by a person running `scripts/package-native-artifacts.sh` or
`scripts/publish-cli-artifact.sh`.

## Current State

- `.github/workflows/native-artifacts.yml` builds the native bundle and CLI.
- It triggers on `development` pushes and tags, and publishes to the rolling
  `native-artifacts` release. It does not trigger on `main` pushes or produce a versioned
  release.
- `native-artifacts` is the default `ort-install` tag, so development builds currently
  reach clients (see `INFRA-013`).
- `scripts/package-native-artifacts.sh` and `scripts/publish-cli-artifact.sh` exist and must
  remain usable by a builder, but the release path must not depend on manual invocation.
- `INFRA-008` embeds the query set in the binary, so the bundle and CLI are self-contained.

## Scope

- Add a release workflow triggered by a push to `main` (and/or a completed release PR):
  - build the native bundle + release CLI on `ubuntu-22.04` and `macos-14`;
  - publish an immutable release tag derived from the crate version and/or commit, for
    example `v<version>` (preferred) or `release-<shortsha>`;
  - attach bundle tarballs, CLI tarballs, `.sha256`, and `.rev` sidecars;
  - include the exact commit SHA, scopemux pin, and query-set key in provenance;
  - generate release notes from merged PRs/board items;
  - mark it as the latest stable release;
  - fail the release if any platform asset or checksum is missing.
- Keep the development rolling tag (`native-artifacts`) for preview builds, but label it as
  preview and do not let it be the fleet default.
- Reuse the existing packaging scripts from the workflow; remove duplicated build logic
  where practical.
- Keep the INFRA-008 clean-host query smoke and add a release-asset integrity check
  (checksum + tarball contents + `debug scopemux-queries`).
- Publish a machine-readable release manifest (tag, commit, scopemux pin, query key, asset
  names, checksums) that `INFRA-012` can consume.

## Acceptance Criteria

- Merging a release PR to `main` produces a complete versioned release with no manual
  command.
- The release contains Linux and macOS bundle + CLI assets, checksums, `.rev`, and manifest.
- A missing/failing platform asset fails the workflow and blocks deployment.
- The rolling development tag is clearly preview-only.
- The release commit matches `main` and the manifest records the scopemux pin/query key.
- `scripts/package-native-artifacts.sh` and `scripts/publish-cli-artifact.sh` still work for
  local publishing.

## Verification

- Dry-run the workflow on a branch/tag in a test repository or with publishing disabled;
  inspect the asset list and manifest.
- Verify the checksum and extract the CLI asset on a clean host; run
  `opencode-modded-rust version` and `debug scopemux-queries`.
- Confirm `main` remains the only trigger for a stable release.

## Non-Goals

- Deploying the release to machines (`INFRA-012`).
- Changing the client auto-update channel (`INFRA-013`).
- Signing/notarization beyond the existing macOS quarantine handling (can be a follow-up).

## Related Items

- `INFRA-006`, `INFRA-007`, `INFRA-008`.
- `INFRA-009` required CI.
- `INFRA-010` protect main.
- `INFRA-012` consumes the release manifest.
- `INFRA-013` stable channel.

## Implementation (2026-11-08)

Added `.github/workflows/release.yml`, merged as PR #173 (`c601c1f`).

- Trigger: push to `main` (+ manual `workflow_dispatch` for staging). Publishing is
  gated on `github.ref == 'refs/heads/main'`, so a dispatch on a feature/development
  ref only builds and uploads the manifest artifact.
- Builds the native bundle and CLI on `ubuntu-22.04` and `macos-14` using
  `scripts/package-native-artifacts.sh`; runs the INFRA-008 clean-host query smoke and a
  clean-HOME `version` smoke.
- Verifies every tarball against `.sha256`, requires both CLI assets and both native
  bundles plus `.rev` sidecars, and writes `release-manifest.json` (tag, version, commit,
  scopemux pin, query key, per-asset sha256/platform).
- Publishes an immutable `v<version>` GitHub release with generated notes, marked latest.
  Fails if the tag already exists; cut a new release by bumping `[workspace.package]
  version` in the release PR.
- Rolling `native-artifacts` tag remains preview-only and is untouched.

Remaining QA gate before `done`: the first real `main` release run (the workflow cannot run
until it is on `main`/the default branch), and a dry-run dispatch
(`gh workflow run release.yml -f dry_run=true`) to confirm staging + manifest without
publishing. Deployment is `INFRA-012`.

## First Release: v0.1.0 (2026-11-08)

- `development -> main` release PR #175 merged (`eeb608f`).
- The `Release` workflow build matrix cancelled `build (macos-14)` at ~1s (no macOS
  runner/minutes in this repo), so `publish` was skipped. Ubuntu built successfully and its
  `release-ubuntu-22.04` artifact was downloaded.
- To publish a complete release, the macOS CLI asset was built on `j8rsxy` (Darwin arm64,
  Rust 1.98, prebuilt `macos-aarch64-2a642c10534f` bundle), the clean-host query smoke passed
  (`73aa2ff86776cc85`), and the darwin CLI + native bundle were packaged. Both platform
  assets and the manifest were attached and `v0.1.0` was created manually with the same
  schema (`latest`, non-prerelease).
- This is a **one-time recovery**: the automation is correct, but macOS asset production
  needs a working macOS runner (enable Actions macOS minutes, or a self-hosted macOS runner
  on `j8rsxy`). Until then every release must be completed the same way.

Release: https://github.com/cchris-p/opencode-modded-rust/releases/tag/v0.1.0

## Update - 2026-11-12

- The release matrix now uses `macos-15` (GitHub retired `macos-14` on 2026-11-02).
- The rolling `native-artifacts` macOS leg is `continue-on-error`, so a hosted macOS runner outage
  cannot block Linux artifact publishing.
- The immutable `release.yml` still requires both Linux and macOS assets, per
  `invariants/release-deployment.md`. A hosted macOS runner outage can still block a `main`
  release by design; resolving that needs reliable macOS runners or an explicit invariant change.

## Update - 2026-11-12 (v0.1.3)

- Automated release attempted twice (`43249125655`, `43252958582`): Linux `build` succeeded, but
  `build (macos-15)` was cancelled after ~15 minutes queued with an empty `runner_name` and zero
  steps; `publish` was skipped. The rolling preview build had queued ~20 minutes and succeeded, so
  this is hosted macOS capacity, not a bad label.
- Recovered manually: Linux asset from Release run `43252958582` (`e2334a3`); macOS CLI/native from
  the tree-identical preview run `43247378454` (PR merge ref `4ae1556`). All three commits share
  tree `6d7d516d4ec17bbaf3ac314c406672151a34d09f`. Verified SHA256s, wrote the manifest
  (`commit=e2334a3`, `version=0.1.3`, pin/query key), tagged and published `v0.1.3` with 12 assets.
- Recommendation: add an automatic retry/fallback for macOS release jobs (or a self-hosted macOS
  runner). A 15-minute queue cancellation currently requires manual recovery.

Release: https://github.com/cchris-p/opencode-modded-rust/releases/tag/v0.1.3

## Update - 2026-11-13 (v0.1.4)

- Release PR #194 merged; `main` at `cc9befa`, version `0.1.4`.
- `Release` run `43443998888`: Linux `build` succeeded, but `build (macos-15)` again
  queued ~15 minutes with `runner_id: 0` and zero steps before cancellation;
  `publish` was skipped. Hosted macOS capacity/minutes remain the blocker.
- Recovered manually: Linux assets from run `43443998888`; macOS CLI built on
  `j8rsxy` from `cc9befa` with the prebuilt `macos-aarch64-2a642c10534f` bundle
  (query smoke key `73aa2ff86776cc85`, matching the Linux key). Repackaged with
  `.rev=cc9befa`, generated `release-manifest.json`, and published `v0.1.4` with
  12 assets.
- Still recommended: automatic macOS retry/fallback or a self-hosted macOS
  runner; hosted macOS availability caused the same manual recovery for v0.1.0,
  v0.1.3, and v0.1.4.

Release: https://github.com/cchris-p/opencode-modded-rust/releases/tag/v0.1.4

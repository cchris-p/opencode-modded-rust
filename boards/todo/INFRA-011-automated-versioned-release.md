---
id: "INFRA-011"
title: "Automate a complete versioned release on every main merge"
priority: "P1"
type: "chore"
area: "INFRA"
spec: "README.md"
status: "todo"
created: "2026-11-05"
updated: "2026-11-05"
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

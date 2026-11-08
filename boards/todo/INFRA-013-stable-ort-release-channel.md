---
id: "INFRA-013"
title: "Make the stable main release the default ORT channel"
priority: "P1"
type: "chore"
area: "INFRA"
spec: "README.md"
status: "todo"
created: "2026-11-05"
updated: "2026-11-05"
---

# Make The Stable Main Release The Default ORT Channel

See invariant: `invariants/release-deployment.md`.

## Summary

`ort-install` and `opencode-config` currently default to the rolling `native-artifacts`
release, which is refreshed on every `development` push. That means fleet machines can
auto-update to unreleased development builds. Switch the default channel to the latest
stable `main` release, and keep the rolling tag as an explicit preview channel.

## Current State

- `~/standards/utilities/ort-install`: `TAG` defaults to `native-artifacts`; install layout
  is `~/.local/share/ort/bin/<rev>/opencode-modded-rust`, `current` points at the newest rev.
- `~/standards/opencode-config`:
  - auto-installs when no binary is present;
  - `_opencode_rust_maybe_auto_update` refreshes from `ort-install --if-installed` every
    `OPENCODE_RUST_UPDATE_INTERVAL` (default 900s);
  - `OPENCODE_RUST_AUTO_UPDATE`, `OPENCODE_RUST_UPDATE_INTERVAL`,
    `OPENCODE_RUST_USE_CHECKOUT`, and `OPENCODE_RUST_BIN` are the existing overrides.
- The rolling tag is the only published channel today.

## Scope

- Define channels: `stable` (latest `main` release) and `preview` (rolling development
  `native-artifacts`).
- Change `ort-install` to default to `stable` and accept `--channel stable|preview`
  (keeping `--tag`/`--rev` for exact pins). A stable install must resolve the latest stable
  release tag and install its `.rev`/checksum.
- Add `OPENCODE_RUST_CHANNEL` (default `stable`) to `opencode-config`; all auto-install and
  auto-update paths use it. Preview must be opt-in per machine.
- Keep exact-revision installs (`--rev`) for the deploy driver.
- Make `ort-doctor` show the active channel, installed revision, and whether the revision is
  stable or preview.
- Prevent accidental cross-channel auto-updates (stable does not follow the rolling tag, and
  preview does not silently downgrade to stable unless asked).
- Update `~/standards` docs/AGENTS and this repo's README with the channel model.
- Consider making preview auto-update opt-in even when the rolling tag moves.

## Acceptance Criteria

- A binary-only host with no configuration installs the latest stable release.
- `OPENCODE_RUST_CHANNEL=preview` (or `ort-install --channel preview`) installs the rolling
  development build.
- `ort` auto-update follows the configured channel only.
- `ort-doctor`/`version` makes the channel and revision visible.
- Development builds cannot reach a default-configured fleet machine through auto-update.
- Exact `--rev` installs remain available to `INFRA-012`.

## Verification

- On a scratch `ORT_BIN_ROOT`, run `ort-install` and confirm it resolves a stable tag, not
  `native-artifacts`.
- Run `ort-install --channel preview` and confirm the rolling tag is used.
- Simulate a rolling-tag update and confirm a stable-configured machine does not move.
- Confirm checksum verification still runs for both channels.

## Non-Goals

- Publishing the stable release (`INFRA-011`).
- Fleet fan-out (`INFRA-012`).
- Changing the auto-update interval or the artifact cache layout.

## Related Items

- `INFRA-007` `ort-install` and artifact-first `ort`.
- `INFRA-011` versioned stable release.
- `INFRA-012` deploy driver (uses exact revisions).
- `INFRA-015` invariant and runbook.

---
id: "INFRA-007"
title: "Consume the published ort CLI binary instead of per-machine repo clones"
priority: "P1"
type: "chore"
area: "INFRA"
spec: ""
status: "qa"
created: "2026-10-23"
updated: "2026-10-23"
---

# Consume The Published ort CLI Binary Instead Of Per-Machine Repo Clones

## Summary

`INFRA-006` publishes the native provider bundle **and** a release CLI tarball
(`opencode-modded-rust-<os>-<arch>.tar.gz` + `.sha256`) to the rolling
`native-artifacts` GitHub release, but nothing consumes the CLI tarball: every
machine still clones the full `opencode-modded-rust` repo (~12 MB source, but
~2.6-3 GB once `target/` exists) and compiles it locally just to run `ort`.

Add an artifact-only consumer path: one builder publishes a release binary, and
all other machines download, checksum, and run that binary without a repo
checkout or Rust toolchain.

## Context

- Published assets already exist (2026-10-14): `opencode-modded-rust-linux-x86_64`
  16 MB compressed / 54 MB binary, `opencode-modded-rust-darwin-aarch64` 15 MB,
  versus a 417 MB debug binary and a 2.6 GB `target/` per machine.
- The release binary embeds its command templates (`include_str!`); config,
  skills, and AGENTS still come from `~/.config/opencode` and `standards`, so a
  binary-only host does not need the product checkout at runtime.
- Fleet glibc: Ubuntu 22.04 = 2.35 (matrillobase1, matrillo, matrillosub1/2),
  Debian 12 = 2.36 (webapps, webappssub1), Fedora 41 = 2.40 (jasper), macOS
  arm64 (j8rsxy). The current linux asset is built on `ubuntu-24.04` and requires
  `GLIBC_2.39`, which is not available on the 22.04 hosts.
- `scripts/ensure-native-artifacts.sh` already implements the download pattern
  to reuse: platform detection, authenticated `gh`/`curl`, checksum verify,
  mirror override, and a cache directory.
- The reported symptom (`sc` -> `cd ~/apps/opencode-modded-rust: No such file`)
  came from binary-only hosts that have no checkout; `sc` needs to become
  helpful instead of failing with a raw `cd` error.

## Scope

- Product (`opencode-modded-rust`):
  - Publish the CLI asset on `development` pushes to the rolling
    `native-artifacts` release, with a `.rev` sidecar carrying the commit SHA.
  - Build the Linux asset on `ubuntu-22.04` (oldest fleet glibc) so it runs on
    2.35+, or switch to a static/musl target; keep the `macos-14` asset.
  - Add `scripts/publish-cli-artifact.sh` so a single dev machine can build and
    upload the asset without waiting for CI.
- Standards (`standards`):
  - Add `utilities/ort-install`: standalone (no product checkout required)
    download + checksum + atomic install into
    `~/.local/share/ort/bin/<rev>/opencode-modded-rust`, with `--check`,
    `--force`, `--tag`, `--rev`, and mirror-URL override.
  - Make `ort` artifact-first in `opencode-config`: prefer the installed cache,
    fall back to a local `target/debug|release` build for developers; add
    `ort-install`/`opencode-rust-install` and an auto-install path when no
    binary is present.
  - Make `sc`/`ocm`/`scopemux-code` explain the binary-only state instead of a
    raw `cd` failure.
- Fleet cleanup: after verification, remove `~/apps/opencode-modded-rust` and
  the superseded `~/standards/vendor/opencode-modded-rust` from non-builder
  hosts; keep one checkout per platform for publishing.

## Non-goals

- Removing the local source-build path for contributors (`ort-build` stays).
- Reworking the native provider bundle (already handled by `INFRA-006`).
- macOS cross-compilation from Linux; the mac asset still comes from macOS/CI.

## Done When

- `ort-install` on a host with no product checkout downloads, verifies, and runs
  `opencode-modded-rust version`.
- `ort` resolves the installed artifact and runs without a repo checkout;
  `OPENCODE_RUST_USE_CHECKOUT=1` (or `OPENCODE_RUST_BIN`) opts a builder into a
  local `target/debug|release` binary.
- The rolling `native-artifacts` linux asset is glibc-2.35 compatible and is
  republished on `development` pushes (with the commit SHA recorded).
- Non-builder hosts no longer carry a product checkout or `target/`, reclaiming
  ~3 GB each.

## Related Items

- `INFRA-006` build ort/scopemux centrally and consume published binaries
  (publish half, in `qa`).
- `INFRA-004` align checkout/setup paths and trim disk footprint.
- `STD-083` decouple managed vendor repos from the standards checkout
  (`standards` board).

## Related Docs

- `.github/workflows/native-artifacts.yml`
- `scripts/ensure-native-artifacts.sh`
- `standards/opencode-config`, `standards/bash-config`

## Implementation - 2026-10-23

See the implementation notes appended once the consumer path lands and is
verified on a non-builder host.

### Implementation - 2026-10-23

Landed the consumer half; `INFRA-006` already published the assets.

Product (`opencode-modded-rust`, uncommitted working tree):

- `.github/workflows/native-artifacts.yml` — Linux CLI asset now builds on
  `ubuntu-22.04` (glibc 2.35); publishes the rolling `native-artifacts` release
  on `development` pushes (paths-filtered), in addition to `v*` tags/dispatch;
  writes `<asset>.tar.gz.rev` with the source commit.
- `scripts/publish-cli-artifact.sh` — one builder can build + package + upload
  without CI (`--dry-run` packages only). Emits `.tar.gz`, `.sha256`,
  `.tar.gz.rev`.
- `README.md` — "CLI binary artifacts" documents publish/consume/overrides.
- `.gitignore` — ignore `dist/`.

Standards (`standards`, uncommitted working tree):

- `utilities/ort-install` — standalone consumer (no product checkout): platform
  detect (`linux|darwin` x `x86_64|aarch64`), `gh`/`curl` download with
  `GH_TOKEN`, checksum verify, `.rev`-keyed install into
  `~/.local/share/ort/bin/<rev>/`, atomic `current` symlink, macOS quarantine
  clear, `--check|--force|--rev|--tag|--dir|--dry-run` and mirror-URL override.
- `opencode-config` — `OPENCODE_RUST_BIN_ROOT`, installed-artifact-first
  resolver with local-checkout fallback, `opencode-rust-install`/`ort-install`,
  one-shot auto-install in `_opencode_rust_exec` on hosts with no checkout,
  `ort-doctor` reports the resolved binary, `OPENCODE_RUST_USE_CHECKOUT=1` and
  `OPENCODE_RUST_BIN` overrides.
- `ort-setup --binary-only` — provision a binary-only host (no clone/build).
- `bash-config` — `sc`/`ocm`/`scopemux-code`/`sc-test` are guarded functions that
  explain the binary-only state instead of failing with a raw `cd` error.

Verification (2026-10-23):

- Rebuilt and published `opencode-modded-rust-linux-x86_64` from
  `matrillobase1` (Ubuntu 22.04, glibc 2.35) at rev `6a8837a`; the previous
  ubuntu-24.04 asset required `GLIBC_2.39` and printed a loader warning on 22.04.
  The new asset runs cleanly and reports `0.1.0`. Removed the orphan
  `opencode-modded-rust-linux-x86_64.rev` asset and uploaded the corrected
  `<asset>.tar.gz.rev` name.
- `utilities/ort-install --force` on `matrillobase1` and `webapps` downloaded,
  checksum-verified, installed under `bin/6a8837a.../`, and ran `version`.
- On `webapps` (a non-builder) with `OPENCODE_RUST_REPO=/nonexistent`, the new
  `opencode-config` auto-installed the artifact and `opencode-rust version`
  succeeded (`0.1.0`) with no checkout.

Remaining (needs explicit commit/push authorization):

- Commit/push the product and standards changes, then `standards-sync` so every
  host has `utilities/ort-install` and the artifact-first launcher.
- After sync, remove `~/apps/opencode-modded-rust` and the superseded
  `~/standards/vendor/opencode-modded-rust` from non-builder hosts (keep one
  checkout per platform for publishing). Do not strip before the config is
  distributed or `ort` breaks on hosts that still expect the checkout.
- Next CI `development` push will attach a `.rev` to the macOS asset as well.
### Fleet rollout and cleanup - 2026-10-23

Committed product `bebe3ab` and standards `53a1486`. CI republished both the
linux and darwin assets at `bebe3ab` with `<asset>.tar.gz.rev`; `ort-install`
keys them under the commit and runs `0.1.0`.

Standards config reached every reachable host (`standards-sync --apply-all` for
jasper/webapps/webappssub1; direct `standards-sync` for matrillo/matrillosub1/
matrillosub2/j8rsxy while `--apply-all` was still archiving mb1's session store).
Each host then installed the artifact and was cleaned to the artifact-only
pattern: removed `~/apps/opencode-modded-rust` and the superseded
`~/standards/vendor/opencode-modded-rust`.

| Host | Reclaimed |
| --- | --- |
| jasper | ~12.0 GB |
| webapps | ~4.2 GB |
| webappssub1 | ~4.1 GB |
| matrillo | ~4.1 GB |
| matrillosub1 | ~8.4 GB |
| matrillosub2 | ~4.6 GB |
| j8rsxy | ~3.6 GB (was at 323 MiB free) |
| matrillobase1 | ~9.2 GB (legacy checkout; builder checkout kept) |

Total: ~50 GB.

Builder decision: **matrillobase1** keeps the single Linux checkout
(`~/apps/opencode-modded-rust` @ `bebe3ab`, 5.3 GB) for `ort-build` and
`publish-cli-artifact.sh`. `webapps` stays an app host and binary-only;
`matrillosub1` is a cold spare (one `vendor-setup opencode-modded-rust` away).
macOS builds stay on the CI `macos-14` job, so `j8rsxy` is binary-only too.

Verified on every cleaned host: `opencode-rust version` reports `0.1.0` from
`~/.local/share/ort/bin/current`, and `sc` prints the binary-only guidance
instead of a raw `cd` error.

Not covered: `moldavite` is offline in Tailscale (last seen 1d) and
`webappssub2` is disabled. `standards-sync --apply-all` was still CPU-bound
archiving mb1's 5.4 GB session store when this pass ended; the git sync for
every reachable machine is complete.

### Session-archive note - 2026-10-23

`standards-sync --apply-all` was stopped during matrillobase1's session-archive
hook: staging grew 33 GB -> 36 GB in 90 s with free space falling to 34 GB, a
disk-exhaustion risk unrelated to this card. The disposable staging directory
was removed (36 GB reclaimed; mb1 free 34 GB -> 74 GB). The git sync for every
reachable machine had already completed. The archive pass can be rerun later
with `STANDARDS_SESSION_ARCHIVE_SYNC=0` for sync-only, or after pruning the
local store.

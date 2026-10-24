# ORT CLI Distribution And Auto-Update

## Purpose

Defines how the Rust product CLI (`opencode-modded-rust`, run as `ort`) is built
once, published as a platform artifact, and kept current on machines that do not
carry a source checkout. Covers the publisher, the consumer, update detection,
refresh triggers, the builder path, and failure behavior.

Related work: `INFRA-006` (central build/publish) and `INFRA-007` (consume the
published CLI instead of per-machine clones).

## Ownership Boundary

- Source of truth: `cchris-p/opencode-modded-rust`, branch `development`.
- Publishing: GitHub Actions (`.github/workflows/native-artifacts.yml`) and the
  manual builder script `scripts/publish-cli-artifact.sh`.
- Consuming: `~/standards/utilities/ort-install` plus the launcher helpers in
  `~/standards/opencode-config` and `~/standards/standards-sync`.
- Builder checkout: `matrillobase1` at `~/apps/opencode-modded-rust`
  (branch `development`). Every other active machine is artifact-only.

## Published Artifacts

Pushes to `development` that touch code or the packaging scripts refresh the
rolling `native-artifacts` GitHub release. Path filters include `Cargo.toml`,
`Cargo.lock`, `crates/**`, `scripts/fetch-scopemux-core.sh`,
`scripts/package-native-artifacts.sh`, `scripts/ensure-native-artifacts.sh`,
`scripts/publish-cli-artifact.sh`, and the workflow file itself.

Each run attaches, per platform:

- `opencode-modded-rust-<os>-<arch>.tar.gz` - the release CLI binary.
- `opencode-modded-rust-<os>-<arch>.tar.gz.sha256` - checksum.
- `opencode-modded-rust-<os>-<arch>.tar.gz.rev` - the source commit SHA.

Platforms: `linux-x86_64` built on `ubuntu-22.04` (glibc 2.35, the oldest fleet
baseline) and `darwin-aarch64` built on `macos-14`. Assets are clobbered in place
under the rolling `native-artifacts` tag; the `.rev` is what makes change
detection cheap.

## Install Layout

`ort-install` installs under
`${ORT_BIN_ROOT:-${XDG_DATA_HOME:-$HOME/.local/share}/ort}`:

```
bin/<rev>/opencode-modded-rust    # immutable per-revision binary
bin/current -> <rev>              # atomically swapped symlink
.last-update-check                # launch-time throttle stamp (epoch seconds)
```

The archive checksum is always verified before install. The `current` symlink is
swapped only after extraction succeeds. On macOS, the quarantine attribute is
cleared best-effort so the downloaded binary runs without a Gatekeeper prompt.

## Consume And Update

`ort-install` with no arguments ensures the latest published revision:

1. Resolve the platform asset name and the auth token (`GH_TOKEN`/`GITHUB_TOKEN`,
   falling back to `~/.config/opencode/.env`).
2. Fetch only the small `<asset>.rev` sidecar through `gh release download`, or
   `curl` with bearer auth as a fallback.
3. Compare it with the revision in the `current` symlink.
   - same: print the installed path and exit without downloading.
   - different: download the tarball, verify `.sha256`, extract to
     `bin/<rev>/`, and swap `current`.
   - unknown (offline, no token, or release unavailable): keep the installed
     revision and warn; the binary keeps working and the next check can update.
4. `--rev`, `--tag`, and `--force` bypass detection and download explicitly.
   `--no-update` keeps the installed revision. `--if-installed` is a no-op when
   nothing is installed yet, which is what the automatic callers use.

The release is private, so the token is required in practice. It is rendered to
`~/.config/opencode/.env` from OpenBao and never committed.

## Refresh Triggers

Three paths converge installed hosts on the latest revision:

1. **Launch-time (background).** `_opencode_rust_maybe_auto_update` runs from
   `_opencode_rust_exec` before the TUI starts, but only when the resolved
   binary is under `OPENCODE_RUST_BIN_ROOT` (never a builder checkout). It is
   throttled by `.last-update-check`, default every 900 seconds, and runs the
   install in a detached background job so startup is not delayed. The current
   launch keeps the already-resolved binary; the next launch uses a revision
   installed by the refresh.
2. **Fleet sync.** `standards-sync` calls the same `--if-installed --quiet`
   refresh after a successful git sync, and only on hosts that already have an
   installed artifact. A changed revision prints
   `standards-sync: ORT artifact updated (<rev>)`. Failures warn and never block
   the sync.
3. **Explicit.** `ort-update` (or `ort-install`) refreshes immediately.

Knobs:

- `OPENCODE_RUST_AUTO_UPDATE=0` disables the launch-time refresh.
- `OPENCODE_RUST_UPDATE_INTERVAL=<seconds>` changes the throttle (default 900).
- `OPENCODE_RUST_BIN=<path>` pins an explicit binary.
- `OPENCODE_RUST_USE_CHECKOUT=1` prefers the builder checkout over the cache.
- `OPENCODE_RUST_AUTO_INSTALL=0` disables the first-run auto-install on a host
  with neither a binary nor a checkout.
- `STANDARDS_SYNC_ORT_UPDATE=0` disables the sync-time refresh.

`ort-doctor` reports `bin_root`, `installed_rev`, `last_update_check`, the
resolved `source` policy, and the binary's `version`.

## Builder Path

`matrillobase1` keeps the only Linux source checkout. Development there is the
normal git flow (`git pull`, feature branch or `development`), then `ort-build`
to build `target/debug/opencode-modded-rust`. Because no artifact cache exists on
the builder, `ort` runs that local build by default. If an artifact is ever
installed there, select the checkout explicitly:

```
OPENCODE_RUST_USE_CHECKOUT=1 ort
```

Publishing is automatic: push to `development` and CI rebuilds both platform
assets and refreshes `.rev`. For an immediate Linux publish without waiting for
CI, run `scripts/publish-cli-artifact.sh` on the builder; it emits the tarball,
checksum, and `.rev` and uploads them to the rolling release.

The macOS asset is always CI-built (`macos-14`); do not cross-compile it from
Linux.

## Rollout And Verification

After a standards config change, converge the fleet with
`~/standards/standards-sync --apply-all` (or each machine's local
`standards-sync`). Then verify per host:

```
cd ~/standards && git rev-parse --short HEAD
readlink ~/.local/share/ort/bin/current
grep -q _opencode_rust_maybe_auto_update opencode-config && echo auto=yes
~/.local/share/ort/bin/current/opencode-modded-rust version
```

An end-to-end update test without touching the live cache uses an isolated root:

```
root=$(mktemp -d)
mkdir -p "$root/bin/oldrev"
cp ~/.local/share/ort/bin/current/opencode-modded-rust "$root/bin/oldrev/"
ln -s oldrev "$root/bin/current"
utilities/ort-install --dir "$root"    # updates oldrev -> published .rev
"$root/bin/current/opencode-modded-rust" version
```

## Failure And Safety Rules

- Never replace a working binary with an unverified download: a missing
  `.sha256` or a checksum mismatch aborts the install.
- Offline or unknown revision state is non-fatal: keep the current binary.
- Auto-refresh is gated on an already-installed artifact; it never installs a
  cache onto a builder checkout or an unrelated app host.
- The launch-time check is bounded and detached; a slow or failed download must
  not delay or fail the TUI launch.
- The installed binary embeds command templates; config, skills, and agent rules
  still come from `~/.config/opencode` and `~/standards`, so artifact-only hosts
  need no product checkout at runtime.

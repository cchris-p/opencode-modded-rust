---
id: "FEAT-080"
title: "Detect a newer published ORT release and prompt the user to install it"
priority: "P2"
type: "feature"
area: "FEAT"
spec: "docs/release-deployment.md"
status: "qa"
created: "2026-11-16"
updated: "2026-11-16"
---

# Detect a newer published ORT release and prompt the user to install it

## Summary

The product should detect that a newer published ORT revision exists for the active channel
and tell the user, with a hotkey/action to install it and then restart. Today the only behavior
is a silent background refresh of the installed artifact; the TUI never surfaces that a new
version exists.

## Why this exists

Users currently cannot tell which revision they are running or that a newer one is published.
The launcher silently swaps the `current` symlink, so machines can drift between revisions with
no visible signal. Combined with `CLI-017` (version visibility), this makes "am I on the latest
build?" unanswerable from inside the product. The user explicitly asked for a detection +
install suggestion with a hotkey.

## Current behavior (verified 2026-11-16)

- `~/standards/opencode-config:886` `_opencode_rust_maybe_auto_update` runs
  `ort-install --if-installed` in the background at most every
  `OPENCODE_RUST_UPDATE_INTERVAL` (default 900s) when the resolved binary is the artifact
  cache. It replaces `~/.local/share/ort/bin/current`; a running process keeps its old binary,
  and the change only takes effect on the next launch. Nothing is shown to the user.
- `~/standards/utilities/ort-install` installs per-rev under
  `~/.local/share/ort/bin/<rev>/`, verifies the `.sha256`, and flips `current` to the newest
  installed rev. The stable channel resolves the latest `v*` release; preview follows the
  rolling `native-artifacts` tag. Latest stable on 2026-11-16 is v0.1.4 (rev `cc9befa1`).
- `~/standards/utilities/ort-verify` reports `rev` and `version` and verifies the query bundle;
  `ort-doctor` reports the channel and source.
- The product has an `upgrade` subcommand (`crates/opencode-cli/src/main.rs:266`,
  `handle_upgrade_command` at `:4183`), but it is vanilla-derived brew/npm/curl install-method
  logic; it does not know about the artifact layout, `ort-install`, or the stable/preview
  channel model.
- The TUI has reusable keybind/dialog patterns for session actions (for example
  `FEAT-025`/`BUG-015` rename dialog), which this feature can mirror.

## Scope

- Version check:
  - Resolve the latest published revision for the active channel (`stable` default, `preview`
    opt-in) from the release manifest / `<asset>.tar.gz.rev` sidecar, without changing the
    channel model (`INFRA-013`).
  - Fetch with a short timeout, cache the result, throttle checks (for example once at startup
    and then on a configurable interval), and never block the UI or a model request.
  - Offline, rate-limited, or unknown responses mean "no notice" (keep working normally).
- Comparison and notice:
  - Compare the published rev to the installed rev (not just semver, since semver can repeat
    across revisions) and show a non-blocking TUI notice when a newer rev is available, for
    example in the status line/toast: `Update available: <version> (<short rev>) - press <key>`.
  - The notice must be dismissible and must not steal focus from a running turn.
- Install action:
  - A hotkey/command-palette action opens an update dialog showing current vs available
    version/rev/channel and offers Install.
  - Install through the artifact path (`ort-install`, checksum-verified) rather than the
    vanilla `upgrade` install-method logic; keep `ort-install` as the source of truth for
    channel/rev resolution and verification.
  - Show progress/errors, then tell the user to restart the session to apply. Do not hot-swap
    or exec over the running process.
  - Do not auto-install without an explicit user action (this supersedes the current silent
    background update as the user-facing behavior; see the decision below).
- Configuration/safety:
  - Respect `OPENCODE_RUST_CHANNEL`, `OPENCODE_RUST_AUTO_UPDATE`,
    `OPENCODE_RUST_UPDATE_INTERVAL`, and an explicit notify/check opt-out.
  - Never install an asset whose `.sha256` does not verify, and never follow a downgrade
    unless explicitly requested.
- Documentation:
  - README/docs: version visibility, notice behavior, hotkey, channel interaction, and how to
    disable checks.

## Decisions / open questions

- Decide whether to keep the silent background auto-update as a fallback, make it opt-in, or
  remove it once notify+install exists. Recommendation: default to **notify + consent** so
  machine revisions stop drifting invisibly; retain the silent path only behind an explicit env
  opt-in.
- Decide where the check runs: TUI startup plus a periodic tick, or server-side with a TUI
  surface. It must avoid adding startup latency and must not require network access to launch.
- Decide whether the product shells out to `ort-install` or grows a native artifact updater.
  Reusing `ort-install` avoids duplicating channel/checksum logic but couples the product to a
  standards utility; document the chosen contract.

## Non-goals

- Changing release publishing (`INFRA-011`) or fleet deploy (`INFRA-012`).
- Changing the channel model or auto-update defaults for the fleet (`INFRA-013`).
- Self-updating the running process in place, or downgrade/rollback UX from inside the TUI.
- Backfilling version info into old sessions or changing session semantics.

## Done when

- With an older installed rev and a newer published stable rev, the TUI shows a non-blocking
  update notice naming the available version/rev.
- The hotkey opens an update action, installs the newer rev with checksum verification, and the
  next launch reports the new version (`ort-verify`/`ort --version` agree).
- With the latest rev installed, no notice is shown.
- Offline, timeout, and rate-limited cases are silent and do not block startup.
- Tests cover the rev comparison, notice gating, and check throttling/opt-out; README/docs are
  updated.
- The chosen behavior relative to the existing silent auto-update is recorded on this card.

## Decisions (resolved 2026-11-16)

- **Silent auto-update vs notify:** the product defaults to **notify + explicit consent**. The
  launcher's pre-launch silent update stays a separate standards-side control
  (`OPENCODE_RUST_AUTO_UPDATE`); making it opt-in so fleet machines visibly show the new notice
  is a follow-up in `~/standards`, not this repo. Interim: set
  `OPENCODE_RUST_AUTO_UPDATE=0` to let the in-product notice drive updates.
- **Where the check runs:** in the TUI, on a background thread at startup and on a throttled
  tick, backed by a shared `opencode-util::update` module. It needs no server feature and adds
  no startup network latency.
- **Install contract:** shell out to `~/standards/utilities/ort-install` with the active
  channel and `ORT_BIN_ROOT`. `ort-install` remains the source of truth for channel
  resolution, checksum verification, and the `bin/<rev>` layout. Restart applies the update;
  the running process is never hot-swapped.

## Implementation notes

- PR: https://github.com/cchris-p/opencode-modded-rust/pull/196
  (`feature/FEAT-080-update-notice`, worktree).
- `crates/opencode-util/src/update.rs`: revision-layout detection, channel/tag/`.rev`
  resolution, rev comparison (semver used only to suppress downgrades), on-disk cache +
  dismissal, throttling, and `ort-install` invocation. GitHub token comes from `GH_TOKEN` /
  `GITHUB_TOKEN` or the managed `~/.config/opencode/.env`.
- TUI: Ctrl+G default keybind, `/update` command, `UpdateDialog`, status-line notice and toast,
  `CustomEvent::UpdateChecked` / `UpdateInstallFinished`, startup + periodic background checks.
  Development/`target/debug` builds are treated as "not artifact-installed" and stay silent.
- CLI: `upgrade --check` and artifact-aware `upgrade` (blocking work runs on
  `spawn_blocking`; the vanilla upgrade path is unchanged for non-artifact installs).
- README documents notice behavior, hotkey, channel interaction, throttling, and
  `OPENCODE_RUST_UPDATE_CHECK=0` / `OPENCODE_RUST_UPDATE_INTERVAL`.

## Verification (2026-11-16)

- `cargo test -p opencode-util` — 18 passed.
- `cargo test -p opencode-cli` — 25 passed.
- New TUI tests pass: status-line notice text, update dialog install gating, `/update`
  command alias, Ctrl+G keybind. Full `cargo test -p opencode-tui` still hits the pre-existing
  `tab_autocomplete_uses_first_candidate` flake (documented on several QA cards).
- `cargo fmt --all -- --check` clean; clippy only pre-existing warnings.
- Temp-root smoke: older rev -> `Available: v0.1.4 (cc9befa)`; current rev -> `Already up to
  date.`; dev build -> `not managed by ort-install`; offline proxy -> silent in TUI path;
  `upgrade` installed the checksum-verified artifact and flipped the temp `current` symlink.
- Fleet `ort --version` identity comes from CLI-017 (already merged); notice carries the rev
  that `ort-verify` reports.

## Related Items

- `CLI-017` Show the ORT version for `ort --version` and `omr --version` (identity/visibility).
- `INFRA-007` Consume the published ORT CLI binary instead of per-machine repo clones.
- `INFRA-011` Automated versioned release.
- `INFRA-012` Automatic fleet deploy on main release.
- `INFRA-013` Make the stable main release the default ORT channel.
- `INFRA-014` Post-deploy fleet verification and rollback (`ort-verify`).
- `INFRA-015` Release/deployment invariant.
- `FEAT-025` Map Ctrl+R to rename (keybind/dialog pattern to mirror).

## Notes

- Created 2026-11-16 at the user's request: the system should detect a new published version
  and suggest installing the latest via a hotkey or equivalent.
- The silent updater (`_opencode_rust_maybe_auto_update`) already exists; this card is about
  making the state visible and giving the user a deliberate install path, not about adding
  another auto-updater.

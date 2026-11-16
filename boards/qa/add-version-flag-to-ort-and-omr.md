---
id: "CLI-017"
title: "Show the ORT version for ort --version and omr --version"
priority: "P2"
type: "bug"
area: "CLI"
spec: "README.md"
status: "qa"
created: "2026-11-16"
updated: "2026-11-16"
---

# Show the ORT version for ort --version and omr --version

## Summary

`ort --version` currently fails instead of printing the product version. The only working
paths are `opencode-modded-rust version` and `omr --version`/`omr version` (which route the
`version` subcommand). Add first-class `-V`/`--version` support so `ort --version` prints the
version and exits 0.

## Current behavior (verified on matrillobase1, 2026-11-16)

Installed artifact `~/.local/share/ort/bin/current/opencode-modded-rust` (rev `cc9befa1`, v0.1.4):

- `opencode-modded-rust version` -> `opencode-modded-rust 0.1.4` (works).
- `opencode-modded-rust --version` -> clap error, exit 2
  (`error: unexpected argument '--version' found`).
- `opencode-modded-rust tui --version` -> clap error, exit 2
  (`error: unexpected argument '--version' found`, tip suggests `--session`).
- `ort --version` hits the `tui --version` path and therefore errors.

Causes:

- The top-level clap command sets `disable_version_flag = true`
  (`crates/opencode-cli/src/main.rs:125`), so `--version`/`-V` is not registered at all.
  A `version` subcommand exists and prints `opencode-modded-rust <semver>`
  (`crates/opencode-cli/src/main.rs:960`).
- The standards launcher treats `--version` as a metadata arg and forwards it to the TUI
  subcommand: `_opencode_rust_is_metadata_arg` matches `version`/`--version`
  (`~/standards/opencode-config:1278`), then `opencode-rust-tui` calls
  `_opencode_rust_exec tui "$@"` (`~/standards/opencode-config:1762`).
- `ort` is TUI-only by design (`ort() { opencode-rust-tui "$@"; }`), while `omr` is the general
  launcher; only `omr --version` maps to the `version` subcommand today.

## Scope

- Product (this repo):
  - Register `-V`/`--version` at the top-level clap command (remove or condition the
    `disable_version_flag` setting) or add explicit pre-parse handling, and print the same
    identity string as the `version` subcommand (`opencode-modded-rust <semver>`).
  - Keep `opencode-modded-rust version` and `omr version` behavior unchanged.
  - Decide whether the printable identity should also include the installed revision and
    channel, in coordination with `INFRA-013`. If it does, it must be stable for tests and
    must not require network access; a build-time commit or artifact rev sidecar is acceptable.
  - Add a CLI test that `--version`/`-V` exits 0 and prints the expected identity, and that
    `tui --version` is either rejected intentionally or mapped.
- Launcher (standards repo, not this repo):
  - Change `_opencode_rust_is_metadata_arg`/`opencode-rust-tui` routing so `ort --version`,
    `ort -V`, and `ort version` resolve to the binary's `version` subcommand instead of
    `tui --version`.
  - Keep `omr --version`/`omr version` and bare `omr` behavior intact, and do not make `ort`
    a general launcher.
  - Verify with the artifact path (`OPENCODE_RUST_USE_CHECKOUT=0`) and with
    `OPENCODE_RUST_USE_CHECKOUT=1` on the dev host, since `ort` is artifact-first.

## Non-goals

- Redesigning `ort` into the general `omr` launcher, or moving every `omr` subcommand into
  `ort`.
- Changing the semver scheme or release/channel model (`INFRA-013` owns that).
- Printing commit/rev/channel through every subcommand.

## Done when

- `ort --version` and `ort -V` print `opencode-modded-rust <version>` and exit 0 on an
  artifact-only host.
- `omr --version`, `omr version`, and `opencode-modded-rust version` still work.
- The standards launcher change is merged, and the routing is covered by a test or a recorded
  manual smoke.
- README/launcher docs mention `--version` where launcher flags are documented.

## Dev Notes

Implemented on `bug/CLI-017-ort-version-flag` (worktree).

- Product: added `#[command(version)]` to the top-level clap `Cli` in
  `crates/opencode-cli/src/main.rs`, registering `-V`/`--version`. It prints the same
  `opencode-modded-rust <semver>` identity as the `version` subcommand. The `oco` subcommand's
  own `disable_version_flag` is unchanged.
- Launcher (standards repo, committed separately on `development`): added
  `_opencode_rust_is_version_arg` and routed a leading `version`/`--version`/`-V` in
  `opencode-rust-tui` to `_opencode_rust_exec version`, so `ort --version` no longer becomes
  `tui --version`. `omr --version` behavior is unchanged.
- Tests: `version_flags_print_the_package_version` (both flags produce `DisplayVersion` with
  the crate version) and `version_subcommand_still_parses`.

Verification:

- `cargo test -p opencode-cli` -> 25 passed, 0 failed.
- `cargo fmt -p opencode-cli -- --check` -> clean.
- Built the binary with the prebuilt native bundle and confirmed:
  `--version`, `-V`, and `version` each print `opencode-modded-rust 0.1.4`, exit 0.
- Sourced `~/standards/opencode-config` and confirmed `ort --version` and `omr --version`
  print the version and exit 0.
- `cargo clippy -p opencode-cli --all-targets` only reports pre-existing warnings unrelated to
  this change.

Standards-side commit: `54abb1c` in `~/standards` on `development` (pushed).

## Related Items

- `CLI-013` Reach the Rust CLI subcommands through the launcher (established `omr`).
- `START-033` Decouple the product command identity from opencode (Layer 1 identity).
- `INFRA-013` Make the stable main release the default ORT channel (channel/revision
  visibility).
- `INFRA-014` Post-deploy fleet verification and rollback (`ort-verify` reports rev/version).
- `FEAT-080` Detect and suggest new ORT release updates (uses the same version/rev identity).

## Notes

- Created 2026-11-16 from `ort --version` failing on matrillobase1. The user wants
  `ort --version` to show the version directly.
- The launcher lives in `~/standards` (`opencode-config`); the product flag behavior lives in
  this repo. Both must land for the user-visible command to work, and the standards-side change
  belongs in its own repo/worktree.
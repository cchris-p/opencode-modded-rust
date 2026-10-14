---
id: "INFRA-006"
title: "Build ort and scopemux centrally and consume published binaries"
priority: "P1"
type: "chore"
area: "INFRA"
spec: ""
status: "doing"
created: "2026-10-14"
updated: "2026-10-14"
---

# Build ort And Scopemux Centrally And Consume Published Binaries

## Summary

The native `scopemux` provider and the `ort` binary are built per machine today:
`ort-build` fetches `third_party/scopemux-core` (itself a repo with nested
tree-sitter/pybind11 submodules) and compiles the C provider locally. That makes
every machine carry a multi-GB build tree and a heavy nested-submodule chain, and
it drives the `standards` checkout to 4.3G.

Build once in CI and distribute artifacts: GitHub Actions builds `scopemux-core`
(and the CLI), publishes release binaries, and machines download the prebuilt
artifacts instead of cloning and building from source.

## Context

- `scripts/fetch-scopemux-core.sh` clones the pinned `scopemux-core` into
  `third_party/scopemux-core` and initializes nested `tree-sitter-*`/`pybind11`
  submodules; `crates/opencode-scopemux/build.rs` compiles the native provider.
- `vendor/scopemux-core` declares 8 nested submodules (tree-sitter,
  tree-sitter-c/cpp/python/javascript/typescript/rust, pybind11) plus test-cases.
- Direction from the operator: tree-sitter and similar low-churn nested deps
  should be **downloaded prebuilt binaries**, not declared as submodules — and
  the same treatment applies to nested submodules in both
  `opencode-modded-rust` and `standards`.
- `INFRA-003` scopes a CI clean-clone native build guard; this card is its
  publish/distribute counterpart.

## Scope

- Add a GitHub Actions workflow that builds `scopemux-core` native artifacts (and
  the `ort`/CLI binary) and publishes them as release artifacts.
- Add an ensure/download path so a machine obtains the prebuilt artifacts instead
  of building from source; integrate with `ort-build`/`ort-setup`.
- Replace low-churn nested submodules (tree-sitter grammars, pybind11) with
  downloaded prebuilt binaries; drop them from `.gitmodules` where the prebuilt
  path replaces source builds.
- Apply the same treatment to nested submodules in `standards` where a prebuilt
  artifact is viable.
- Document the CI + download path and keep a documented source-build fallback for
  contributors.

## Non-goals

- Removing per-machine local builds entirely; a contributor source-build path stays.
- The `standards` submodule-decoupling mechanics — tracked as `STD-083`.

## Done When

- A clean machine can obtain a working `ort`/native provider by downloading a
  published artifact, with no `scopemux-core` source tree fetched.
- CI builds and publishes the artifacts on release (or on a pin bump).
- Tree-sitter/pybind11 are no longer required as submodules for the download path.
- Docs state the artifact source, versioning, and the source-build fallback.

## Related Items

- `INFRA-002` automate scopemux-core fetch and pin bumps.
- `INFRA-003` add a CI clean-clone native build guard.
- `INFRA-004` align checkout/setup paths and trim disk footprint.
- `STD-083` decouple managed vendor repos from the standards checkout
  (`standards` board).

## Related Docs

- `AGENTS.md` "Local Launchers"
- `scripts/fetch-scopemux-core.sh`, `scripts/bump-scopemux-pin.sh`

## Implementation - 2026-10-14 (pass 1)

Built the publish + download path and the consumer, so a machine can link a
prebuilt native provider with no scopemux-core source tree, CMake, Python, or
Tree-sitter/pybind11 submodules.

- `.github/workflows/native-artifacts.yml` — builds on `ubuntu-latest` and
  `macos-14`: fetches the pinned scopemux-core, packages the native provider
  bundle, builds the release CLI binary, uploads workflow artifacts, and (on tag
  push or `workflow_dispatch`) publishes assets to the rolling `native-artifacts`
  release.
- `scripts/package-native-artifacts.sh` — builds the C provider through the
  crate's own `cargo build -p opencode-scopemux --features native` (same CMake
  invocation as `build.rs`) and stages `lib/libparser_core.a` +
  `lib/libtree-sitter-*.a` + `queries/` + `manifest.json` into
  `opencode-native-<os>-<arch>-<pinshort>.tar.gz` (+ `.sha256`).
- `scripts/ensure-native-artifacts.sh` — resolves platform + pin, downloads the
  bundle (over `gh`/`curl`, or `SCOPEMUX_PREBUILT_URL` for a mirror), verifies the
  checksum, extracts into `~/.cache/opencode-modded-rust/native/`, and prints the
  dir for `SCOPEMUX_PREBUILT_DIR`.
- `crates/opencode-scopemux/build.rs` — when `SCOPEMUX_PREBUILT_DIR` is set and
  complete, links the prebuilt libs + bakes the bundle's `queries/` and returns
  before any CMake/submodule work; the source build remains the fallback
  (`SCOPEMUX_CORE_DIR` wins, `SCOPEMUX_SKIP_NATIVE_BUILD` still skips).
- Docs: `README.md` "Native provider artifacts"; `AGENTS.md` "Local Launchers";
  the product checkout path refreshed to `$HOME/apps/opencode-modded-rust`
  (`STD-083`).

Standards side (`opencode-config`, `STD-083`/INFRA-006 integration):
`opencode-rust-ensure-native-artifacts` calls the product's ensure script and
exports `SCOPEMUX_PREBUILT_DIR`; `opencode-rust-ensure-scopemux-core` uses it
first and falls back to the source build. Disable with
`OPENCODE_RUST_PREBUILT_NATIVE=0`.

Verification (macOS arm64): produced a bundle with `package-native-artifacts.sh`;
`cargo build -p opencode-scopemux --features native` linked it with no
`third_party/scopemux-core` present (build-script output confirms the bundle
`link-search`/`QUERIES_DIR`); with no bundle/source it stalls on the expected
"scopemux-core sources not found" error; `ensure-native-artifacts.sh` was
exercised over a local HTTP server (download, checksum, extract, `--check`).

Remaining: CI release-asset publish is verified only once the workflow runs on
GitHub; the nested Tree-sitter/pybind11 submodules in the `scopemux-core` repo
can now be dropped for the download path (separate `scopemux-core` change); the
standards-side `opencode-config` hook lands with this pass. The workflow ships
Linux-only for now: the pinned `scopemux-core` fails to compile under the macOS
runner's older AppleClang (`symbol_management.c` has a label-followed-by-
declaration, a C23 extension), which needs a one-line `scopemux-core` fix before
the macOS bundle can be re-enabled.

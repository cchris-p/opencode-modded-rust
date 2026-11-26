---
id: "INFRA-003"
title: "Add a CI clean-clone native build guard"
priority: "P3"
type: "chore"
area: "INFRA"
spec: ""
status: "done"
created: "2026-09-26"
updated: "2026-11-25"
---

# Add a CI clean-clone native build guard

## Summary

The product has no `.github/workflows`. A fresh clone, a stale pin, or a
tree-sitter ABI mismatch is only caught on a developer machine at build time. Add
CI that reproduces the native build from a clean checkout.

## Why this exists

`INFRA-002` makes the local build self-healing, but nothing guards the committed
pin or a fresh clone in CI. The provider-activation QA found a tree-sitter ABI
mismatch that CI would have caught.

## Prerequisite (human decision)

`$HOME/apps/scopemux-core` is a private repository, so a CI job that fetches it
needs credentials the default `GITHUB_TOKEN` does not provide:

- a deploy key / SSH secret for `cchris-p/scopemux-core`, or
- a fine-grained PAT with read access, or
- a GitHub App token.

Decide the mechanism and add the secret before enabling the native job.

## Scope

- Job A (no secret): `cargo fmt --check`, `cargo clippy`, and
  `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test --workspace` — validates the Rust
  product without the core.
- Job B (secret-gated): on a clean runner, run `scripts/fetch-scopemux-core.sh`,
  then `cargo build -p opencode-cli` (native default) or
  `cargo test -p opencode-scopemux --features native`. Must skip cleanly when the
  secret is absent.
- Cache cargo plus the fetched core to keep the job affordable.

## Done when

- A PR that breaks the Rust side fails Job A.
- A pin bump or ABI regression fails Job B.
- Both jobs are documented in `AGENTS.md`.

## Related Items

- `INFRA-002` local fetch and drift automation (this is the CI counterpart).

## Delivered / superseded (2026-11-25)

The premise is stale: `.github/workflows/ci.yml` (INFRA-009) and
`.github/workflows/native-artifacts.yml` (INFRA-006) now cover this card.

- Job A equivalent: `ci.yml` runs `cargo fmt --all --check`,
  `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo clippy --workspace --all-targets`, and
  `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test --workspace --exclude opencode-cli`.
- Job B equivalent: `ci.yml` `native` job checks out clean, installs cmake, runs
  `scripts/fetch-scopemux-core.sh`, packages the native bundle, runs
  `cargo test -p opencode-scopemux --features native`, `cargo test -p
  opencode-cli`, builds the release CLI, and smoke-tests portable queries.
- Caching: cargo registry/git/target caches are keyed on `Cargo.lock`.
- Prerequisite resolved: `github.com/cchris-p/scopemux-core` is now public, so the
  clean-clone fetch needs no deploy key/PAT (`git ls-remote` anonymous HTTPS
  succeeds, verified 2026-11-25). If it becomes private again, add token auth to
  the `native` job.
- Documented in `AGENTS.md` under CI and release deployment.

No new workflow was added; a duplicate native job would only add cost.

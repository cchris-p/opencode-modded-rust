---
id: "INFRA-003"
title: "Add a CI clean-clone native build guard"
priority: "P3"
type: "chore"
area: "INFRA"
spec: ""
status: "todo"
created: "2026-09-26"
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

---
id: "FUTURE-PROOFING-016"
title: "Package the native workspace-intelligence provider as an artifact"
priority: "P1"
type: "chore"
area: "FUTURE-PROOFING"
spec: "wiki/workspace-intelligence.md"
status: "todo"
created: "2026-10-24"
---

# Package the native workspace-intelligence provider as an artifact

## Summary

Make the native workspace-intelligence provider consumable as a pinned artifact, with
independent versioning for the provider and the map schema.

The source-build path stays for contributors; the user path must not require CMake,
Python headers, or a `scopemux-core` checkout.

## Why this exists

A capability that requires a C toolchain on every machine is not future-proof. It is a
deployment burden, a support surface, and a reason to disable the capability. The
`INFRA-006`/`INFRA-007` work already publishes native bundles and the CLI; WIP should
consume that path.

## Scope

- Consume the existing native-artifact download path for `scopemux-core`.
- Publish the WIP native provider with a version and checksum.
- Version the map schema independently from the provider binary.
- Add a health/compatibility check at startup.
- Degrade to generic retrieval when the artifact is missing, incompatible, or fails to
  load.
- Keep the source-build fallback for contributors.
- Document the artifact source, versioning, and fallback in `wiki/workspace-intelligence.md`.
- Add a CI check that a clean machine can enable WIP from the artifact with no
  `scopemux-core` source tree.

## Acceptance criteria

- A clean machine can run WIP from the published artifact with no source checkout.
- The provider and map schema versions are recorded and checked.
- A missing/incompatible artifact falls back to generic retrieval.
- The source-build path still works for contributors.
- CI covers the artifact path.
- The artifact is pinned and reproducible.

## Non-goals

- Publishing to a public package registry.
- Rewriting the native engine in Rust.
- Removing the source-build fallback.
- Multi-platform packaging beyond the supported fleet.

## Related

- `FUTURE-PROOFING-013`
- `FUTURE-PROOFING-014`
- `INFRA-006` native artifacts
- `INFRA-007` published CLI binary
- `wiki/workspace-intelligence.md`
- `invariants/workspace-intelligence.md`

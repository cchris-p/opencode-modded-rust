---
id: "SKILLS-003"
title: "Achieve exact local skills list parity"
priority: "P1"
type: "feature"
area: "SKILLS"
spec: "invariants/skills/discovery.md"
status: "doing"
created: "2026-09-19"
---

# Achieve exact local skills list parity

## Summary

Make locally discovered skill lists exactly match between vanilla OpenCode and OpenCode Rust for the same global and project-local filesystem skill environment.

## Why this exists

`SKILLS-001` aligned the broad local-filesystem skills behavior, but the required invariant is stricter: for locally discovered skill files, vanilla OpenCode and OpenCode Rust must report the same skill names from the same workspace and user configuration context.

The intended reference is the locally available vanilla OpenCode implementation, not a hand-maintained copied list. In normal development environments both products are expected to be available locally, so Rust parity can be verified by running both discovery implementations against the same fixture workspace and global/config roots.

URL-backed skills are explicitly excluded from this item and remain tracked by `SKILLS-002`.

## Strong Invariant

The locally discovered skill list must be exactly the same between vanilla OpenCode and OpenCode Rust for the same global and project-local environment.

This includes global and project-local skill files discovered from supported local filesystem roots.

This excludes URL-loaded skills, which are tracked separately by `SKILLS-002`.

## Scope

- Compare vanilla OpenCode and OpenCode Rust local skill discovery behavior against the same test fixture environment.
- Treat the local vanilla OpenCode checkout as the executable/source reference when available, rather than copying skill files as the normal implementation strategy.
- Align Rust discovery roots, file patterns, frontmatter handling, duplicate-name precedence, and output ordering with vanilla for local filesystem skills.
- Add durable Rust-side parity fixtures that model vanilla-compatible local skill layouts.
- Add an optional developer parity check, script, or test path that can run both products locally and compare discovered local skill names exactly.
- Update `invariants/skills/discovery.md` so exact local list parity is an explicit invariant.

## Out of Scope

- URL-backed skill sources and any remote fetch/cache behavior.
- Making Rust depend on vanilla OpenCode at runtime.
- Copying a fixed set of vanilla skill files into production discovery behavior.
- Broad post-parity skill feature expansion beyond local skill discovery/listing equality.

## Implementation Notes

- Start from the current vanilla skill discovery implementation in `$HOME/repos/opencode-modded` and this repo's `crates/opencode-tool/src/skill.rs`.
- Prefer a parity fixture harness over copied real-user skills. The harness should create the same temporary global/config/project roots for both implementations and compare the final discovered name list.
- If direct cross-product execution is too brittle for CI, keep the cross-product comparison as an explicit local developer verification command and back it with Rust unit/integration tests that encode the observed vanilla rules.
- Any copied vanilla `SKILL.md` files should be test fixtures only, used to represent valid vanilla-compatible skill formats. Do not make copied files the source of truth for production behavior.

## Acceptance Criteria

- Given the same local global and project skill files, vanilla OpenCode and OpenCode Rust discover the exact same local skill names.
- Global skill roots and project-local skill roots are treated compatibly between both implementations.
- Duplicate local skill names resolve to the same surviving skill in both implementations.
- Sorting/order of the listed local skills matches the product contract or is normalized consistently before exact comparison, with the chosen rule documented.
- URL-backed skills are not included in this parity requirement.
- Tests cover representative vanilla-compatible skill files and local root layouts.
- A local parity verification path exists for machines that have both vanilla OpenCode and OpenCode Rust checkouts.
- `invariants/skills/discovery.md` records the exact-list-parity invariant.

## Related Items

- `SKILLS-001` established local filesystem skill behavior parity.
- `SKILLS-002` tracks URL-backed skills parity planning.

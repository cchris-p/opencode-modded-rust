---
id: "START-034"
title: "Scrub scopemux-code product-name references"
priority: "P2"
type: "refactor"
area: "START"
spec: "invariants/reference-boundary.md"
status: "qa"
predecessors: "START-033"
created: "2026-10-14"
updated: "2026-10-14"
---

# Scrub scopemux-code product-name references

## Summary

The product is named `opencode-modded-rust`; `scopemux-code` is no longer the product name and its
lingering use adds confusion. Remove `scopemux-code` product-name references across product-facing
docs, wiki, invariants, and code, and correct the canonical-name rule that still declares it
canonical. This sits between `START-033` (command/binary identity) and `CLI-013` (launcher
reachability).

## Why this exists

- `START-003` declared `scopemux-code` canonical. That decision is superseded; the product is now
  `opencode-modded-rust`.
- `scopemux-code` still appears across current and authoritative surfaces, including the invariant
  that is supposed to be the source of truth (`invariants/reference-boundary.md`), `README.md`,
  `USER_GUIDE.md`, `docs/`, and `wiki/`, so the naming is internally contradictory and confusing.
- `opencode-modded-rust` must be the single canonical name wherever the product names itself.

## Critical boundary — do NOT scrub ScopeMux

Only the **product-name token `scopemux-code`** is in scope. The **ScopeMux integration** is a real,
separate subsystem and must be left intact:

- keep `scopemux-core`, `scopemux-notes`, the native ScopeMux provider, `SCOPE-*` cards, and
  `invariants/integration-scope.md`.
- keep product references to `scopemux`/`ScopeMux` where they describe the integration, not the
  product name.

When in doubt, a `scopemux-code` that names *the product* is scrubbed; a `scopemux`/`scopemux-core`
that names the *integration* is kept.

## Inventory (as of 2026-10-14)

Current / authoritative surfaces to correct:

- `invariants/reference-boundary.md` (canonical-name line), `invariants/README.md`,
  `invariants/documentation-boundary.md`.
- `README.md`, `USER_GUIDE.md`, `docs/README.md`, `docs/opencode-tui.md`.
- `wiki/`: `product-boundary.md`, `vision.md`, `v1.md`, `v1-runtime-loop.md`, `reference-strategy.md`,
  `README.md`, `agent-evaluation-strategy.md`, `scopemux-map-integration.md`, `laya-integration.md`,
  `jev-ultrafast-integration.md` (verify each: product name vs integration).

Historical surfaces (see decision below):

- `boards/done/standardize-product-name.md`, `freeze-ts-reference-line.md`,
  `idealized-scopemux-map-integration.md`, `adopt-cross-repo-doc-boundary.md`.
- `boards/archive/rebrand-tui-branding-constants-to-scopemux-code.md`.
- `docs/transcripts/feat-023-confirm-done-and-delete-duplicate.md`.
- The two refinement cards in this set (`START-033`, `CLI-013`) if they still reference the name.

## Scope

- Replace `scopemux-code` with `opencode-modded-rust` (the product name) across current/authoritative
  docs, wiki, and invariants.
- Correct `invariants/reference-boundary.md` to make `opencode-modded-rust` canonical, and supersede
  `START-003` in the plan.
- Sweep any `scopemux-code` string in `crates/` and `scripts/`.
- Add a superseding note to historical cards/transcripts rather than rewriting their history (see
  Resolved Decisions).
- Leave `RustingOpenCode`/`ROCode` runtime-string replacement to `START-033` (branding constants);
  this card handles the `scopemux-code` name text.

## Non-goals

- The command/binary/clap identity rename (`START-033`).
- Launcher subcommand passthrough (`CLI-013`).
- Layer 2 config/state/env paths (`START-033`, deferred).
- Renaming crates, the repo, or the ScopeMux integration.

## Resolved Decisions (locked 2026-10-14)

- **Historical surfaces** (`boards/done/**`, `boards/archive/**`, `docs/transcripts/**`): annotate
  with a superseding note referencing `START-034`; do **not** rewrite their history.
- **`wiki/*-integration.md` hits:** scrub when the token names the *product*; keep when it names the
  *ScopeMux integration*.

## Done when

- No current/authoritative doc, wiki, or invariant uses `scopemux-code` as the product name.
- `invariants/reference-boundary.md` states `opencode-modded-rust` as canonical and marks
  `scopemux-code` as a retired/legacy name.
- No `scopemux-code` string remains in `crates/` or `scripts/`.
- ScopeMux integration references are demonstrably untouched.
- Historical surfaces are annotated or left per the resolved decision.

## Recommended verification

- `rg -i "scopemux-code" . --glob '!target/**' --glob '!.git/**'` returns only intentional historical
  annotations (or nothing) and the ScopeMux integration references.
- `rg -i "scopemux-code" crates/ scripts/` returns nothing.
- Re-read `invariants/reference-boundary.md` and confirm the canonical-name rule is correct.

## Related Items

- `START-033` Decouple the product command identity from opencode — predecessor; sets the name.
- `START-003` Standardize product name — superseded decision this card corrects.
- `CLI-013` Reach the Rust CLI subcommands through the launcher — follows this card.
- `invariants/integration-scope.md` — ScopeMux integration boundary that must be preserved.

## Notes

- Created 2026-10-14 to sit between `START-033` and `CLI-013`, per user direction, because the stale
  `scopemux-code` name adds confusion.

## Implementation Notes

- 2026-10-14: implemented on `feature/START-034-scrub-scopemux-code` (stacked on PR #142).
  - `invariants/reference-boundary.md`: `opencode-modded-rust` canonical; `scopemux-code` marked
    retired/legacy.
  - Replaced `scopemux-code` with `opencode-modded-rust` in `README.md`, `USER_GUIDE.md`,
    `docs/README.md`, `docs/opencode-tui.md`, `wiki/` (product-boundary, README, reference-strategy,
    vision, v1, v1-runtime-loop, agent-evaluation-strategy, scopemux-map-integration,
    laya-integration, jev-ultrafast-integration), `invariants/README.md`, and
    `invariants/documentation-boundary.md`.
  - `docs/opencode-tui.md` branding updated to `APP_NAME=opencode-modded-rust`, `APP_SHORT_NAME=omr`.
  - Historical surfaces annotated (not rewritten) with a `START-034` superseding note:
    `boards/done/{standardize-product-name,freeze-ts-reference-line,idealized-scopemux-map-integration,adopt-cross-repo-doc-boundary}.md`,
    `boards/archive/rebrand-tui-branding-constants-to-scopemux-code.md`,
    `docs/transcripts/feat-023-confirm-done-and-delete-duplicate.md`. Supersedes `START-003`.
  - ScopeMux integration (`scopemux-core`, `scopemux-notes`, `SCOPE-*`,
    `invariants/integration-scope.md`) untouched.
- Verification: `rg -i "scopemux-code" crates/ scripts/` empty; repo-wide matches are historical
  annotations/retired-name notes only.
- PR: [#143](https://github.com/cchris-p/opencode-modded-rust/pull/143) (`development`, open).

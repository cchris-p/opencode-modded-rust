---
id: "INFRA-009"
title: "Add a required CI test suite for development and main"
priority: "P1"
type: "chore"
area: "INFRA"
spec: "wiki/workspace-intelligence.md"
status: "qa"
created: "2026-11-05"
updated: "2026-11-08"
---

# Add A Required CI Test Suite For Development And Main

See invariant: `invariants/release-deployment.md`.

## Summary

The repository has only the `native-artifacts` workflow. There is no PR/push test
workflow, so `main` can advance without a green build, test, lint, or portability check.

Add a required CI test suite that gates every PR to `development` and `main`, and every
push to those branches. It must catch the classes of regression that matter for deployment:
compilation, unit/integration tests, formatting, clippy, the native ScopeMux provider, and
the portable query bundle.

## Current State

- `.github/workflows/native-artifacts.yml` builds/packages the native bundle and CLI but its
  test surface is only the INFRA-008 clean-host query smoke.
- No `ci.yml`/`tests.yml`; `README.md` lists local commands only.
- `cargo test` has documented pre-existing native failures (`SCOPE-013`):
  `native_provider_reports_plan_reconciliation_evidence` and
  `native_delta_slice_returns_projected_entries`. The CI policy must handle these honestly
  (fix, quarantine with an owner/date, or add the owner-visible failure gate) rather than
  hide them.

## Scope

- Add `.github/workflows/ci.yml` with jobs:
  - `fmt`: `cargo fmt --all --check`.
  - `check`: `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo check --workspace`.
  - `clippy`: `cargo clippy --workspace --all-targets` with the repo's accepted lint level.
  - `test`: `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test --workspace`.
  - `native`: build with the published/prebuilt native bundle, run
    `cargo test -p opencode-scopemux --features native`, and run the INFRA-008 clean-host
    `debug scopemux-queries` smoke.
  - Linux on `ubuntu-22.04` (oldest fleet glibc) and macOS on `macos-14`.
- Cache Cargo artifacts; set timeouts and `concurrency` so superseded runs cancel.
- Publish a single stable check name (or a small required set) for branch protection.
- Add or repair tests needed to make the suite trustworthy:
  - make the pre-existing native failures either fixed or explicitly quarantined with a
    tracked board ID and a `#[ignore = "..."]` reason;
  - add a regression test that fails if the packaged binary loses the embedded query set;
  - add a smoke test that the CLI starts and reports a version on a clean `HOME`.
- Keep publishing out of this workflow; release publication belongs to `INFRA-011`.

## Acceptance Criteria

- Every PR to `development`/`main` runs fmt, check, clippy, tests, native tests, and the
  clean-host smoke.
- A failing job blocks merge once `INFRA-010` protects the branch.
- The pre-existing native failures are resolved or explicitly quarantined with a board ID
  and reason, not silently omitted.
- The workflow does not publish artifacts or need release secrets.
- Runtime and flakiness are acceptable (target under roughly 20 minutes per platform).

## Verification

- Open a throwaway PR that breaks formatting, a test, and the query smoke separately; each
  must fail the relevant job.
- Open a throwaway PR with only a docs change; the suite must pass.
- Confirm the native job resolves the prebuilt bundle and materializes queries from the
  binary, not from a checkout.

## Non-Goals

- Release publication or fleet deployment (`INFRA-011`, `INFRA-012`).
- Branch protection configuration (`INFRA-010`).
- Full multi-OS coverage beyond Linux and macOS.
- Replacing the existing native-artifacts publication workflow.

## Related Items

- `INFRA-006`, `INFRA-007`, `INFRA-008` (native bundle, CLI artifact, portable queries).
- `INFRA-010` protect main and require these checks.
- `INFRA-011` automated versioned release.
- `INFRA-015` release/deployment invariant.

## Implementation And QA (2026-11-08)

Added `.github/workflows/ci.yml` and merged as PR #172 (`3772bf1`).

Jobs and stable check names (consumed by `INFRA-010`):

- `fmt` - `cargo fmt --all --check`
- `check` - `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo check --workspace --all-targets`
- `clippy` - `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo clippy --workspace --all-targets`
  (repo has not adopted `-D warnings`)
- `test` - `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test --workspace --exclude opencode-cli`
  (`opencode-cli` defaults to the native provider)
- `native (ubuntu-22.04)` / `native (macos-14)` - fetch pinned scopemux-core, package the
  prebuilt bundle, `cargo test -p opencode-scopemux --features native`,
  `cargo test -p opencode-cli`, the INFRA-008 clean-host query smoke, and a clean-HOME
  `version` smoke.

Caching, per-job `timeout-minutes`, and `concurrency` (`cancel-in-progress`) are set. The
workflow publishes nothing and needs no release secrets.

Evidence (PR #172 run `42466217181`): `fmt`, `check`, `clippy`, `test`, and
`native (ubuntu-22.04)` all pass. `native (macos-14)` fails at ~0s: the macOS runner is
unavailable/cancelled in this repo (the same pre-existing failure as the
`native-artifacts` workflow), not a code failure.

Pre-existing suite blockers repaired in the same PR: `every_workspace_crate_is_categorized`
(`opencode-oco` missing) and `policy_only_depends_on_stable_core`
(`opencode-scopemux` -> `opencode-watcher` after SCOPE-017). `opencode-watcher` is now
classified as core OS-notify infrastructure; `opencode-oco` is boundary.

Remaining QA gate before `done`: the failure-injection checks from "Verification"
(break fmt/test/query-smoke separately; docs-only PR passes) and resolving or explicitly
tracking the macOS runner availability.

---
id: "INFRA-009"
title: "Add a required CI test suite for development and main"
priority: "P1"
type: "chore"
area: "INFRA"
spec: "wiki/workspace-intelligence.md"
status: "todo"
created: "2026-11-05"
updated: "2026-11-05"
---

# Add A Required CI Test Suite For Development And Main

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

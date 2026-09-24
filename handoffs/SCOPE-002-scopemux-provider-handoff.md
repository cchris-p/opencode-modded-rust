---
id: "H-010"
title: "SCOPE-002 scopemux-core FFI provider - Handoff"
status: "in_progress"
created: "2026-09-24"
updated: "2026-09-24"
owner: ""
target: "$HOME/apps/scopemux-core"
blocked_reason: ""
needs_human: ""
items: ["SCOPE-002", "START-025", "H-001"]
---

# SCOPE-002 ScopeMux-Core FFI Provider - Handoff

## Objective

Connect the external `scopemux-core` native engine to this product as one concrete retrieval provider behind the `START-025` boundary, so task context assembly can draw on structural, symbol-aware, budgeted context (on languages `scopemux-core` parses) instead of only direct file reads. The generic provider stays the default and fallback.

## Decision

**Rust FFI against the `scopemux-core` C API** (confirmed by the product owner). It is in-process and is the only path that reaches `ProjectContext`, tiered context, search, and prompt assembly without IPC. An out-of-process helper is the fallback only if coupling the product build to CMake/tree-sitter proves unacceptable. The Python sidecar is rejected: `FIX-004` established that the exposed Python surface lacks `ProjectContext`, tiered context, search, and prompt assembly.

## Prerequisites (done)

- `START-025` boundary: `opencode-retrieval::{RetrievalProvider, GenericRepositoryProvider}` plus v1 consumption in `SessionPrompt::create_user_message` (PR #104).
- `scopemux-core` C API and build contract fixed: `FIX-001`/`FIX-003`/`FIX-004` merged (`scopemux-core` PR #7); C test baseline green (`TESTS-006`, `scopemux-core` PR #8).
- Rust is not a `scopemux-core` language, so this product's own repo exercises the generic provider by design.

## Deliverables

1. Pinned `scopemux-core` source plus reproducible build wiring.
2. An `opencode-scopemux` crate: FFI bindings and a `ScopemuxProvider` implementing `RetrievalProvider`.
3. Provider selection so the scopemux provider is opt-in and one peer among providers; the generic provider remains default.
4. Fallback and provenance: unsupported language, absent provider, or call failure leaves assembly unchanged with an explicit provenance signal.
5. Verification comparing generic and scopemux retrieval on a supported-language (C/C++/Python/JavaScript/TypeScript) workspace.

## Minimal C API surface

Headers: `$HOME/apps/scopemux-core/core/include/scopemux/project_context.h`.

- `project_context_create(root)` / `project_context_free`
- `project_add_directory(project, dir, extensions, ...)` and `project_parse_all_files`
- `project_resolve_references` and `project_context_rebuild_info_blocks`
- `project_context_search_info_blocks(project, ProjectSearchRequest)` returning `ProjectSearchResult` (hits carry a `ProjectInfoBlock`: id, qualified_name, file_path, tier, estimated_tokens)
- optionally `project_context_build_tiered_context` for stage-aware slices

## Build pinning (choose one)

- **Git submodule** `third_party/scopemux-core` pinned to a recorded commit, built by `build.rs` (the `cmake` crate) with `SCOPEMUX_BUILD_TESTS=OFF`, linking `parser_core`/`context_engine` statically. Heavier repo, fully reproducible.
- **Vendored prebuilt** static library plus headers checked in, linked by `build.rs`. No CMake at build time; needs an artifact-refresh process.

Recommendation: submodule with a `build.rs` CMake step, feature-gated so default builds do not require it.

## Implementation steps

1. Pin `scopemux-core`; add a `build.rs` that builds only the C targets needed (not the Python bindings) and emits include/link directives.
2. Add `opencode-scopemux` with `#[cfg(feature = "native")]` FFI; non-native builds compile a stub that reports the provider unavailable.
3. Implement `ScopemuxProvider::retrieve`: map `RetrievalRequest` to a `ProjectSearchRequest`, run the search, and map hits to `RetrievalCandidate`s with provenance, confidence, and token estimates.
4. Add provider selection (config/feature) keeping the generic provider default; record the provider name and reason in context provenance.
5. Tests: mapping unit tests without native, a native integration test on a multi-language fixture, and a fallback test with the provider absent.

## Verification

- `cargo fmt`, `cargo check`, `cargo clippy --workspace --all-targets`; `cargo test -p opencode-scopemux` and `-p opencode-session`.
- Same task with scopemux present and absent: identical task lifecycle, only context quality differs.
- A Rust workspace falls back cleanly with an explicit provenance signal.
- Pinned version reproducible from a clean checkout.

## Risks and findings

- The `scopemux-core` test harness is Linux/GNU only (Criterion, GNU `ld --whole-archive`); use its `scripts/docker_test.sh` for its own tests. The product-side FFI build must cover macOS and Linux C toolchains.
- `scopemux-core`'s CMake unconditionally calls `find_package(Python 3.10...<3.12)` even when only C targets are built; a CMake option to skip Python for the C-only library would simplify the product build.
- FFI struct layouts must match `project_context.h` exactly; keep the binding surface minimal and tied to the pinned commit.

## Progress (2026-09-24)

First cut implemented on `opencode-modded-rust` PR #105 (branch `feature/SCOPE-002-scopemux-provider`):

- `opencode-scopemux` crate with `ScopemuxProvider` (FFI to the C API), feature-gated `native`.
- `build.rs` builds `parser_core` + Tree-sitter static libs via CMake; bakes the queries path.
- Provider selection in `opencode-session` with generic fallback.
- `scripts/fetch-scopemux-core.sh` pins `scopemux-core` at `1a1b681`.
- Upstream portability fix merged: `scopemux-core` PR #9 (AppleClang declarations + glibc guard).
- Verified: default tests/clippy pass; native `cargo test -p opencode-scopemux --features native` passes (4 tests, FFI round-trip included).

Remaining: config-driven provider enablement, stage-aware tiered-context slices, and replacing the fetch script with a submodule or vendored artifact. The native build relaxes AppleClang-only diagnostics via `CMAKE_C_FLAGS`; a CMake option to skip Python for the C-only build remains a recommended upstream simplification.

## Exit criteria

- `ScopemuxProvider` returns ranked candidates for a supported-language workspace behind the `START-025` boundary.
- The generic provider remains default and fallback, with no scopemux logic outside the provider.
- The build is pinned and reproducible, and verification is recorded on `SCOPE-002`.

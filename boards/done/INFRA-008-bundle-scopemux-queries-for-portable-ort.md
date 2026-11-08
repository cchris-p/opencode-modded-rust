---
id: "INFRA-008"
title: "Embed ScopeMux queries so the published ort binary is portable"
priority: "P1"
type: "bug"
area: "INFRA"
spec: "wiki/workspace-intelligence.md"
status: "done"
created: "2026-11-05"
updated: "2026-11-08"
---

# Embed ScopeMux Queries So The Published ort Binary Is Portable

## Summary

The native ScopeMux provider is statically linked into the `ort` binary, but its
Tree-sitter query files (`.scm`) are **not** embedded. The binary only carries a
build-machine absolute path to a `queries/` directory, so the published artifact (and any
binary copied to another machine or run after its build/cache directory is removed) cannot
initialize ScopeMux and silently falls back to the generic provider.

Make the provider actually portable: embed the queries (or ship and resolve them relative
to the executable), remove the build-path dependency, and add a clean-host smoke test.

## Finding (2026-11-05)

- The C code and grammar archives are linked statically:
  `crates/opencode-scopemux/build.rs:237-247` (`parser_core` + `tree-sitter-*` static
  libraries) and `:190-211` for the prebuilt bundle path. That part is portable.
- The Tree-sitter query files are a runtime file dependency:
  - `crates/opencode-scopemux/build.rs:142-160` copies `queries/` into `OUT_DIR` and bakes
    the absolute `OUT_DIR/queries` path into the binary via `SCOPEMUX_QUERIES_DIR`.
  - `crates/opencode-scopemux/build.rs:227-232` does the same with the prebuilt bundle's
    absolute cache path (`~/.cache/opencode-modded-rust/native/.../queries`).
  - `crates/opencode-scopemux/src/lib.rs:420-425` sets `SCMU_QUERIES_DIR` from that
    compile-time path at runtime.
- `scopemux-core` requires a real directory on disk: it reads
  `$SCMU_QUERIES_DIR/<lang>/*.scm` and otherwise tries only cwd-relative `queries`,
  `../queries`, `../../queries`, `../../../queries`
  (`scopemux-core/core/src/parser/ts_init.c:74-104`, `:231-260`).
- The published CLI tarball contains only the binary:
  `.github/workflows/native-artifacts.yml:86-98` copies
  `target/release/opencode-modded-rust` and nothing else. `ort-install` installs only
  `.../ort/bin/<rev>/opencode-modded-rust`.
- Observed: `strings target/debug/opencode-modded-rust` contains
  `/home/matrillo/.cache/opencode-modded-rust/native/linux-x86_64-2a642c10534f/queries`.
  That is machine- and build-specific.
- Consequence on a binary-only host (`INFRA-007` model): the baked path and the cwd
  fallbacks are absent, `parser_set_language` fails with "queries directory does not
  exist", `project_parse_all_files` fails, the provider returns an error, and the session
  uses generic retrieval. ScopeMux is compiled in but not usable.

## Why This Matters

- `INFRA-007` deliberately makes artifact-only hosts the norm and removes product
  checkouts. On exactly those hosts, the default binary's ScopeMux is dead.
- Copying a developer-built binary to another machine, or cleaning `target/`/the native
  cache, has the same effect, so the failure is not limited to CI artifacts.
- The fallback is silent apart from logs; the user cannot tell ScopeMux is unavailable.
- A single self-contained binary is the stated portability goal; depending on an external
  query tree undermines it.

## Scope

- Choose and implement the portability mechanism:
  - **Preferred: embed the queries in the binary** (for example a `build.rs`-generated
    module using `include_str!`, or the `include_dir` crate) and materialize them on first
    use into a stable per-version directory such as
    `dirs::data_local_dir()/opencode/scopemux/queries/<pin-or-hash>/`; then set
    `SCMU_QUERIES_DIR` to that directory.
  - Acceptable alternative: ship `queries/` inside the CLI tarball, install it next to the
    binary, and resolve it from `std::env::current_exe()`.
  - Keep the explicit `SCMU_QUERIES_DIR` override working for developers and tests.
- Remove the build-machine absolute path from the release artifact. Source builds and
  prebuilt-bundle builds must both work.
- Make a missing query tree an explicit, non-silent condition: record
  `provider_notes = ["scopemux_queries_missing"]` (or equivalent) and log it, while still
  falling back to generic retrieval.
- Add a CI/release smoke test that runs the built binary from a clean temporary `HOME` and
  cwd with no repo checkout and no native cache, and proves ScopeMux initializes.
- Update `docs/`/`wiki/` and the packaging scripts to reflect the chosen distribution
  layout.

## Acceptance Criteria

- A published `ort` binary starts ScopeMux on a host with no product checkout, no
  `target/`, no native cache, and a cwd without `queries/`.
- The release binary contains no build-machine absolute query path; a test or release
  check fails if one is baked in.
- Source builds and `SCOPEMUX_PREBUILT_DIR` builds both resolve queries correctly.
- `SCMU_QUERIES_DIR` still overrides resolution.
- If queries cannot be materialized, the runtime reports an explicit fallback reason and
  continues with the generic provider; the session does not fail.
- The CI artifact remains a single self-contained binary (preferred), or the shipped
  `queries/` layout is documented and installed atomically by `ort-install`.
- `cargo test -p opencode-scopemux --features native` and the clean-host smoke test pass.

## Recommended Verification

```bash
# Build as the release pipeline does, then run from a clean environment.
cargo build --release -p opencode-cli
clean_home="$(mktemp -d)"
clean_cwd="$(mktemp -d)"
( cd "$clean_cwd" && HOME="$clean_home" XDG_DATA_HOME="$clean_home/.local/share" \
    /path/to/opencode-modded-rust version )
# Then exercise retrieval and confirm the provider logs `provider=scopemux`
# (scripts/scopemux-qa-check.sh) rather than `provider=generic`.
```

- `strings target/release/opencode-modded-rust | rg '/home/|/Users/|target/.*/queries'`
  must not show a build-machine query path.
- Delete the extracted query cache and confirm it is re-materialized from the binary, not
  read from the build tree.
- Run from a directory that does contain `queries/` and confirm the embedded/cache path is
  still used (no accidental cwd dependence).

## Design Notes

- Embedding keeps the single-binary promise and avoids per-platform tarball layout changes.
  Materializing to a content-addressed cache directory is cheap (small `.scm` files) and
  must be atomic (temp dir + rename) and version-keyed by the scopemux pin so upgrades do
  not mix query sets.
- Use the existing platform path helpers (`dirs::data_local_dir()` / `dirs::cache_dir()`)
  rather than a hardcoded path.
- Do not change the `scopemux-core` C API; this is a packaging/resolution fix on the
  product side.
- Keep the prebuilt bundle's `queries/` for local prebuilt builds; the release binary must
  not depend on the cache remaining present.
- If the alternative (ship `queries/` next to the binary) is chosen, `ort-install` must
  install both atomically and keep the `<rev>` directory self-contained.

## Non-Goals

- Changing ScopeMux activation (still the local-Qwen gate).
- Re-enabling exact LSP facts by default (`SCOPE-014` retired them).
- Reworking `scopemux-core` query semantics or adding grammars.
- The remaining ScopeMux program in `H-018`; this card is independent and not part of that
  handoff.
- Cross-compiling macOS from Linux.

## Related Items

- `INFRA-006` Build the native provider once and publish platform bundles.
- `INFRA-007` Consume the published ort CLI binary instead of per-machine clones.
- `SCOPE-002` Integrate scopemux-core behind the retrieval-provider boundary.
- `SCOPE-005` Qwen-only activation (the current activation gate).
- `SCOPE-014` exact-fact evaluation (opt-in `OPENCODE_SCOPEMUX_EXACT_FACTS`).
- `H-018` remaining ScopeMux program handoff (independent; do not fold this card into it).
- `wiki/workspace-intelligence.md`, `invariants/retrieval.md`

## Implementation (2026-11-05)

Branch `feature/INFRA-008-embed-scopemux-queries` (dedicated worktree
`$HOME/worktrees/opencode-modded-rust/infra-008-portable-queries`). PR:
https://github.com/cchris-p/opencode-modded-rust/pull/160 (base `development`, awaiting review).

- `build.rs` stages the query set into `OUT_DIR` and generates
  `scopemux_queries.rs` with `include_bytes!` entries plus a content hash
  (`SCOPEMUX_QUERIES_KEY`). Works for both source and `SCOPEMUX_PREBUILT_DIR`
  builds; the `SCOPEMUX_SKIP_NATIVE_BUILD` check path gets an empty stub.
- `ensure_queries_dir()` materializes the embedded set atomically into
  `dirs::cache_dir()/opencode/scopemux/queries/<content-key>/` on first use and
  sets `SCMU_QUERIES_DIR`. An explicit `SCMU_QUERIES_DIR` directory wins; an
  invalid override falls back to the embedded set.
- `retrieve` now returns `Unavailable("scopemux_queries_missing: ...")` when no
  query directory is usable instead of depending on an absolute build path.
- New `debug scopemux-queries` CLI command prints the resolved directory.
- Release workflow smoke test runs the binary from a clean `HOME`/cwd and fails
  if a build/cache `queries/` path is embedded.
- `AGENTS.md`, `docs/opencode-cli.md`, and `wiki/workspace-intelligence.md`
  updated to describe the portability contract.

Verification (dedicated target directory; the shared worktree target was
polluted by concurrent builds from another worktree and is not a reliable
verification environment):

- `cargo fmt --all --check`: pass.
- `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo check -p opencode-scopemux --features native`: pass.
- `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo check -p opencode-cli`: pass.
- `cargo clippy -p opencode-scopemux -p opencode-cli --all-targets`: pass
  (pre-existing warnings only).
- `cargo test -p opencode-scopemux --features native embedded_queries`: 2/2 pass.
- Full native suite: 18 passed, 1 known pre-existing failure
  (`native_delta_slice_returns_projected_entries`), 1 known pre-existing SIGSEGV
  skipped (`native_provider_reports_plan_reconciliation_evidence`). Both are the
  failures `SCOPE-013` documented on base `development`.
- Release binary clean-host smoke:
  `HOME=<tmp> XDG_CACHE_HOME=<tmp> <binary> debug scopemux-queries` resolved the
  cache path and `rust/docstrings.scm`; the release binary contains no
  build-machine `queries/` path.

Not covered here: the prebuilt C archive still embeds its CI `__FILE__` source
paths in log strings. Those are not query paths and are pre-existing.

Keep in `doing` until PR #160 merges, then move to `qa` for post-merge QA.

## Post-Merge QA (2026-11-08)

- PR #160 merged into `development` (`c9ac28d`). CI native job: `build
  (ubuntu-22.04)` pass; `build (macos-14)` cancelled (pre-existing runner issue,
  not a code failure; no test workflow exists yet, see `INFRA-009`).
- Rebased the long-lived branch onto current `development` (the crate had been
  split into `engine`/`provider`/`supervisor`; embedding moved to a new
  `crates/opencode-scopemux/src/queries.rs`).
- Post-merge QA on merged `development` found a regression: `WorkspaceEngine::open`
  still read the removed compile-time `SCOPEMUX_QUERIES_DIR`, so engine-level
  contexts parsed zero nodes (2 failing engine tests). Fixed in PR #171
  (`a6d4e2b`, merged as `e113708`): the engine now calls
  `ensure_queries_dir()` before creating a context.

Verification on merged `development` (`e113708`), prebuilt native bundle:

```text
cargo fmt --all --check                                            PASS
SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo check -p opencode-scopemux --features native   PASS
SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo check -p opencode-cli          PASS
cargo clippy -p opencode-scopemux -p opencode-cli --all-targets   PASS (pre-existing warnings only)
cargo test -p opencode-scopemux --features native                 40 passed; 0 failed
cargo build -p opencode-cli --bin opencode-modded-rust            PASS
clean-host: HOME=<tmp> XDG_CACHE_HOME=<tmp> <binary> debug scopemux-queries
  -> <tmp>/cache/opencode/scopemux/queries/73aa2ff86776cc85 (rust/docstrings.scm present)
strings <binary> | grep <build/cache queries path>                -> no match
```

The two `SCOPE-013` failures noted above no longer reproduce on this base.


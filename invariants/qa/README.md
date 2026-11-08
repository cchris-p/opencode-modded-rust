# QA Invariants

Durable verification rules for this repo. Keep this list small and
evidence-backed.

## Focused Test Commands

- Rust behavior changes run the narrowest crate tests first, for example:
  `cargo test -p opencode-agent`.
- Compile/API changes run `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo check -p opencode-cli`.
  The skip is valid for checking; it does not produce a linkable binary.

## Native ScopeMux Build Requirement

- Building `opencode-modded-rust` needs the native ScopeMux provider. Either:
  - use the cached prebuilt bundle:
    `SCOPEMUX_PREBUILT_DIR="$(scripts/ensure-native-artifacts.sh --check | tail -n 1)"`,
    or
  - provide a `scopemux-core` source tree and let `build.rs` compile it.
- `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo build` fails at link time with undefined
  `project_context_*` symbols. Do not report that as a product failure.
- `third_party/scopemux-core` is absent in the normal dev checkout; a plain
  `cargo build` fails its build script. Use the prebuilt bundle or
  `scripts/fetch-scopemux-core.sh`.

## Runtime Smoke For Permission Changes

- Verify both paths when a change touches permissions:
  - plain CLI: `opencode-modded-rust run --agent <agent> "<probe>"`
  - server/TUI: `serve` + `POST /session/{id}/prompt`, or the TUI.
- A permission fix must show the denied operation did **not** happen (no file
  created, no command output) and that an allowed operation still works.

## Board Completion

- Record the commands, results, and runtime evidence on the board card.
- A card may move to `done` only with recorded QA evidence or explicit owner
  direction.

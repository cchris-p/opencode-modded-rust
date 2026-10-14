---
id: "START-033"
title: "Decouple the product command identity from opencode"
priority: "P2"
type: "refactor"
area: "START"
spec: "invariants/reference-boundary.md"
status: "doing"
predecessors: ""
created: "2026-10-14"
updated: "2026-10-14"
---

# Decouple the product command identity from opencode

## Summary

Decouple the product's user-facing *command identity* from bare `opencode`. The Rust product binary
is currently compiled as `opencode` and its clap command is `name = "opencode"`, so `ort --help`
prints `Usage: opencode tui ...` and is indistinguishable from vanilla OpenCode. This card makes the
product identify itself as the canonical product, `opencode-modded-rust`, at the command/binary/help
layer, so the modded product can never be confused with vanilla `opencode`.

The canonical product name is **`opencode-modded-rust`** (not `scopemux-code`; see
`START-034`). This card is **layer 1** (interface identity). **Layer 2** (config/state/env paths)
is planned here but explicitly deferred. **Layer 3** (crate/package names) is a non-goal.

## Why this exists

- `opencode` on `PATH` is the pinned **vanilla** upstream build (`~/standards/opencode-config:548`)
  and must not change.
- The Rust product is reached via `ort`/`omr`/`opencode-rust*`, but its binary and clap command are
  `opencode` (`crates/opencode-cli/Cargo.toml:15-17`, `crates/opencode-cli/src/main.rs:34`), so help,
  usage, hints, and the server banner all read `opencode` or `OpenCode`.
- `START-003` declared `scopemux-code` canonical; that is now superseded — this card and `START-034`
  make `opencode-modded-rust` the single canonical product name and identity.
- This unblocks `CLI-013` (launcher subcommand reachability), which needs a settled command name.

## Layer 1 — interface identity (IN SCOPE)

### Product repo (`opencode-modded-rust`)

- **Binary name:** `crates/opencode-cli/Cargo.toml:15-17` `[[bin]] name = "opencode"`.
- **clap identity:** `crates/opencode-cli/src/main.rs:34` `#[command(name = "opencode")]`;
  `about = "OpenCode - Open source AI coding agent"` (`:35`).
- **Subcommand/about strings:** e.g. `:72` "Attach TUI to a running OpenCode server"; audit all
  `#[command(about = ...)]` and help text for `OpenCode`/`opencode`.
- **Server banner:** `crates/opencode-cli/src/main.rs:2407` `"Starting OpenCode {mode} server on {addr}"`.
- **Resume hint:** `resume_launcher_from` (`:1028`, returns `"ort"`/`"opencode"`) and
  `resume_hint_lines` (`:1036`).
- **Reattach hint:** `reattach_hint_lines` (`:1283`, prints `opencode attach <url>` / `ort --attach`).
- **Tests asserting these strings:** `:7526`, `:7532`, `:7543`, `:7555`, `:7567`, `:7576` (update in
  lockstep).
- **TUI runtime branding:** `crates/opencode-tui/src/branding.rs` — `APP_NAME = "RustingOpenCode"`,
  `APP_SHORT_NAME = "ROCode"`; consumers `terminal.rs`, `components/home.rs`, `components/sidebar.rs`;
  dead `components/logo.rs`. Absorbs archived `FEAT-032`.
- **Any other user-visible `OpenCode`/`scopemux-code` string in `crates/`** (audit with ripgrep).

### Standards launcher (`~/standards/opencode-config`, in scope, cross-repo)

- `_opencode_rust_exec` (`:950`) resolves `$repo/target/debug/opencode` — update to the new binary
  name.
- Launchers/aliases: `opencode-rust-tui` (`:1307`), `ort` (`:1327`), `omr` (`:1331`),
  `opencode-rust-run` (`:1536`), `opencode-rust-root` (`:1540`), `ort-*` helpers, `ort-kill` patterns,
  `ort-doctor`, `ort-reload`.
- `OPENCODE_*` env names used by the launcher (decision: rename to `OMR_*`/product-scoped or keep —
  see Layer 2).
- Ensure `opencode` (vanilla) and its compat normalization are untouched.

### Docs

- Product-facing command references in `README.md`, `USER_GUIDE.md`, `docs/`, `wiki/` — align the
  command name (name-string swap is `START-034`; this card only fixes command/usage references).

## Layer 2 — config/state/env paths (PLANNED, DEFERRED)

Documented here only to record the deferred scope; **do not implement in this card**. These are
future-card questions, not open decisions for `START-033`.

- **Config dir:** `~/.config/opencode/*` (shared with vanilla; standards-managed config and
  `AGENTS.md` live here). A future card may introduce a product-scoped config dir (e.g.
  `~/.config/opencode-modded-rust/`) or keep sharing `~/.config/opencode/`.
- **Data dir / DB:** `dirs::data_local_dir()/opencode/opencode.db`
  (`crates/opencode-storage/src/database.rs:174-178`); `dirs::data_local_dir()/opencode/traces/server.log`.
  A future card may rename the `opencode` data subdir or keep it for continuity.
- **Env-var names:** `OPENCODE_SERVER_LOG`, `OPENCODE_RUN_TIMEOUT_MS`, `OPENCODE_STREAM_BUDGET_MS`,
  and the launcher pass-through vars `OPENCODE_CONFIG_CONTENT`, `OPENCODE_CONFIG_DIR`,
  `OPENCODE_TUI_CONFIG`, plus `OPENCODE_MODEL_*` presets. A future card may product-scope these.
- **Migration/compat:** read-old/write-new, one-time migration, or no migration (accept fresh state).
- **Risks:** state fragmentation, breaking standards-managed config, macOS vs vanilla DB path
  mismatch, and hidden coupling in launchers/plugins/tests.

## Scope (this card)

- Layer 1 only: binary + clap identity, help/usage/banner, resume/reattach hints, TUI branding
  constants, and the standards launcher's binary/alias wiring.
- Update docs' *command* references; the `scopemux-code` name-string sweep is `START-034`.

## Non-goals

- Layer 2 config/state/env path changes (planned above, deferred).
- Layer 3 crate/package/repo-name renames (`opencode-cli`, `opencode-server`, `opencode-modded-rust`
  repo) — retained as implementation/repo identifiers.
- Repointing `opencode` at the Rust build (forbidden by global rule).
- Launcher subcommand passthrough (`CLI-013`) and the `scopemux-code` doc scrub (`START-034`).

## Resolved Decisions (locked 2026-10-14)

- **Product name / binary / clap command:** `opencode-modded-rust`.
- **Primary short launcher:** `omr` (general entry point; `START-033` only points it at the new
  binary; subcommand dispatch is `CLI-013`).
- **TUI alias:** `ort` retained as the TUI launcher.
- **`APP_NAME`:** `opencode-modded-rust`; **`APP_SHORT_NAME`:** `omr`.
- **Launcher env vars:** keep `OPENCODE_*` names in Layer 1; renaming is deferred with Layer 2.

## Done when

- No user-facing help/usage/banner/hint surface presents the product as `opencode`/`OpenCode`.
- Binary and clap command use `opencode-modded-rust` (or the decided command name).
- `ort`/`omr` launch the product; `opencode` remains vanilla and unchanged.
- TUI branding renders the product name (no `RustingOpenCode`/`ROCode`).
- `cargo check -p opencode-cli`/`-p opencode-tui` and `cargo test -p opencode-cli` pass; launcher
  smoke-tested.

## Recommended verification

- `cargo fmt --all`; `cargo check -p opencode-cli -p opencode-tui`; `cargo test -p opencode-cli`.
- `ort-build`; run `ort --help` and confirm it no longer reads `Usage: opencode ...`.
- Run `opencode --version`; confirm vanilla is unchanged.
- Trigger a resume/reattach hint and confirm the product command name is printed.

## Related Items

- `START-003` Standardize product name — declared `scopemux-code`; superseded by this card and
  `START-034`.
- `START-034` Scrub scopemux-code product-name references — the name-string sweep that follows this.
- `CLI-013` Reach the Rust CLI subcommands through the launcher — depends on the settled command name.
- `FEAT-032` Rebrand TUI branding constants — archived draft absorbed here.
- `INFRA-004` Align checkout/setup paths — adjacent launcher/path work.
- `invariants/reference-boundary.md` — canonical-product-name rule (to be corrected by `START-034`).

## Notes

- Created 2026-10-14; reframed 2026-10-14: canonical name corrected to `opencode-modded-rust`, layers
  1/2 inventory expanded, and the `scopemux-code` scrub split out to `START-034`.

## Implementation Notes

- 2026-10-14: Layer 1 implemented on `feature/START-033-command-identity`.
  - `crates/opencode-cli/Cargo.toml` `[[bin]] name` and clap `#[command(name)]`/`about` renamed to
    `opencode-modded-rust`; subcommand about strings updated.
  - Server banner, `version`, interactive banner, and task/auth/mcp/github hints renamed.
  - `resume_launcher_from` prints `opencode-modded-rust` when the Rust launcher env is absent; `ort`
    retained; tests updated.
  - `branding.rs` `APP_NAME=opencode-modded-rust`, `APP_SHORT_NAME=omr`; dead `components/logo.rs`
    (`RO CODE`) removed.
  - Server web page title/h1, `init` command description/heading, copilot reauth message, OpenAPI
    title updated.
  - Command references updated across `README.md`, `USER_GUIDE.md`, `docs/`, `wiki/`, `scripts/`,
    `AGENTS.md`.
  - Kept as compatibility/implementation surfaces: `OPENCODE_*` env names, `opencode.local` mdns
    domain, `opencode.server.starting` event id, provider ids, and the model system-prompt identity.
  - Layer 2 (config/state/env path renames) deferred.
- Verification: `cargo fmt --all`; `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo check -p opencode-cli
  -p opencode-tui -p opencode-server`; `cargo test -p opencode-cli` (21 passed); `ort-build`;
  `omr --help`/`ort --help` now print `Usage: opencode-modded-rust …`; vanilla `opencode --version`
  unchanged.
- PR: [#142](https://github.com/cchris-p/opencode-modded-rust/pull/142) (`development`, open).
- Standards launcher `_opencode_rust_exec`/`ort-doctor` binary path updated after this lands;
  commit deferred to preserve machine consistency.

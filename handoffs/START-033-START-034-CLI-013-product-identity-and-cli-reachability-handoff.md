---
id: "H-015"
title: "START-033/START-034/CLI-013 Product identity and CLI reachability - Handoff"
status: "in_progress"
created: "2026-10-14"
updated: "2026-10-14"
owner: ""
target: "development"
blocked_reason: ""
needs_human: ""
items: ["START-033", "START-034", "CLI-013"]
---

# START-033/START-034/CLI-013 Product Identity and CLI Reachability - Handoff

## Objective

Make the Rust product identify itself as `opencode-modded-rust` (never bare `opencode`/`OpenCode`),
purge the retired `scopemux-code` product name, and make the product's headless CLI subcommands
reachable through the `omr` launcher. End state: a user can run `omr serve`, `omr task new -f …`,
`omr session list`, `omr attach <url>` from any workspace, `omr --help` shows the full command
surface, and `opencode` remains the untouched vanilla build.

## Covered items

- `START-033` Decouple the product command identity from opencode (layer 1).
- `START-034` Scrub scopemux-code product-name references.
- `CLI-013` Reach the Rust CLI subcommands through the launcher.

## Dependencies and order

**Strict order: START-033 → START-034 → CLI-013** (explicit, from card `predecessors` chains).

- `START-033` sets the canonical command/binary name; `START-034` and `CLI-013` both depend on it
  being locked.
- `START-034` (name scrub) runs before `CLI-013` so the launcher/docs changes reference the final
  name and no stale `scopemux-code` text remains when the launcher is rewritten.
- No parallel-safe steps: all three touch the same surfaces (`opencode-modded-rust` identity strings
  and `~/standards/opencode-config`). Execute strictly sequentially.

## Locked decisions (do not re-open)

- Product name / binary / clap command: **`opencode-modded-rust`**.
- Primary short launcher: **`omr`** (general: TUI default + subcommand passthrough after `CLI-013`).
- TUI alias: **`ort`** retained.
- `APP_NAME` = `opencode-modded-rust`; `APP_SHORT_NAME` = `omr`.
- Launcher env vars keep `OPENCODE_*` names in this handoff; product-scoping is deferred (Layer 2).
- **Layer 2** (config/state/env path renames) is explicitly out of scope. Do not implement.
- `START-034` scrubs only the product-name token `scopemux-code`; the **ScopeMux integration**
  (`scopemux-core`, `scopemux-notes`, native provider, `SCOPE-*`, `invariants/integration-scope.md`)
  is preserved.

## Cross-repo coordination

Two repositories are edited:

- **Product** `opencode-modded-rust` (this repo): binary/clap/branding/hints, docs, invariants.
- **Standards** `~/standards/opencode-config`: the launcher (`_opencode_rust_exec` binary path,
  `omr`/`ort`/`opencode-rust-*` wiring, subcommand dispatch).

Product changes flow through feature branches → PRs into `development`. Standards changes commit on
the standards repo's `development` branch per its workflow. Never repoint `opencode` at the Rust
build.

---

## Phase 1 - START-033: command identity (Layer 1)

Deliverable: no user-facing surface presents the product as `opencode`/`OpenCode`.

Steps:

1. Product: rename the binary in `crates/opencode-cli/Cargo.toml` `[[bin]] name = "opencode"` →
   `opencode-modded-rust`.
2. Product: rename the clap command `crates/opencode-cli/src/main.rs:34`
   `#[command(name = "opencode")]` → `opencode-modded-rust`; update `about` (`:35`) and subcommand
   `about` strings (e.g. `:72`).
3. Product: update the server banner `main.rs:2407` (`"Starting OpenCode {mode} server…"`).
4. Product: update resume/reattach hints — `resume_launcher_from` (`:1028`), `resume_hint_lines`
   (`:1036`), `reattach_hint_lines` (`:1283`) — and their tests (`:7526`, `:7532`, `:7543`, `:7555`,
   `:7567`, `:7576`). Keep `ort` as the TUI launcher hint when the Rust launcher env is present;
   otherwise print the product command.
5. Product: TUI branding `crates/opencode-tui/src/branding.rs` — `APP_NAME = "opencode-modded-rust"`,
   `APP_SHORT_NAME = "omr"`; confirm `terminal.rs`/`home.rs`/`sidebar.rs` render without layout
   regressions; remove/neutralize dead `components/logo.rs` `RO CODE` art (absorbing `FEAT-032`).
6. Product: audit remaining user-visible `OpenCode`/`opencode` strings in `crates/` (ripgrep) and fix.
7. Standards: update `~/standards/opencode-config` — `_opencode_rust_exec` binary path
   `$repo/target/debug/opencode` → `…/opencode-modded-rust`; keep `ort` (TUI) and `omr` pointing at
   the launcher; leave `opencode` (vanilla, `:548`) untouched. Keep `OPENCODE_*` env names.
8. Docs: update product-facing *command* references (README/USER_GUIDE/docs/wiki).

Verification gate (must pass before Phase 2):

- `cargo fmt --all`; `cargo check -p opencode-cli -p opencode-tui`; `cargo test -p opencode-cli`.
- `ort-build` succeeds; `omr --help` / `ort --help` never print `Usage: opencode ...`.
- `opencode --version` still reports vanilla and is unchanged.
- Trigger a resume/reattach hint; it prints the product command/`ort`, not `opencode` (except the
  vanilla branch).

---

## Phase 2 - START-034: scrub scopemux-code product-name references

Deliverable: `opencode-modded-rust` is the only canonical product name; no current/authoritative
surface uses `scopemux-code`.

Steps:

1. Rewrite current/authoritative docs: `invariants/reference-boundary.md` (canonical-name line),
   `invariants/README.md`, `invariants/documentation-boundary.md`, `README.md`, `USER_GUIDE.md`,
   `docs/README.md`, `docs/opencode-tui.md`, and `wiki/` (`product-boundary.md`, `vision.md`,
   `v1.md`, `v1-runtime-loop.md`, `reference-strategy.md`, `README.md`,
   `agent-evaluation-strategy.md`, `scopemux-map-integration.md`, `laya-integration.md`,
   `jev-ultrafast-integration.md`).
2. In each `wiki/*-integration.md` hit, scrub only where the token names the *product*; keep where it
   names the *ScopeMux integration*.
3. Sweep any `scopemux-code` string in `crates/` and `scripts/`.
4. Supersede `START-003` in the plan; mark `scopemux-code` a retired/legacy name in
   `invariants/reference-boundary.md`.
5. Historical surfaces (`boards/done/**`, `boards/archive/**`, `docs/transcripts/**`): add a
   superseding note referencing `START-034`; do not rewrite history.
6. Update the two remaining refinement/`todo` cards and this handoff if they still name the product
   `scopemux-code`.

Verification gate (must pass before Phase 3):

- `rg -i "scopemux-code" . --glob '!target/**' --glob '!.git/**'` returns only intentional
  historical annotations and ScopeMux-integration references.
- `rg -i "scopemux-code" crates/ scripts/` returns nothing.
- `invariants/reference-boundary.md` states `opencode-modded-rust` canonical.

---

## Phase 3 - CLI-013: launcher subcommand reachability

Deliverable: `omr <subcommand>` runs the product's headless CLI; bare `omr`/TUI flags launch the TUI.

Steps (standards `~/standards/opencode-config`):

1. Give `omr` subcommand dispatch: if the first non-flag token is in an explicit allowlist of the
   product's top-level subcommands (`task`, `serve`, `session`, `attach`, `run`, `import`, `export`,
   `mcp`, `auth`, `models`, `pr`, `debug`, `stats`, `db`, `acp`, `web`) → `_opencode_rust_exec "$@"`;
   otherwise launch the TUI as today.
2. Keep `ort` TUI-only for back-compat.
3. `omr --help` (and `omr` with no args) must surface the product's top-level help / full command
   list rather than only `tui`.
4. Update `wiki/cli-surface.md` and any launcher docs so the headless path is documented.

Verification gate:

- `omr serve`, `omr task new "hi"`, `omr task view`, `omr session list`, `omr attach <url>` reach the
  product.
- `omr`, `omr --model <m>`, `omr --continue`, `ort` still launch the TUI.
- `omr --help` lists the product commands.
- `opencode` remains vanilla and unchanged.

---

## PR / merge plan

- Product repo PRs target `development`, in dependency order:
  - PR-1: `feature/START-033-command-identity` (Phase 1 product changes).
  - PR-2: `feature/START-034-scrub-scopemux-code` (Phase 2), branched after PR-1 merges or stacked
    on it.
- Standards repo: Phase 1 launcher path/alias edit and Phase 3 dispatch edit commit on the standards
  `development` branch (or one focused PR), after the corresponding product PR lands so the binary
  name exists.
- Keep each PR open until the workflow's completion step; merge the full set into `development` in
  order (START-033 → START-034 → CLI-013), then move all three cards to `qa` for post-merge QA
  (a merge alone does not move a card to `done`).
- `opencode` (vanilla) must never be modified or repointed.

## Completion criteria

- All three cards are in `qa` with merged PR/commit references recorded.
- `omr`/`ort` run the `opencode-modded-rust` product; `omr <subcommand>` reaches the CLI surface.
- No current surface presents the product as `opencode` or `scopemux-code`; `opencode` is vanilla.
- Layer 2 remains explicitly deferred and untouched.

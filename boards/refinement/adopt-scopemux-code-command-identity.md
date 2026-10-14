---
id: "START-033"
title: "Adopt the scopemux-code command identity and decouple the product binary from opencode"
priority: "P2"
type: "refactor"
area: "START"
spec: "invariants/reference-boundary.md"
status: "refinement"
predecessors: ""
created: "2026-10-14"
updated: "2026-10-14"
---

# Adopt the scopemux-code command identity and decouple the product binary from opencode

## Summary

Decouple the product's user-facing *command identity* from `opencode`. Today the Rust product
binary is literally named `opencode` and its clap header is `name = "opencode"`, so `ort --help`
prints `Usage: opencode tui ...` and is indistinguishable from vanilla OpenCode. This card makes the
product present itself under the canonical product name (`scopemux-code`, per `START-003`) at the
command/binary/help/hint layer, so a user can never confuse the modded product with vanilla
`opencode`.

This is **layer 1** (interface identity). **Layer 2** (config/state directory rename) is explicitly
deferred to a separate decision; **layer 3** (crate/package names) is a non-goal.

## Why this exists

- `opencode` on `PATH` is the pinned **vanilla** upstream build (`~/standards/opencode-config:548`),
  and that must not change.
- The Rust product is reached via `ort`/`omr`/`opencode-rust*`, but its binary is compiled as
  `opencode` (`crates/opencode-cli/Cargo.toml:15-17`) and its clap command is `name = "opencode"`
  (`crates/opencode-cli/src/main.rs:34`). The result is direct user confusion: `ort --help` looks
  like vanilla OpenCode, and the product's own name never appears.
- `START-003` ("Standardize product name") already declared `scopemux-code` canonical and marked
  `opencode`/`opencode-*` as compatibility/implementation identifiers only
  (`invariants/reference-boundary.md`). `FEAT-032` tried to apply it to TUI branding constants but was
  archived as never implementation-ready. This card applies the identity at the command layer.
- This is the prerequisite for the follow-up CLI reachability work (`CLI-013`): the launcher cannot
  be cleanly renamed/dispatched until the binary/command identity is decided.

## Current State (evidence)

- Binary name: `crates/opencode-cli/Cargo.toml` — `[[bin]] name = "opencode"`.
- clap command name: `crates/opencode-cli/src/main.rs:34` — `#[command(name = "opencode")]`.
- Help/usage surfaces therefore render `opencode ...` everywhere.
- Resume/reattach hints render `opencode attach <url>` / `opencode --continue`
  (`resume_launcher_from`, `resume_hint_lines`, `reattach_hint_lines` in `crates/opencode-cli/src/main.rs`).
- Launcher resolves the binary path by name: `~/standards/opencode-config:950`
  (`_opencode_rust_exec`: `$repo/target/debug/opencode`); `ort`/`omr` at `:1327`/`:1331`.
- TUI branding constants still carry the retired name (`crates/opencode-tui/src/branding.rs`
  `APP_NAME`/`APP_SHORT_NAME`; see archived `FEAT-032`).
- Docs already use `scopemux-code` as product name (`README.md`, `wiki/product-boundary.md`,
  `invariants/reference-boundary.md`).

## Scope (layer 1)

- Decide the product command/binary name (`scopemux-code` recommended) and, if desired, a short
  launcher alias for daily typing.
- Rename the product binary (`[[bin]] name`) and the clap command name away from `opencode`, so all
  help/usage/error text shows the product name.
- Update resume/reattach hints and any other usage strings that print `opencode <subcommand>` to the
  new command name (with launcher awareness preserved for `ort`).
- Update the standards launcher (`~/standards/opencode-config`): `_opencode_rust_exec` binary path,
  and the `ort`/`omr` (and any new alias) wiring. `opencode` stays vanilla.
- Apply the product name to TUI branding constants (`branding.rs`) so the rendered name matches,
  resolving the archived `FEAT-032` scope (if the short-name decision lands here).
- Update product-facing docs/AGENTS status that describe the command identity.

## Non-goals

- **Layer 2 (config/state paths):** do NOT rename `~/.config/opencode` or
  `dirs::data_local_dir()/opencode/opencode.db` in this card. Those are shared with vanilla and the
  standards config; a path rename is a separate, deliberate decision.
- **Layer 3:** do NOT rename crates/packages (`opencode-cli`, `opencode-server`, …) or the repository
  name (`opencode-modded-rust`). They are implementation/repo identifiers per `START-003`.
- Repointing `opencode` at the Rust build is forbidden (global rule).
- CLI subcommand reachability / launcher passthrough is `CLI-013` (follow-up), not this card.

## Open Decisions

- Exact command name: `scopemux-code` (canonical) vs a shorter primary command. The command is typed
  daily, so length matters.
- Short launcher alias: keep the existing `omr` (and `ort`), or introduce a new shorter one derived
  from the product name.
- Whether the binary name and the primary typed command should be identical, or the binary is
  `scopemux-code` while a short alias is the primary entry point.
- Whether `FEAT-032`'s `APP_SHORT_NAME` decision is settled here or split out.

## Done when

- No user-facing help/usage/hint surface presents the product as `opencode`.
- The Rust binary and clap command use the decided product name.
- `ort`/`omr` (or the agreed alias) still launch the product; `opencode` remains vanilla.
- TUI branding renders the product name (if in scope).
- `cargo check -p opencode-cli` and `cargo test -p opencode-cli` pass; launcher smoke-tested.

## Recommended verification

- `cargo fmt --all`; `cargo check -p opencode-cli`; `cargo test -p opencode-cli`.
- Build (`ort-build`) and run `ort --help`; confirm it no longer reads `Usage: opencode ...`.
- Run `opencode --version`; confirm it is still vanilla and unchanged.
- Confirm resume/reattach hints print the new command name.

## Related Items

- `START-003` Standardize product name — established `scopemux-code` canonically; this card applies it
  to the command/binary identity.
- `FEAT-032` Rebrand TUI branding constants to scopemux-code — archived draft; naming + constant scope
  is absorbed here.
- `CLI-013` Reach the Rust CLI subcommands through the launcher — the follow-up this card unblocks.
- `INFRA-004` Align checkout/setup paths with the standards-managed submodule — adjacent launcher/path
  work.
- `invariants/reference-boundary.md` — canonical product-name rule.

## Notes

- Created 2026-10-14 from a user request to stop confusing the modded product with vanilla `opencode`.
- Layering (interface / config-state / crate names) was agreed as the framing; only layer 1 is in
  scope now, layer 2 deferred.

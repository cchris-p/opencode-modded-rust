---
id: "FEAT-032"
title: "Rebrand TUI branding constants to scopemux-code"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "todo"
created: "2026-09-21"
attention: "Open question: exact logo art and short-name value not finalized"
---

# Rebrand TUI branding constants to scopemux-code

## Summary

Replace the lingering `RustingOpenCode` / `ROCode` branding constants in the TUI with the canonical product name `scopemux-code` so every rendered surface (terminal title, home screen, sidebar) shows the current brand instead of the retired one.

This is a draft card. The constant replacement is in scope now; the graphical logo art and the exact short name are still open and must be decided before this is implementation-ready.

## Why this exists

`START-003` ("Standardize product name") declared `scopemux-code` as the canonical product name, and `START-013` ("Remove RO CODE logo and Rusted tagline") removed the rendered RO CODE mark. However the branding constants were never updated, so the product still presents the old name at runtime.

Current state:

- `crates/opencode-tui/src/branding.rs:1` — `APP_NAME = "RustingOpenCode"`.
- `crates/opencode-tui/src/branding.rs:2` — `APP_SHORT_NAME = "ROCode"`.
- `crates/opencode-tui/src/branding.rs:3` — `APP_VERSION_DATE = "2026.02.23"` (a date, not a name; leave unless it needs refreshing).

These constants are live and user-visible:

- `crates/opencode-tui/src/terminal.rs:3,12,17` — terminal/window title uses `APP_NAME` and `APP_SHORT_NAME`.
- `crates/opencode-tui/src/components/home.rs:11,216` — home surface renders `APP_SHORT_NAME APP_VERSION_DATE`.
- `crates/opencode-tui/src/components/sidebar.rs:13,584` — sidebar header renders `APP_NAME (APP_SHORT_NAME)`.

The old ASCII logo component also still exists as dead code: `crates/opencode-tui/src/components/logo.rs` (the `LOGO_LEFT` / `LOGO_RIGHT` `RO CODE` art) is still declared and re-exported via `crates/opencode-tui/src/components/mod.rs:5,39`, even though `START-013` removed its render path on the home screen.

## Scope

- Update `crates/opencode-tui/src/branding.rs` so `APP_NAME` and `APP_SHORT_NAME` reflect `scopemux-code`.
- Keep all existing consumers working without layout regressions:
  - terminal title (`terminal.rs`)
  - home surface (`home.rs:216`)
  - sidebar header (`sidebar.rs:584`)
- Remove or neutralize the retired `RO CODE` ASCII art in `crates/opencode-tui/src/components/logo.rs` so no stale branding art remains exported (decide delete vs. replace with new art; see Open Questions).
- Update any product-facing docs that still describe `RustingOpenCode` / `ROCode` as current branding.

## Non-goals

- Designing or adding a new graphical logo art block (tracked as an open question; may split into a follow-up card).
- Re-adding a rendered logo to the home screen.
- Changing `APP_VERSION_DATE` semantics or the versioning scheme.
- Renaming crates, binaries, storage paths, or any `opencode-*` implementation identifiers (those are compatibility/implementation names per `START-003`).

## Open Questions

- What is the correct `APP_SHORT_NAME` value — `scopemux-code`, `scopemux`, or something else? The short name is shown in compact surfaces, so its length affects the sidebar/home layout.
- Should `logo.rs` be deleted outright (since it is already unused) or replaced with new `scopemux-code` art in this card?
- Is the exact logo art to be provided by the user, or is sourcing/creating it part of a follow-up design card?

## Done when

- No user-visible TUI surface renders `RustingOpenCode` or `ROCode`.
- `branding.rs` carries the agreed `scopemux-code` name and short name.
- Terminal title, home surface, and sidebar all render the new branding without layout breakage at narrow and wide terminal widths.
- `crates/opencode-tui/src/components/logo.rs` no longer exports retired `RO CODE` art (deleted or replaced per the resolved open question).
- Product-facing docs no longer describe `RustingOpenCode` / `ROCode` as current branding.

## Recommended verification

- Run `cargo check -p opencode-tui` (and `ort-build` if available) after the change.
- Run `ort` and confirm the terminal/window title, home surface, and sidebar all show `scopemux-code` branding.
- Resize the terminal to a narrow width and confirm the sidebar/header branding does not clip or wrap badly.
- `grep -rn "RustingOpenCode\|ROCode\|RO CODE"` across `crates/` and `docs/` and confirm no current, user-facing occurrences remain.
- Confirm `logo.rs` is either gone or no longer exports the retired art, and that removal does not break `components/mod.rs` exports.

## Related Items

- `START-003` Standardize product name — established `scopemux-code` as canonical; this card applies it to runtime branding.
- `START-013` Remove RO CODE logo and Rusted tagline — removed the rendered mark; this card finishes the job for the constants and dead logo code.

## Notes

- This card was created as a draft per the user's request; the scope is intentionally limited to the branding constants pending a decision on the artwork.
- Related completed work lives in `boards/done/standardize-product-name.md` and `boards/done/remove-ro-code-logo-and-rusted-tagline.md`.

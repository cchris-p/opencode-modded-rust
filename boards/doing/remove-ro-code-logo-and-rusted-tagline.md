---
id: "START-013"
title: "Remove RO CODE logo and Rusted tagline"
priority: "P1"
type: "feature"
area: "START"
spec: ""
status: "doing"
created: "2026-08-29"
---

# Remove RO CODE logo and Rusted tagline

## Summary

Remove the `RO CODE` graphical logo and the `A Rusted OpenCode Version` tagline from the TUI and related product-facing surfaces.

## Why this exists

That branding is not wanted in the product and should not appear in the interface.

## Scope

- Identify every place the graphical `RO CODE` mark is rendered.
- Remove the `A Rusted OpenCode Version` tagline from the TUI.
- Clean up any nearby layout or spacing that depended on that branding.
- Update any local docs that still describe the removed tagline.

## Done when

- The TUI no longer shows the `RO CODE` graphical logo.
- The TUI no longer shows the `A Rusted OpenCode Version` text.
- Product-facing documentation no longer describes that branding as current.

## Related items

- `START-003` Standardize product name

## Notes

- This is a branding removal task, not a broader visual redesign.

## Dev Notes

- Removed the home-screen `Logo` render path and the `APP_TAGLINE` constant so the TUI no longer shows the RO CODE mark or Rusted tagline.
- Kept the surrounding home layout centered after removing the branding block.
- Updated `docs/opencode-tui.md` to stop documenting the tagline as current branding.

## Verification

- `cargo check -p opencode-tui`

## Dev Notes

- Removed the home-screen logo render and tagline render path from `crates/opencode-tui/src/components/home.rs`.
- Removed the now-unused `APP_TAGLINE` constant from `crates/opencode-tui/src/branding.rs`.
- Updated `docs/opencode-tui.md` so the branding section no longer documents the removed tagline.
- Verification: `cargo check -p opencode-tui`

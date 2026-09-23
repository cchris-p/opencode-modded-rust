---
id: "FEAT-054"
title: "TUI color scheme consistency across all surfaces"
priority: "P3"
type: "feature"
area: "FEAT"
spec: ""
status: "todo"
created: "2026-09-23"
---

# TUI color scheme consistency across all surfaces

## Summary

Switching the active theme should recolor the entire TUI. Today some surfaces are theme-driven and
others hardcode colors or use dead styling helpers, so a theme change leaves parts of the UI (spinner,
toasts, todos, agent picker, syntax highlighting, some markdown fallbacks) visually out of sync with
the selected preset. Establish one source of truth for color and route the stragglers through it.

## Why this exists

`Theme` already carries a broad set of semantic tokens
(`crates/opencode-tui/src/theme/mod.rs:66-115`) and is applied globally via `AppContext::theme`
(`crates/opencode-tui/src/context/app_context.rs:284-294`). But several render paths predate the
token set or ignore it, so the same semantic concept is drawn with different colors depending on the
component.

## Current behavior and evidence

- Dead styling helper with hardcoded colors: `Styles` (`crates/opencode-tui/src/theme/mod.rs:691-717`)
  defines `muted/success/error/warning` as literal RGB values that duplicate theme tokens. It has zero
  references anywhere in the crate (`grep -rn "Styles" crates/opencode-tui/src`), so it is unused
  duplication that will drift from the theme.
- Spinner ignores the theme: `KnightRiderSpinner::new()` seeds `Color::Rgb(255, 0, 0)` and derives its
  trail/inactive palette from that
  (`crates/opencode-tui/src/components/spinner.rs:81`, `:93-96`). The prompt feeds it a color via
  `with_color`, but the default is a hardcoded red.
- Toasts hardcode a color table: `crates/opencode-tui/src/components/toast.rs:184-204` maps named
  colors to literals; only the border/accent uses theme tokens (`toast.rs:116-121`).
- Todo items use named ANSI colors instead of semantic tokens:
  `crates/opencode-tui/src/components/todo_item.rs:26-29` (`Gray`, `Yellow`, `Green`, `DarkGray`)
  rather than `theme.text_muted` / `theme.warning` / `theme.success`.
- Agent picker uses named colors and a raw literal:
  `crates/opencode-tui/src/components/dialogs/agent_select.rs:30-55` (`Color::Cyan`, `Color::Magenta`,
  `Color::Yellow`, `Color::Green`, `Color::Rgb(180, 100, 255)`).
- Syntax highlighting ignores the active theme entirely: the syntect highlighter selects fixed
  `base16-ocean.dark` / `base16-ocean.light` / `InspiredGitHub` themes
  (`crates/opencode-tui/src/components/markdown/syntax.rs:18-20`, `:82-93`), so code blocks are colored
  by a bundled palette rather than the selected preset. `CodeTheme` itself does map app tokens
  (`crates/opencode-tui/src/components/markdown/code_block.rs:368-381`), but syntect overrides it.
- Markdown renderer falls back to a literal white:
  `crates/opencode-tui/src/components/markdown/renderer.rs:631` (`Color::White`).
- Light mode has no dedicated contrast handling outside `selected_foreground`
  (`crates/opencode-tui/src/theme/mod.rs:151-162`); several of the hardcoded surfaces above will also
  be unreadable on light backgrounds.

## Scope

- Route the spinner, toasts, todo items, agent picker, and any other hardcoded surfaces through
  `Theme` tokens, adding tokens to `Theme` where a concept is missing.
- Remove the dead `Styles` helper or reimplement it as thin accessors over `Theme`; do not leave two
  color tables.
- Choose the syntect theme from the active app theme (light/dark-aware) or replace syntect foreground
  colors with app tokens so code blocks match the preset.
- Replace remaining `Color::White` / named-color fallbacks in markdown and dialogs with theme tokens.
- Add a regression guard (test or lint-style assertion) that flags new hardcoded colors outside the
  theme module, if practical.

## Non-goals

- Adding, removing, or re-authoring preset theme JSON files.
- Changing the theme selection UX, the theme list dialog, or how the theme name persists.
- Redesigning any component's layout or glyphs.
- Tool-call block styling (that is FEAT-055/FEAT-056).

## Done when

- Switching themes recolors the spinner, toasts, todo items, agent picker, code/syntax blocks, and
  markdown fallbacks consistently with the rest of the UI.
- The dead `Styles` struct is gone or delegates to `Theme`; there is a single color source of truth.
- No component outside `theme/` hardcodes a UI color except for documented, intentional fallbacks.
- Light mode is legible on every surface listed above.
- `cargo check -p opencode-tui` and `cargo test -p opencode-tui` pass, with at least one test asserting
  a component derives its color from the provided `Theme`.

## Recommended verification

- `ort-build`, then `ort`; cycle through `opencode`, a light preset, and a high-contrast preset, and
  confirm the spinner, toasts, todos, agent picker, and code blocks all recolor.
- Force the spinner active and trigger an info/success/warning/error toast; confirm each matches theme
  tokens.
- Render a fenced code block in several languages; confirm syntax colors track the active theme rather
  than the fixed base16 palette.
- Repeat on a light preset and confirm no white-on-white or black-on-black text.
- `grep -rn "Color::" crates/opencode-tui/src --include=*.rs | grep -v theme/mod.rs` and account for
  each remaining match.

## Related Items

- `FEAT-028` Add a hide-tool-calls toggle that compacts runs to a tool-call count
- `FEAT-053` Make TUI display-toggle defaults configurable from opencode.json
- `FEAT-032` Rebrand TUI branding constants
- `BUG-022` `/thinking` toggle shows a line count instead of reasoning content

## Notes

- Relevant files: `crates/opencode-tui/src/theme/mod.rs`,
  `crates/opencode-tui/src/components/spinner.rs`,
  `crates/opencode-tui/src/components/toast.rs`,
  `crates/opencode-tui/src/components/todo_item.rs`,
  `crates/opencode-tui/src/components/dialogs/agent_select.rs`,
  `crates/opencode-tui/src/components/markdown/syntax.rs`,
  `crates/opencode-tui/src/components/markdown/code_block.rs`,
  `crates/opencode-tui/src/components/markdown/renderer.rs`.
- `ThemeMode` is binary (`Dark`/`Light`); syntax theme selection can key off
  `code_luminance(theme.text)` as `select_theme` already does, but should pick app-derived colors
  rather than the bundled syntect themes if strict consistency is the goal.
- Coordinate with FEAT-055/FEAT-056 so tool/terminal blocks inherit the same theme tokens rather than
  re-hardcoding colors during their rework.

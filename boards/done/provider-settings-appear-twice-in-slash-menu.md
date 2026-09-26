---
id: "BUG-042"
title: "Provider settings appear twice in the slash menu"
priority: "P2"
type: "bug"
area: "BUG"
spec: ""
status: "done"
created: "2026-09-26"
---

# Provider settings appear twice in the slash menu

## Summary

Two commands in the slash-command registry share the identical title `Open Provider Settings`:
`/connect` (`CommandAction::ConnectProvider`) and `/settings` (alias `/provider`,
`CommandAction::OpenSettings`). `CommandRegistry::search` matches a query against command titles as
well as names and aliases, so typing `/provider`, `/settings`, or `/connect` lists both entries with
the same visible label. The user sees "provider settings" twice and cannot tell which one is the
authoritative entry point.

This is a regression against the closeout claim of `START-027` ("`/connect` now routes back into
`Settings > Provider` so the product no longer presents a competing provider-setup entry point").
The routing was unified, but the two commands remained separately discoverable under the same title.

## Reported Behavior

- Open the in-session slash menu (type `/`).
- Type `provider`, `settings`, or `connect`.
- Two results render with the same title `Open Provider Settings`.
- Both are actionable but resolve through different actions (`ConnectProvider` vs `OpenSettings`), so
  the duplication is visible but the destination is not obviously identical.

## Root Cause

- `crates/opencode-tui/src/command.rs:310-319` registers `/connect` with title `Open Provider Settings`.
- `crates/opencode-tui/src/command.rs:321-330` registers `/settings` (alias `/provider`) with the same
  title `Open Provider Settings`.
- `crates/opencode-tui/src/command.rs:546` scores candidates against `cmd.title`, so a title match
  surfaces a command even when the typed query does not match its name.
- The slash popup only deduplicates by `cmd.name` (`crates/opencode-tui/src/components/slash_command.rs:80-91`),
  so distinct names with identical titles both survive filtering.

## Why This Matters

`Settings > Provider` is the documented authoritative provider setup path. Showing two identically
labelled entries in the primary command menu reintroduces exactly the "competing provider-setup entry
point" confusion that `START-027` was meant to eliminate, and the user reports it is still prevalent.

## Expected Behavior

- The slash menu presents a single discoverable provider-settings entry point.
- If `/connect` is intentionally retained as an alias/route, it must not render a second
  `Open Provider Settings` result alongside `/settings`.
- The retained entry's title/description makes clear it opens `Settings > Provider`.

## Done When

- Typing `provider`, `settings`, or `connect` in the slash menu yields exactly one provider-settings
  entry.
- Automated coverage asserts that no two visible command entries share the same title (or that the
  provider-settings title is unique) after title-based search.
- Manual verification: open `/`, type `provider`, confirm one result, and confirm it opens
  `Settings > Provider`.

## Verification

- `cargo test -p opencode-tui command`
- `ort-build`, then `ort`; type `/provider` and confirm a single `Open Provider Settings` entry.

## Related Items

- `START-027` Unify provider setup into one authoritative user path (done; closeout claim contradicted
  by the current registry).
- `BUG-002` Remove duplicate session actions and visible hotkey hints from the session UI (prior
  duplicate-command precedent).
- `BUG-037` Slash menu freezes keyboard input (same slash-popup surface, different defect).

## Notes

- Fix direction: rename one title, or drop the `/connect` command entry and keep it as an alias of
  `/settings`, or deduplicate search results by title. Confirm the intended `/connect` UX before
  removing it.

## Dev Notes

- Chose the alias direction: `/connect` is now an alias of the single `/settings` provider-settings
  command instead of a separate registration with a duplicate title.
- `crates/opencode-tui/src/command.rs`: removed the standalone `/connect` `SlashCommand`, added
  `/connect` to `/settings` aliases (`/provider`, `/connect`), and removed the now-unused
  `CommandAction::ConnectProvider` variant. `/connect` therefore still resolves and still routes to
  `Settings > Provider` via `CommandAction::OpenSettings`.
- `crates/opencode-tui/src/app/app.rs`: removed the `CommandAction::ConnectProvider` match arm, which
  called the same `open_provider_settings()` as `OpenSettings`.
- Added regression test `provider_settings_surface_exactly_one_entry` asserting that queries
  `provider`, `settings`, and `connect` each surface exactly one `Open Provider Settings` entry and
  that `registry.get("/connect")` maps to `CommandAction::OpenSettings`.
- Added render-level regression test `provider_query_renders_one_provider_settings_entry`, which
  drives the real `SlashCommandPopup` filter path and renders it through a ratatui `TestBackend`,
  asserting the string `Open Provider Settings` appears exactly once on screen.

## Verification Notes

- `cargo fmt --all`
- `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test -p opencode-tui command` -> 8 passed, including the new
  `provider_settings_surface_exactly_one_entry`.
- `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test -p opencode-tui --lib slash_command` -> 4 passed, including
  the new `provider_query_renders_one_provider_settings_entry`.
- Full `cargo test -p opencode-tui` has 2 pre-existing flaky `components::prompt::tests` failures
  (`tab_autocomplete_uses_first_candidate`, `utf8_backspace_delete_and_cursor_are_char_safe`);
  confirmed the same test fails intermittently on unmodified `development`, so it is unrelated to
  this change.

## QA Report

```
QA: BUG-042 — slash menu renders one provider settings entry
commit: bug/BUG-042-provider-settings-appear-twice
binary: shared-target opencode-tui test harness (cargo)
command: cargo test -p opencode-tui --lib provider_query_renders_one_provider_settings_entry
base signal (development command registry): rendered "Open Provider Settings" == 2 (FAIL)
fix signal: rendered "Open Provider Settings" == 1 (PASS)
result: PASS
```

## QA Handoff

- PR branch `bug/BUG-042-provider-settings-appear-twice`; awaiting merge closeout.

## Closeout

- PR #113 (`fix(tui): surface a single provider settings slash command`) merged into `development` at
  merge commit `b826091`.
- Code/task completeness: the PR removes the duplicate provider-settings command surface and adds both
  registry-level and render-level regression tests, matching this card's scope.
- QA report recorded above and passing; card promoted from `qa` to `done`.
- Branch `bug/BUG-042-provider-settings-appear-twice` and the temporary worktree were cleaned up.




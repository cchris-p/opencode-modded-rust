---
id: "BUG-031"
title: "Temporarily hide OpenRouter from provider lists"
priority: "P1"
type: "bug"
area: "BUG"
spec: ""
status: "todo"
created: "2026-09-22"
---

# Temporarily hide OpenRouter from provider lists

## Summary

Temporarily remove OpenRouter from normal provider/model selection lists while keeping the provider implementation available for explicit configuration and later re-enable work.

## Why this exists

OpenRouter is currently implemented and documented as supported, but it should not appear in day-to-day provider lists while its current UX/support state is being reconsidered. This is a temporary visibility change, not a permanent provider removal.

## Scope

- Hide `openrouter` from user-facing provider/model lists in the TUI and CLI.
- Hide `openrouter` from the `Settings > Provider` curated provider list and provider auth/status summary.
- Keep explicit `openrouter/...` config handling intact unless implementation evidence shows that visible-list hiding requires a narrower runtime guard.
- Do not delete the OpenRouter provider module, catalog entries, auth key mapping, transform logic, or tests unless a follow-up explicitly changes the provider invariant.

## Done when

- Normal provider selection surfaces no longer list OpenRouter.
- Existing explicit OpenRouter configuration either still works or fails with a clear, intentional message documented in implementation notes.
- `invariants/providers.md` remains accurate for the temporary behavior.
- The implementation notes identify what needs to be reverted or changed to re-enable OpenRouter visibility.

## Likely touchpoints

- `crates/opencode-tui/src/components/settings.rs`
- `crates/opencode-server/src/routes.rs`
- `crates/opencode-cli/src/main.rs`
- `crates/opencode-tui/src/components/dialogs/model_select.rs`

## Recommended verification

- Run the TUI provider settings path and confirm OpenRouter is absent.
- Run CLI provider/model listing paths and confirm OpenRouter is absent.
- Confirm an explicit non-OpenRouter provider still works.
- Confirm explicit OpenRouter config behavior is documented.

## Related items

- `START-012` Refresh provider and model catalog
- `START-027` Unify provider setup into one authoritative user path
- `FEAT-015` Preserve manually selected model and provider across sessions
- `invariants/providers.md`

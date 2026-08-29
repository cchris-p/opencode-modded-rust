---
id: "START-012"
title: "Refresh provider and model catalog"
priority: "P1"
type: "feature"
area: "START"
spec: ""
status: "done"
created: "2026-08-29"
---

# Refresh provider and model catalog

## Summary

Update the project's provider and model catalog so it closely matches the current OpenCode reference at `/Users/cchrisleepyles/repos/opencode-modded` instead of the outdated local list.

## Why this exists

The current model and provider surface is visibly stale in the TUI, which makes the product feel behind and blocks realistic day-to-day use.

## Scope

- Audit the provider and model definitions in this repo against `/Users/cchrisleepyles/repos/opencode-modded`.
- Restrict V1 provider scope to `openai`, `anthropic`, `deepseek`, and `openrouter`.
- Refresh provider entries, model lists, defaults, and labels for those four providers.
- Add `openrouter` placeholders only where needed so the catalog surface is present even if day-to-day support remains light.
- Keep this story focused on provider/model catalog data, selection metadata, and any minimal wiring required to expose the refreshed catalog.
- Do not treat full provider setup UX as part of this story; track that separately under `START-009`.
- If exposing the `openai` catalog requires mirroring current Codex/OpenAI auth assumptions from the reference repo, document that dependency or include only the minimum supporting wiring needed for catalog correctness.
- Capture all intentionally unsupported providers or models as explicit follow-up items instead of leaving silent gaps.

## Done when

- The provider list in this repo is aligned with the reference repo for the V1 set: `openai`, `anthropic`, `deepseek`, and `openrouter`.
- Default and suggested models for those providers are no longer obviously outdated in the TUI or CLI model listing.
- `openrouter` appears as an intentional placeholder surface rather than an accidental omission.
- Any provider or model intentionally excluded from V1 is documented in follow-up board items.
- Any dependency on OpenAI Codex auth setup is called out explicitly, either in implementation notes or a linked follow-up item.

## Recommended verification

- Compare the local provider/model catalog against `/Users/cchrisleepyles/repos/opencode-modded` for the four in-scope providers.
- Run the local model-listing path and verify the refreshed defaults and labels are visible.
- Confirm any out-of-scope providers from the reference repo are documented as deferred work.

## Related items

- `START-002` Freeze TS reference line
- `START-008` Full parity deferred
- `START-009` Port provider setup from OpenCode
- `START-015` Mirror OpenAI auth configuration in settings
- `START-014` Track deferred provider integrations beyond the V1 catalog

## Notes

- Prefer a structured catalog refresh over one-off model name edits.
- `/Users/cchrisleepyles/repos/opencode-modded` is the canonical reference repo for provider/model comparisons in this product.
- OpenAI auth configuration dependencies should link to `START-015` instead of being folded into this catalog story.

## Dev Notes

- Added a bundled four-provider fallback catalog in `crates/opencode-provider/src/bootstrap.rs` so the repo no longer falls back to older hardcoded provider model lists when no models.dev cache is present.
- Limited the user-facing CLI and server provider/model listings to the current V1 surface: `openai`, `anthropic`, `deepseek`, and `openrouter`.
- Refreshed the TUI model-picker placeholder data to current model families for those four providers.
- OpenAI-specific auth/setup follow-up remains tracked under `START-015` rather than expanding this story into provider setup work.

## Verification

- `cargo check -p opencode-provider -p opencode-server -p opencode-cli -p opencode-tui`
- `cargo test -p opencode-provider bundled_v1_catalog -- --nocapture`

## PR

- https://github.com/cchris-p/opencode-modded-rust/pull/3

## Completion

- Merged into `development` after local QA confirmation on 2026-08-29.

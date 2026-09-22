---
id: "START-029"
title: "Set up OpenAI Codex 5.5 compatibility"
priority: "P1"
type: "feature"
area: "START"
spec: "docs/provider-setup.md"
status: "refinement"
created: "2026-09-22"
---

# Set up OpenAI Codex 5.5 compatibility

## Summary

Evaluate and document whether the Rust product can use OpenAI Codex 5.5 through the existing OpenAI provider path, then implement only the compatibility gaps required to make setup reliable from the normal provider setup flow.

## Why this exists

The user wants to try Codex 5.5 as a daily-driver model. The codebase already has OpenAI auth, OpenAI Responses transport, GPT-5/Codex reasoning-model handling, and the `Settings > Provider` setup path, but Codex 5.5 has not been verified as an explicit supported model in this product.

This card should turn the current uncertainty into one of two clear outcomes:

- Codex 5.5 works with documented setup steps.
- Codex 5.5 is blocked by explicit, tracked runtime, catalog, auth, or account-access gaps.

## Current Evidence

- `docs/provider-setup.md` defines `Settings > Provider` as the authoritative V1 setup path.
- `START-015` implemented OpenAI auth entry/status in settings and persisted auth reuse.
- `START-027` unified provider setup around `Settings > Provider`, with config and environment overrides treated as secondary paths.
- `crates/opencode-provider/src/openai.rs` has a native OpenAI Responses route and falls back to chat completions if Responses fails.
- `crates/opencode-provider/src/openai.rs` recognizes `gpt-5` and `codex` model IDs when mapping reasoning effort.
- `crates/opencode-provider/src/responses.rs` treats `gpt-5*` and `codex-*` IDs as OpenAI Responses reasoning models.
- `crates/opencode-provider/src/openai.rs` still contains an older hardcoded OpenAI model list (`gpt-4o`, `gpt-4o-mini`, `gpt-4-turbo`, `o1-preview`, `o1-mini`), so the UI/catalog may not expose Codex 5.5 even if direct `-m` model selection works.

## Setup Direction To Validate

Use the normal OpenAI provider path first:

1. Build the current product with `ort-build`.
2. Launch with `ort` from the intended workspace.
3. Open `Settings > Provider`.
4. Select `openai`.
5. Confirm OpenAI auth is present, or enter an OpenAI API key there.
6. Select the closest available OpenAI model if Codex 5.5 is visible.
7. If Codex 5.5 is not visible in the picker, test direct model selection from the CLI/TUI config path with the exact OpenAI model id, expected to be something in the `gpt-5*` or `codex-*` family.
8. Run a small non-tool prompt and verify a normal response.
9. Run a small tool-using prompt and verify tool calls, tool results, and continuation all work.
10. If the model rejects the request, capture the exact HTTP/status/provider error before changing code.

## Open Questions

- What is the exact OpenAI API model id for "Codex 5.5" on the account being tested?
- Is the model available through the standard OpenAI API key path, or only through a Codex/ChatGPT subscription or a different auth scope?
- Does the model require the Responses API only, or does it also accept chat completions fallback?
- Does the model require extra Responses provider options beyond the current reasoning-effort mapping, such as encrypted reasoning inclusion, verbosity, priority/flex processing, prompt cache, or store settings?
- Should Codex 5.5 be selectable in `Settings > Provider`, or is direct advanced model override acceptable for initial use?

## Likely Touchpoints

- `docs/provider-setup.md`
- `crates/opencode-provider/src/openai.rs`
- `crates/opencode-provider/src/responses.rs`
- `crates/opencode-provider/src/bootstrap.rs`
- `crates/opencode-provider/src/transform.rs`
- `crates/opencode-server/src/routes.rs`
- `crates/opencode-tui/src/components/dialogs/provider.rs`
- `opencode.jsonc`

## Scope

- Confirm the exact model id and auth/access requirements for Codex 5.5.
- Verify whether the existing OpenAI Responses path can complete normal text prompts with that model.
- Verify whether the existing OpenAI Responses path can complete tool-using coding prompts with that model.
- Add Codex 5.5 to the local catalog/model picker if the model is usable but hidden.
- Add or adjust model-specific OpenAI Responses options only if real provider errors prove they are required.
- Document the final setup path in repo-local docs.

## Non-goals

- Full upstream provider parity beyond the Codex 5.5 path.
- Replacing the existing `Settings > Provider` setup direction.
- Building a separate Codex-only auth flow unless API-key or existing OpenAI login support is proven insufficient.
- Speculative provider-option changes without a captured failing request or upstream reference evidence.

## Done When

- The exact model id and access path for Codex 5.5 are recorded on this card.
- The product either successfully runs Codex 5.5 or records the concrete blocker that prevents setup.
- If usable, Codex 5.5 is reachable through the documented provider setup path or an explicitly documented advanced override.
- If catalog visibility is needed, Codex 5.5 appears in the model picker with correct provider metadata.
- Tool-use behavior is verified on the OpenAI Responses path, including follow-up continuation after tool results.
- Any required code changes have focused tests or a documented manual verification path.
- `docs/provider-setup.md` is updated with Codex 5.5 setup instructions or blocker notes.

## Recommended Verification

- `ort-build`
- `ort`, then `Settings > Provider` OpenAI auth/model setup.
- Direct smoke test with the exact model id: `opencode run -m openai/<model-id> 'reply with exactly OK'`.
- Tool smoke test with the exact model id: ask the model to inspect a small local file and report one fact.
- Confirm failures distinguish product bugs from OpenAI account/model-access errors.
- Run relevant provider tests after code changes, likely `cargo test -p opencode-provider openai` plus any new targeted tests.

## Related Items

- `START-012` Refresh provider and model catalog
- `START-015` Mirror OpenAI auth configuration in settings
- `START-027` Unify provider setup into one authoritative user path
- `FEAT-013` Model capability gating and deprecated model surfacing
- `BUG-005` OpenAI-compatible chat providers reject requests once tools are attached
- `BUG-012` Session summary runs before tool results and breaks OpenAI-compatible continuation

## Initial Guidance For User Setup

It may already be possible if the OpenAI account exposes the model through the standard API key and the exact model id is accepted by `/v1/responses`.

Try this first after building:

1. Run `ort-build`.
2. Run `ort` from the workspace you want to use.
3. In `Settings > Provider`, choose `openai` and confirm auth is configured.
4. If Codex 5.5 appears in the model picker, select it and run a short prompt.
5. If it does not appear, use a direct model override with the exact model id, for example `opencode run -m openai/<model-id> 'reply with exactly OK'`.

Do not assume the displayed catalog is authoritative yet. Current code appears more likely to need catalog/model-picker work than a full transport rewrite, but real compatibility depends on the exact model id, OpenAI account access, and any provider errors from the first smoke tests.

## Blockers To Resolve If Setup Fails

- Missing or incorrect exact Codex 5.5 model id.
- OpenAI account lacks API access to that model.
- Existing OpenAI auth path does not provide the credential type/scope that Codex 5.5 requires.
- Model picker/catalog hides the model even though direct `-m openai/<model-id>` works.
- Responses request options are incomplete for this model family.
- Tool-call streaming or continuation differs from the currently tested OpenAI Responses behavior.

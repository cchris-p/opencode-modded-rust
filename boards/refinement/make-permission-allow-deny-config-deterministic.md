---
id: "START-026"
title: "Make permission allow/deny config deterministic"
priority: "P1"
type: "feature"
area: "START"
spec: ""
status: "refinement"
created: "2026-08-30"
---

# Make permission allow/deny config deterministic

## Summary

Fix path-based permission behavior so configured allow/deny rules and runtime "allow always" approvals resolve deterministically for `external_directory` access, without repeated approval prompts for directories that already have an explicit effective allow decision.

## Why this exists

The current behavior is faulty: a directory declared as always allow can still trigger runtime permission prompts. In the current Rust stack, path-based approval behavior is split across config loading, permission rule evaluation, external-directory boundary derivation, and in-memory session approval caching. That makes it too easy for equivalent paths or overlapping rule sources to produce different outcomes. The product needs one authoritative permission-resolution model so configured behavior is stable across runs, scopes, and repeated accesses.

## Scope

- Audit the current `external_directory` permission resolution path across config loading, rule normalization, external-path canonicalization, and runtime approval checks.
- Make explicit configured allow and deny path decisions deterministic for repeated access to matching directories and files.
- Ensure global and workspace-level config participate in permission resolution the same way they do in `/Users/cchrisleepyles/repos/opencode-modded`, while improving reliability so they always work without scope-order weirdness.
- Define and implement a single precedence model when the same or overlapping rules appear in multiple config scopes or when configured rules interact with runtime-saved approvals.
- Align the implementation with the existing config load order already encoded in `crates/opencode-config`, unless a concrete mismatch with the reference product is found and documented.
- Keep the work focused on permission/config correctness, not broad approval UX redesign.

## Non-goals

- Broad redesign of the approval prompt UI
- Reworking unrelated tool-permission rules that do not depend on path matching
- Introducing a second permission persistence mechanism when existing config and saved-approval paths can be made deterministic

## Implementation Direction

- Treat `external_directory` as the primary failing path unless investigation proves the same bug exists in other path-scoped permissions such as `read`, `edit`, or `lsp`.
- Use the current Rust config load order as the starting precedence model: global config, explicit project config discovered to the project boundary, `.opencode` config directories, then higher-priority runtime/env overlays.
- Preserve the reference product's scope behavior where it is already intentional: config closer to the opened workspace should override broader workspace config, while policy/rule evaluation may intentionally reverse load order so higher-priority global overrides remain possible where the upstream model requires that.
- Decide and document one explicit precedence rule for configured path rules versus runtime-saved "allow always" approvals. The upstream reference already treats configured deny as higher priority than saved approval; this card should preserve that kind of deterministic override behavior.
- Normalize path matching before evaluation. Equivalent references such as canonical absolute paths, existing symlink-resolved paths where applicable, and directory patterns like `/path/dir`, `/path/dir/`, and `/path/dir/*` must not silently diverge.
- Prefer reusing one canonical path-boundary helper for external-directory permission resources instead of letting each tool derive its own approval pattern ad hoc.

## Acceptance Detail

- A path declared as always allow in effective config is not re-prompted by the agent for matching accesses.
- A path declared as always deny in effective config is rejected consistently for matching accesses.
- A runtime "allow always" approval suppresses repeated prompts for the same effective path boundary within its intended persistence scope.
- Configured deny precedence over saved approval is deterministic and covered by tests.
- Global config and workspace config are both loaded and applied deterministically.
- The effective result for overlapping rules is stable and explainable, with one documented precedence model.
- Path matching behavior is normalized so equivalent directory references do not produce different outcomes because of formatting, ancestor selection, or resolution differences.

## Proposed Decisions

- The authoritative resource for `external_directory` approval should be the canonical existing external directory boundary expressed as `dir/*`, not the originally requested raw path string. File requests should resolve to their canonical parent directory boundary; directory requests should resolve to that directory boundary directly.
- Direct configured permission rules should follow normal config specificity: broader config loads first and more local config overrides later. In practice, workspace-local config should beat global config for direct permission rules unless an explicitly documented higher-priority overlay source applies.
- If this repo also keeps a separate policy-style layer with intentionally reversed evaluation order for upstream parity, that behavior must stay isolated and documented rather than leaking into direct path-rule matching semantics.
- Runtime-saved "allow always" approvals should be lower priority than explicit configured deny rules and lower priority than explicit configured allow/deny rules generally. Saved approvals are a convenience layer, not the source of truth.
- Runtime-saved "allow always" approvals for this card should be treated as session-scoped unless the implementation already has an existing persisted saved-permission path that can be reused deterministically. This item should not invent new cross-session persistence semantics.
- Normalization should happen before both configured-rule evaluation and saved-approval lookup so the same directory cannot produce different results because one path was canonicalized and the other was not.

## Likely Touchpoints

- `crates/opencode-config/src/loader.rs` for config discovery, merge order, and env-overlay precedence
- `crates/opencode-permission/src/ruleset.rs` for rule expansion, merge semantics, and match evaluation
- `crates/opencode-permission/src/engine.rs` for runtime "allow always" approval caching semantics
- `crates/opencode-tool/src/external_directory.rs` for external-path boundary derivation and permission request pattern generation
- Any tool callsites that currently derive their own external-directory patterns instead of using one canonical path-resolution path
- TUI or API surfaces only if needed to expose the effective configuration clearly

## Verification

- Automated tests cover always-allow and always-deny behavior for `external_directory` across global config, workspace config, and `.opencode` config sources.
- Tests cover overlapping global and workspace rules and prove the same precedence result every time.
- Tests cover configured rules interacting with runtime-saved "allow always" approvals, including configured deny overriding saved approval.
- Tests cover normalized-equivalent paths such as canonical absolute path, trailing-slash variants, file path vs parent directory boundary, and existing descendant paths under the same approved directory.
- Tests prove the same path decision survives repeated accesses in the same session without re-prompting.
- Tests prove session-scoped "allow always" approvals do not silently change configured precedence behavior.
- If an existing persisted saved-permission path is reused, restart-persistence tests cover that behavior explicitly; otherwise tests confirm the session-only boundary clearly.
- Manual verification confirms a configured always-allow external directory no longer triggers repeated permission prompts in the user-facing flow.

## Done when

- Permission allow/deny behavior for path-scoped approvals is deterministic and reliable across repeated accesses.
- Global/workspace config behavior matches the intended upstream/reference model from `/Users/cchrisleepyles/repos/opencode-modded` where that model is correct.
- Any intentional deviations from the reference behavior are documented explicitly and improve determinism rather than weaken it.
- There is one authoritative runtime path for path-based permission resolution rather than scattered ad hoc checks.
- The final card implementation leaves a documented precedence model that future permission work can reuse instead of rediscovering.
- The card no longer depends on implementation-time product decisions about resource shape, config precedence, or saved-approval scope.

## Notes

- Treat the reference product as the baseline for config scope behavior, but prefer clearer and more deterministic semantics where the current behavior is ambiguous or flaky.
- The current Rust implementation already has notable relevant behavior to preserve or clarify:
- `ConfigLoader::load_all` encodes a concrete config-source order.
- `ruleset::evaluate` currently resolves by reverse-last-match semantics.
- `PermissionEngine` stores "always" approvals in session memory.
- `external_directory` requests currently derive a `parent_dir/*` pattern ad hoc in tool code.
- If the bug turns out to involve path canonicalization, config merge order, or cached permission state, document the exact failure mode before broadening scope.

## Related Items

- `START-018` Complete TUI approval and question handling
- `START-004` Assess current Rust state
- `START-005` Define V1 runtime loop
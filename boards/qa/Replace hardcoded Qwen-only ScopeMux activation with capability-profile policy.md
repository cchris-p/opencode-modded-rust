---
id: "FUTURE-PROOFING-015"
title: "Replace hardcoded Qwen-only ScopeMux activation with capability-profile policy"
priority: "P1"
type: "feature"
area: "FUTURE-PROOFING"
spec: "wiki/workspace-intelligence.md"
status: "qa"
created: "2026-10-24"
---

# Replace hardcoded Qwen-only ScopeMux activation with capability-profile policy

## Summary

Move Workspace Intelligence (ScopeMux) activation from the hardcoded
`ollama` + `qwen*` branch in `crates/opencode-scopemux/src/lib.rs` into a capability
profile.

The Qwen path remains the initial experiment profile. New models, subagents, and
workspaces get a profile entry, not a code change.

## Why this exists

The current gate was the right experiment: it kept the default daily-driver path on the
generic provider while testing the map on a small local model. As a permanent rule,
however, it is a future-proofing anti-pattern:

- activation is tied to a model id, not to capability,
- subagents and other local models cannot opt in,
- adding a model requires code,
- removing the experiment requires code,
- it hides the policy from the session and the decision log.

## Scope

- Define a capability-profile entry for workspace intelligence:
  - provider (generic/native),
  - enabled rungs (W0-W4),
  - languages,
  - token budget,
  - fallback,
  - experiment flag.
- Select the profile at session start; record it in the session and decision log.
- Keep the Qwen-via-Ollama path as a named experiment profile with identical behavior.
- Allow a profile to enable ScopeMux for a subagent task class.
- The profile is the hard bound for System-1 runtime routing (`DECISION-017`); the
  classifier may select within it, never outside it.
- Reject an unknown provider/rung with a clear error and fall back to generic.
- Remove the model-id branch from the core; keep `model_scope_enabled` only if it
  implements the Qwen profile, not the policy.
- Add tests for profile selection, fallback, and no-profile behavior.
- Update `invariants/providers.md`, `AGENTS.md`, and `wiki/workspace-intelligence.md` to
  reflect profile-driven activation.

## Acceptance criteria

- With no profile, every model uses the generic provider.
- The Qwen experiment profile reproduces current behavior.
- A new model can enable ScopeMux by profile, with no code change.
- The session and decision log record the profile.
- Unknown or incompatible profiles fall back to generic.
- The core no longer hardcodes a model-id rule as policy.
- Tests cover profile selection and fallback.

## Non-goals

- Changing the default model.
- Enabling ScopeMux for frontier models by default.
- Building a UI for profiles.
- Removing the Qwen experiment before evaluation.

## Dev Notes

- Added `WorkspaceIntelligenceProfile` / `WorkspaceIntelligencePolicy` contracts in
  `opencode-types` and `opencode-scopemux`. A profile records provider, model
  selectors, rungs, languages, token budget, fallback, and experiment flag.
- `workspaceIntelligence.profiles` is now parsed from `opencode.jsonc` and mapped to a
  policy in the server for both the main prompt runner and the persisted child-session
  runner. The first validated matching profile wins; invalid profiles are logged and
  dropped; no match falls back to generic.
- `SessionPrompt` accepts the policy via `with_workspace_intelligence_policy`, passes it
  through retrieval, uses the profile token budget for the injected reminder, and
  records the active profile in session metadata (`workspace_intelligence_profile`) and
  the `retrieval provider selected` log line. The language bound is enforced before
  parsing.
- The built-in `ollama` + `qwen*` path is now the named `qwen-ollama-experiment`
  profile. `model_scope_enabled` remains only as that profile's predicate.
- Repo `opencode.jsonc` opts this checkout into a `deepseek-experiment` profile for
  `deepseek/deepseek-flash`; this is config, not a core model-id branch.
- Verification: `cargo check` for `opencode-types`, `opencode-config`,
  `opencode-scopemux`, `opencode-session`, `opencode-server`; focused tests for
  profile parsing, selection, rejection/fallback, budget, and language gating;
  `cargo fmt --all -- --check`; clippy on touched crates showed no new warnings in the
  changed code.
- Not yet run: native `scopemux-core` QA (`scripts/scopemux-qa-check.sh`) because this
  checkout has no `third_party/scopemux-core` or `SCOPEMUX_CORE_DIR`. The provider still
  falls back to generic until a native build is present.
- Follow-ups: record the profile in the decision log (not only session metadata), carry
  the policy into in-memory `SubtaskExecutor` subsessions, and measure DeepSeek with and
  without ScopeMux (`Measure ScopeMux against generic retrieval`).

## Related

- `FUTURE-PROOFING-013`
- `FUTURE-PROOFING-014`
- `FUTURE-PROOFING-017`
- `FUTURE-PROOFING-003` capability profiles and shim registry
- `DECISION-017` System-1 WIP routing
- `SCOPE-005` Qwen-only activation (predecessor)
- `invariants/providers.md`
- `invariants/workspace-intelligence.md`
- `wiki/workspace-intelligence.md`
---
id: "FUTURE-PROOFING-013"
title: "Workspace Intelligence Protocol program"
priority: "P1"
type: "epic"
area: "FUTURE-PROOFING"
spec: "wiki/workspace-intelligence.md"
status: "refinement"
created: "2026-10-24"
---

# Workspace Intelligence Protocol program

## Summary

Make ScopeMux a first-class, future-proof part of the project substrate under the name
**Workspace Intelligence Protocol (WIP)**: a versioned, capability-negotiated contract
between the harness and a workspace-intelligence provider.

This epic owns the role definition, protocol, activation policy, packaging, evaluation,
and the individual capability cards. It is the program home for the existing `SCOPE-*`
work, not a replacement for it.

## Locked decisions

- **Option B: workspace-intelligence layer.** ScopeMux is not just a retrieval
  optimization and not a general LSP replacement. It owns the context/data pillar for code
  workspaces.
- **Term: Workspace Intelligence Protocol (WIP).** ScopeMux is the first implementation.
- **LSP analogy is useful for the contract, not the scope.** Borrow capability negotiation,
  versioning, health, and multiple implementations. Do not rebuild LSP.
- **ScopeMux is part of the project substrate.** `opencode-modded-rust` is the main
  utilizer and reference consumer.
- **Small models and subagents are the primary beneficiary.** The map exists to return the
  smallest faithful slice, not a file dump.
- **Qwen-only activation is the initial experiment profile.** The target is
  capability-profile-driven activation.
- **Keep/expand/retire criteria are open.** Options are recorded in
  `FUTURE-PROOFING-017`; the decision is pending.

## Why this exists

The existing ScopeMux work is strong but scattered: a provider behind the retrieval
boundary, a map model, a plan-node projection, stage-aware slices, and a grep-replacement
card. What is missing is the strategic placement:

- no substrate pillar,
- no capability ladder,
- no profile-driven activation,
- no protocol/versioning,
- no evaluation/exit criteria,
- no data/provenance contract,
- no explicit subagent/small-model case.

WIP closes those gaps without making ScopeMux a hard dependency.

## Scope

- Define and maintain the WIP contract (`FUTURE-PROOFING-014`).
- Replace hardcoded Qwen-only activation with capability profiles
  (`FUTURE-PROOFING-015`).
- Package the native provider as an artifact and version the map schema
  (`FUTURE-PROOFING-016`).
- Measure ScopeMux against generic retrieval and set keep/expand/retire criteria
  (`FUTURE-PROOFING-017`).
- Wire map signals into decision classes (`DECISION-016`).
- Evaluate System-1 routing of WIP provider/rung/budget at runtime (`DECISION-017`).
- Own the scope-specific cards:
  - `SCOPE-006` map-backed locate tool with grep fallback (one capability, not the role),
  - `SCOPE-007` subagent-scoped workspace intelligence,
  - `SCOPE-008` retrieval provenance and outcomes,
  - `SCOPE-009` freshness/staleness SLOs,
  - `SCOPE-010` language coverage and fallback,
  - `SCOPE-011` out-of-process transport.
- Keep the generic provider as the default and fallback.

## Authority

- WIP is evidence-only. It never advances task stage, verification, review, completion, or
  a disposition.
- Plan nodes are projections of the authoritative task record; the task record wins.
- The runtime owns inclusion thresholds, token budgets, role filtering, and stage.
- WIP composes with `opencode-lsp`; it does not replace it.

## Execution order

1. `FUTURE-PROOFING-014` — protocol and capability negotiation.
2. `FUTURE-PROOFING-015` — profile-driven activation (can land with 014).
3. `FUTURE-PROOFING-016` — artifact packaging and schema versioning (parallel).
4. `FUTURE-PROOFING-017` — evaluation and exit criteria (parallel; gates investment).
5. `SCOPE-007`..`SCOPE-011` — capability work, ordered by value:
   subagents, data, freshness, language coverage, transport.
6. `DECISION-016` — decision integration after 014/015.
7. `DECISION-017` — System-1 WIP routing after 015 and the decision boundary; shadow
   first, profile-bounded.

## Non-goals

- Replacing `opencode-lsp`.
- Making ScopeMux a V1 hard dependency.
- A general-purpose platform for every harness.
- Owning task state or runtime decisions.
- Sending private code to a hosted engine.
- Building a second core loop.

## Done when

- WIP contract and capability negotiation exist and are tested with a stub.
- Activation is profile-driven; the Qwen-only gate is only a profile entry.
- The native provider is artifact-packaged and schema-versioned.
- A measured keep/expand/retire decision exists.
- At least one subagent path uses a scoped map slice.
- Retrieval provenance and outcomes are in the data contract.
- The generic fallback still completes tasks.

## Related

- `wiki/workspace-intelligence.md`
- `invariants/workspace-intelligence.md`
- `wiki/harness-substrate.md`
- `wiki/scopemux-integration-plan.md`
- `wiki/scopemux-map-integration.md`
- `SCOPE-001`..`SCOPE-006`
- `FUTURE-PROOFING-001` guidance epic

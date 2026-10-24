# Workspace Intelligence Invariants

These are binding truths for the Workspace Intelligence Protocol (WIP) and its first
implementation, ScopeMux. Explanatory detail lives in `wiki/workspace-intelligence.md`.

## Role

- WIP is the versioned contract between the harness and a workspace-intelligence
  provider.
- ScopeMux is the first WIP implementation and part of the project substrate.
  `opencode-modded-rust` is the main utilizer and reference consumer.
- WIP is the **context/data** capability. It is not a separate runtime, decision, or
  orchestration authority.

## Optionality

- WIP MUST be optional. The harness MUST pass the eight future-proofing tests without it.
- The generic repository-local retrieval provider MUST remain the default and fallback.
- Absent provider, unsupported language, provider failure, or budget exhaustion MUST
  degrade to generic behavior; the task MUST still complete.
- `scopemux` MUST NOT become a V1 hard dependency or a build requirement for the daily
  driver.

## Authority

- WIP PROPOSES context and map evidence. The runtime DISPOSES.
- WIP MUST NOT advance a task stage, verify, review, complete, or change a disposition.
- Plan nodes MUST be projections of the authoritative task record. The task record wins on
  disagreement.
- Heuristic links MUST be confidence-labeled and MUST NOT be treated as semantic truth.
- The runtime MUST own inclusion thresholds, token budgets, role filtering, and stage.

## Capability ladder

- Capabilities MUST be individually enableable, measurable, and removable:
  - W0 parser/index,
  - W1 map (delta/observability/refactor),
  - W2 task projection/reconciliation,
  - W3 map-backed tools,
  - W4 decision integration.
- Higher rungs MUST NOT be prerequisites for lower rungs.
- Removing any rung MUST NOT require a core-loop change.

## Protocol

- WIP MUST be versioned and capability-negotiated.
- A provider MUST advertise its protocol version, capabilities, languages, map schema
  version, and health.
- The harness MUST only request advertised capabilities.
- A missing capability MUST degrade to the next rung, not fail the task.
- An incompatible schema MUST be rejected with a clear reason and fall back.
- The contract MUST be tested with a deterministic stub, the generic provider, and the
  native provider.
- WIP MUST compose with `opencode-lsp`; it MUST NOT replace or duplicate the LSP client.

## Activation

- Activation MUST be capability-profile-driven, not hardcoded to a model id.
- The Qwen-via-Ollama ScopeMux path is an initial experiment profile, not a permanent
  policy.
- Every model or subagent class that uses WIP MUST have a profile recording enabled rungs,
  budgets, and fallback.
- The profile MUST be recorded in the session and the decision log.

## System-1 routing

- The WIP provider, rung, budget, and slice sufficiency MAY be proposed by a System-1
  decision provider.
- The capability profile is a hard bound. A classifier MUST NOT enable a rung, provider,
  or budget outside the profile.
- A low-confidence routing decision MUST fall back to the profile default.
- The runtime MUST enforce; the classifier only proposes.
- A routing decision MUST NOT advance a stage, verify, review, complete, or change a
  disposition.
- Routing decisions MUST be logged with profile, engine, confidence, cost, and fallback
  reason, and MUST be replayable.
- Routing MUST start in shadow mode and MUST NOT be promoted without evaluation.
- A null routing result is valid; static profiles MAY be sufficient.

## Subagents

- A subagent request MUST carry its own objective, stage, seeds, and budget.
- A subagent slice MUST be scoped to the subagent's task, not the parent's context.
- The parent task record remains authoritative.
- Per-subagent token and cost budgets MUST be enforced.
- A subagent without WIP MUST fall back to generic retrieval.

## Data

- Retrieval provenance, provider/version, capability, map schema, confidence, plan signals,
  fallback reason, and downstream outcome MUST be recorded.
- Retrieval and map data MUST follow `wiki/decision-log-egress.md`.
- Private code MUST NOT be sent to a hosted engine.
- Map provenance MUST be retained long enough to diagnose bad retrieval and to build
  evaluation/fine-tuning sets under the egress policy.

## Packaging

- The native provider MUST be consumable as a pinned artifact, not a per-machine C build.
- The native provider and map schema MUST be versioned independently.
- A missing or incompatible artifact MUST degrade to generic retrieval.
- A source build MAY remain a contributor path, not the user path.

## Evaluation

- WIP MUST be compared against generic retrieval on real tasks.
- Evaluation MUST include tokens, context precision/recall, task success, review quality,
  latency, freshness, fallback rate, and cost per successful task.
- Evaluation MUST use held-out and out-of-distribution tasks.
- Keep/expand/retire criteria MUST be recorded before a large investment decision.
- A null or negative result MUST be a valid outcome.

## Governance

- Changing these invariants is a policy change and requires human review.
- New WIP capabilities MUST be registered as board items with an evaluation plan and a
  retirement criterion.
- No vendor, benchmark, or convenience exception overrides the authority or egress rules.

## Related

- `wiki/workspace-intelligence.md`
- `wiki/scopemux-integration-plan.md`
- `wiki/scopemux-map-integration.md`
- `invariants/future-proofing.md`
- `invariants/retrieval.md`
- `invariants/providers.md`
- `invariants/integration-scope.md`
- `wiki/decision-log-egress.md`

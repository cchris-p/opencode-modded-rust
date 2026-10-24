# Future-Proofing Invariants

These are binding truths for the final desired `opencode-modded-rust` system. They
govern how the harness absorbs change. Explanatory detail lives in
`wiki/harness-substrate.md`.

## Absorption

- The harness MUST NOT make model-owned capability its core value. Model-facing
  prompting, scaffolding, and guardrails MUST be policy, not core.
- The harness MUST own layers the model cannot absorb: runtime, deterministic
  state, trust, efficiency, and private data.
- Model-compensating scaffolding MUST be removable when a model absorbs it.
  Keeping absorbed scaffolding is a defect, not conservatism.

## Core and policy

- The runtime MUST separate a small stable core from a volatile policy layer.
- The stable core MUST contain session/task state, lifecycle, tool execution,
  permission authority, durable storage, provider abstraction, and the event bus.
- The stable core MUST NOT import a specific decision engine, model, or prompt.
- Adding or removing a decision class, shim, prompt, or model profile MUST be a
  policy change, not a core change.

## Decision and generation

- Decisions and generation MUST be separable. A decision selects among known
  options with confidence and provenance; generation produces content.
- A decision provider MUST be replaceable without changing the runtime loop.
- A decision provider MUST NOT be required for the harness to complete work.

## Deletion-ready

- Every shim and policy item MUST have one owner, a firing or usage metric, a kill
  switch, a retirement criterion, and a documented removal path.
- Removing any single shim, decision class, or policy item MUST NOT require a core
  rewrite.
- A shim that cannot be removed without a core change is a design defect.

## The eight tests

A structural change MUST pass all eight tests:

1. **Swap** — replace the decision engine without touching the loop.
2. **Delete** — remove a shim, class, or policy item without breaking the runtime.
3. **Add** — add a decision class or shim without a core change.
4. **Model-upgrade** — adopt a model by adding a profile and evaluation, not a
   loop rewrite.
5. **Offline** — complete a real task without a hosted decision engine.
6. **Replay** — reproduce logged decisions under the recorded policy version.
7. **Cost-cap** — hit a spend ceiling and degrade gracefully.
8. **Audit** — explain every runtime decision from the log.

A change that fails a test is a core change and requires explicit justification.

## Efficiency and trust

- Efficiency and trust MUST be first-class evaluation axes, not tie-breakers.
- Efficiency MUST be measured across tokens, cost, cache behavior, latency,
  startup, memory, and disk footprint.
- Trust MUST be measured across permission blast radius, secret scope, spend caps,
  and audit coverage.
- Raw model latency MAY remain secondary; harness-induced cost and trust MAY NOT.

## Data

- Owned data MUST carry provenance and a sensitivity tier.
- Decision logs and outcomes MUST be append-only at runtime; corrections are new
  records.
- Retention, access, and egress MUST follow `wiki/decision-log-egress.md`.
- Secrets MUST never be logged or sent to a hosted engine.
- The decision log MUST NOT be synchronized to any remote store by default.

## Change management

- The exposed user-facing surface MUST remain minimal; internals MUST NOT become
  de facto contracts.
- Policy MUST be versioned, canaried, replayable, and reversible independently of
  the binary.
- A session MUST record the policy version and capability profile it used.
- Change-management cost is part of the design, not an afterthought.

## Degradation

- The harness MUST complete work when a hosted decision engine is unavailable,
  rate-limited, or over budget.
- The degradation ladder MUST be: preferred engine -> local engine -> deterministic
  rules -> baseline behavior.
- A billing, quota, or credit error MUST be a stop-and-resume gate, not a crash
  and not a permanent failure.

## Capability profiles

- Every supported model MUST have a capability profile.
- A new model MUST be adopted by adding a profile and running the evaluation pack.
- A profile MUST record which shims are enabled and which decision classes are
  delegated.
- Absorbed shims and classes MUST be disabled or removed through the profile, not
  through core code.

## Governance

- Changing these invariants is a policy change and requires human review.
- Exceptions MUST be recorded as board items with a test plan and an expiry.
- No feature, benchmark score, or vendor relationship overrides these rules
  silently.

## Related

- `wiki/harness-substrate.md`
- `wiki/decision-layer.md`
- `wiki/decision-log-egress.md`
- `wiki/post-jev-roadmap.md`
- `invariants/decision-layer.md`
- `invariants/verification.md`

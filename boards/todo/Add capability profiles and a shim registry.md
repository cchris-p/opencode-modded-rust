---
id: "FUTURE-PROOFING-003"
title: "Add capability profiles and a shim registry"
priority: "P1"
type: "feature"
area: "FUTURE-PROOFING"
spec: "wiki/harness-substrate.md"
status: "todo"
created: "2026-10-24"
---

# Add capability profiles and a shim registry

## Summary

Add two data-driven structures:

- **Capability profiles**: model -> enabled shims, delegated decision classes, prompt
  variants, and budgets.
- **Shim registry**: every shim with an owner, trigger, firing metric, cost, kill
  switch, retirement criterion, and board link.

These are the mechanisms for riding the flywheel: a new model is adopted by adding a
profile and evaluation, and an absorbed shim is retired by changing a profile, not by
editing the loop.

## Why this exists

Today, model-specific behavior is spread across prompts, parser code, and launcher
config. There is no single place that says "this model still needs this shim" or "this
shim no longer fires." That makes absorption invisible and retirement expensive.

## Scope

- Define a profile schema and store it as versioned data.
- Select a profile at session start; record it in the session and decision log.
- Define a shim record schema and register existing shims.
- Instrument shim firing counters and cost.
- Add a report of shims by firing rate, age, owner, and retirement status.
- Add a kill switch per shim.
- Add a startup check that rejects a profile referencing an unknown shim or decision
  class.
- Document the adoption workflow: new model -> profile -> evaluation -> enable/disable.

## Acceptance criteria

- Every existing shim has a registry record and an owner.
- A session records the capability profile it used.
- Disabling a shim is a profile change, not a code change.
- A zero-firing shim is surfaced with a removal recommendation.
- A new model can be adopted without a core change.
- The kill switch is tested.

## Non-goals

- Retiring every shim in this card.
- Building a UI for profiles.
- Automating profile selection from model metadata.
- Removing model-specific code that is not a shim (that is a separate cleanup).

## Related

- `FUTURE-PROOFING-001`
- `FUTURE-PROOFING-002`
- `FUTURE-PROOFING-006`
- `wiki/harness-substrate.md`
- `invariants/future-proofing.md`

---
id: "FUTURE-PROOFING-005"
title: "Build the eight-test future-proofing conformance suite"
priority: "P1"
type: "feature"
area: "FUTURE-PROOFING"
spec: "invariants/future-proofing.md"
status: "todo"
created: "2026-10-24"
---

# Build the eight-test future-proofing conformance suite

## Summary

Turn the eight future-proofing tests into an executable suite that runs in CI and
against a candidate release.

The tests are:

1. swap the decision engine without touching the loop;
2. delete a shim/class/policy item without breaking the runtime;
3. add a decision class without a core change;
4. adopt a model by adding a profile and evaluation;
5. complete a real task with no hosted decision engine;
6. replay a decision log and reproduce the behavior;
7. hit a spend cap and degrade gracefully;
8. explain every decision from the log.

## Why this exists

Future-proofing is otherwise a narrative. A suite makes it falsifiable and prevents a
structural regression from being merged as "just a small change."

## Scope

- Build fixtures and harnesses for each test.
- Test 1: compile the same runtime against a stub engine, Laya, and Jev adapter.
- Test 2: remove each registered shim/class behind a feature flag in a test build.
- Test 3: add a synthetic class through registry data only.
- Test 4: add a synthetic model profile and run a smoke task.
- Test 5: run a representative task with the hosted engine unreachable.
- Test 6: replay a recorded log and compare decisions and policy versions.
- Test 7: set a zero budget and assert graceful degradation, not a crash.
- Test 8: reconstruct a decision explanation from the log alone.
- Wire the suite into CI and document how to run it locally.

## Acceptance criteria

- All eight tests run in CI.
- A failure names the violated test and the offending change.
- The suite does not require live Jev/Laya credentials.
- The offline test uses a real task shape, not a stub prompt.
- The replay test is deterministic.
- The suite is documented in `wiki/harness-substrate.md`.

## Non-goals

- Benchmarking model quality.
- Replacing the existing regression suite.
- Testing every decision class; the suite tests the substrate.
- Requiring a network in CI.

## Related

- `FUTURE-PROOFING-001`
- `FUTURE-PROOFING-002`
- `FUTURE-PROOFING-004`
- `DECISION-004`
- `DECISION-005`
- `invariants/future-proofing.md`

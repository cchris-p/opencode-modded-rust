---
id: "FEAT-066"
title: "Explore smartphone-emulator control as an opencode-agent capability"
priority: "P3"
type: "research"
area: "FEAT"
spec: ""
status: "todo"
created: "2026-09-26"
---

# Explore smartphone-emulator control as an opencode-agent capability

## Summary

Investigate whether `ort` should gain a first-class capability to drive an Android smartphone emulator — boot/attach, screen and UI-tree capture, input injection, and app navigation — as a general agent tool rather than project-specific glue.

## Why this exists

Planning the standalone DraftKings emulator project (now tracked as `BET-001` on the `acebets` board) surfaced a product question: the same control primitives could be a reusable opencode-agent capability. This card is the adoption-research probe and does not commit the product to building it.

## Scope

Questions to answer:

- Which backends (Android `adb`/emulator, `scrcpy`, Appium, Genymotion) expose the needed control primitives, and what the local setup cost is.
- Tool contract shape: an observe action (screenshot / accessibility UI tree), an act action (tap, swipe, type, launch app), and a device-state query.
- How the capability maps onto the existing tool, permission, and approval model (`invariants/providers.md`, permission allow/deny, subagent behavior).
- Runtime concerns: a long-lived device process, streams of screenshots, and interaction with `OPENCODE_RUN_TIMEOUT_MS` and `OPENCODE_STREAM_BUDGET_MS`.
- Determinism and verification: how emulator actions could be tested reproducibly.
- Security and safety: device isolation, capability scoping, and preventing data exfiltration from captured screens.

## Non-goals

- Committing to implement the capability now.
- Betting or bankroll logic (owned by `BET-001` on the `acebets` board).
- Any change to the product runtime in this card.

## Done when

- A written recommendation (build / do not build / build later) with a tool-shape sketch, risks, and an effort estimate.
- If build/later: a concrete follow-up feature card is filed; if adopted, findings are captured in a wiki/spec doc.

## Recommended verification

- Read-only research; no runtime changes to verify.

## Related Items

- `BET-001` Autonomous smartphone-emulator DraftKings betting project (moved to the `acebets` board; source of the idea)
- `FEAT-065` Explore Feynman research-agent uses and decide whether to adopt into ort (comparable adoption-research card)

## Notes

- Research-only; stop at a decision and a follow-up card rather than starting implementation.

---
id: "BET-001"
title: "Autonomous smartphone-emulator DraftKings betting project"
priority: "P3"
type: "epic"
area: "BET"
spec: ""
status: "todo"
created: "2026-09-26"
---

# Autonomous smartphone-emulator DraftKings betting project

## Summary

Stand up a standalone project that drives an Android smartphone emulator to autonomously explore information and place bets on DraftKings: an agent loop that observes the emulator UI, gathers and reasons over research, forms a bankroll-aware wager decision, and executes it through the app UI.

This is a tracking epic for an independent project, not part of the `opencode-modded-rust` product. It lives on this board at user request.

## Why this exists

The user wants an autonomous betting agent that operates a real mobile sportsbook surface rather than an API, using a smartphone emulator as the controlled device. It is a distinct product/project with its own stack, so it is tracked here as an epic while remaining separate from the opencode product roadmap.

## Scope

- Emulator control layer: boot/attach an Android emulator, capture screen/UI tree, inject taps, swipes, and text, launch and navigate the DraftKings app.
- Perception layer: turn raw screenshots/UI trees into structured observations (markets, odds, line movement, account/bankroll state, bet slip).
- Information-gathering layer: autonomously search and read external sources (news, injuries, stats, weather, odds comparison) before betting.
- Decision layer: an explicit, inspectable policy mapping observations plus research to a wager decision and stake size.
- Execution layer: build and confirm bet slips in the app, capture confirmation and post-bet state.
- Bankroll and risk controls: per-bet and daily limits, stop-loss, exposure caps, and a dry-run/paper-trading mode by default.
- Guardrails and compliance: DraftKings Terms of Service and any anti-automation constraints, jurisdictional legality, account safety, and responsible-gambling limits; no circumvention of anti-bot or integrity controls.
- Observability: durable logs of observations, reasoning, decisions, and executed bets for audit and replay.

## Non-goals

- Circumventing DraftKings anti-bot, integrity, or account-protection controls, or operating in violation of its Terms of Service.
- Fully unattended real-money betting without a human-in-the-loop gate and hard limits.
- Latency arbitrage or high-frequency betting.
- Integration into the `opencode-modded-rust` product runtime (separate project; only the optional capability probe in `FEAT-066` touches the product).

## Done when

- An emulator-driven agent can observe the DraftKings app, gather research, decide, and place a bet in a controlled test/paper mode end to end.
- Bankroll/risk limits and a human-in-the-loop gate are enforced and documented.
- ToS, legal, and responsible-gambling constraints are documented and reviewed before any real-money run.
- Observation/decision/execution logs are durable and replayable.

## Recommended verification

- Paper-trading run against the emulator with a fixed scenario set and asserted decisions.
- Fault-injection checks: app crash, login expiry, network loss, and unexpected UI must halt safely, not bet blindly.
- Explicit confirmation that configured limits and the human gate block an over-limit bet.

## Related Items

- `FEAT-066` Explore smartphone-emulator control as an opencode-agent capability

## Notes

- Standalone project; not part of the product roadmap and not gated by product invariants.
- Compliance, ToS, and responsible-gambling guardrails are first-class scope, not follow-up polish.
- Child cards are not created yet; this epic defines the workstreams above and can be split into feature/bug cards when work starts.

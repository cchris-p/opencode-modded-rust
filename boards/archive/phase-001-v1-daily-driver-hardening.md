---
id: "PHASE-001"
title: "V1 daily-driver hardening"
priority: "P1"
type: "epic"
area: "PHASE"
spec: "wiki/v1.md"
status: "archived"
created: "2026-09-10"
---

# V1 daily-driver hardening

## Archived

Archived 2026-09-21 at user request; phase-level tracking is no longer needed because the tool is sufficient for the current daily-driver workflow.

## Summary

Make the narrow V1 workflow trustworthy end to end: the agent can inspect a repository, use tools reliably, and produce correct, trustable output on a real daily-driver session without wasting turns on broken tools or misleading results.

This phase groups the correctness/quality work that sits on top of the now-working agentic loop (`BUG-004`/`BUG-005`/`BUG-006`).

## Why this exists

The agentic loop now works (agent prompt + environment + tools attached; tool calls execute; reasoning round-trips). The next step for V1 is not more features but making the tools and context behavior reliable enough to trust daily. The BUG-006 QA session exposed several such gaps.

## Scope

- Fix the tool-quality defects found in real sessions: `BUG-007`, `BUG-008`, `BUG-009`.
- Make model/provider capability handling explicit: `FEAT-013`.
- Ensure approvals/questions are handled cleanly in the TUI: `START-018`.
- Verify the V1 narrow workflow (`wiki/v1.md`) end to end on the daily-driver path.

## Done when

- The V1 narrow workflow completes without agent-visible tool failures on this repository.
- Tool output (listings, batch, reads) is correct and context-efficient.
- The grouped child cards are complete and verified in the TUI.

## Related Items

- `BUG-007` ls tool returns partial/misleading directory listings - **merged** into `development` via PR #31 (`bda8afc`); in `qa`, awaiting user QA.
- `BUG-008` batch tool unusable on the session path - **merged** into `development` via PR #32 (`c56287f`, tool removed); in `qa`, awaiting user QA.
- `BUG-009` Instruction files re-injected on every read - **deferred** (2026-09-15); remains in `todo`.
- `FEAT-013` Model capability gating and deprecated-default-model surfacing - open; the remaining executable child.
- `START-018` Complete TUI approval and question handling - **done** (prerequisite, not open work).
- `BUG-004` Coding sessions run as bare chat (completed prerequisite)
- `BUG-006` DeepSeek tool loop (completed prerequisite)

## Notes

- Child cards remain in their own lanes; this card is the grouping/tracking parent.
- Use `wiki/v1.md` for the workflow definition and `invariants/coding-session-behavior.md` for the binding rules.
- Reconciled (2026-09-15): the `Scope`/`Done when` text still names `START-018` as open work, but START-018 is already in `done`; treat it as a prerequisite, not a remaining task. `BUG-007`/`BUG-008` are merged into `development` and only await QA. `BUG-009` is deferred until a small-context model shows instruction-injection pressure. The only open executable phase child is `FEAT-013`.

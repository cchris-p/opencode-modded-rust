---
id: "EPIC-001"
title: "DeepSeek default-model reliability hardening"
priority: "P1"
type: "epic"
area: "EPIC"
spec: "invariants/coding-session-behavior.md"
status: "refinement"
created: "2026-10-07"
---

# DeepSeek default-model reliability hardening

## Summary

The product-owned default model is `deepseek/deepseek-flash`
(`crates/opencode-config/src/loader.rs`). It is the daily-driver path, and it has produced a recurring
class of reliability defects that keep resurfacing in new shapes even after individual fixes land.
This epic consolidates that class, records a remediation strategy, and proposes the concrete work
items that should follow. It is a coordination/tracking card, not a single code change.

## Why this exists

- The default path is DeepSeek, so every DeepSeek defect is a first-prompt defect for new users.
- Fixes so far have been **reactive and one-symptom-at-a-time**: `BUG-006` fixed reasoning passback
  and split tool calls, `BUG-038` added a stream idle timeout, `BUG-027` addressed the thinking-mode
  freeze, `BUG-034` repaired default-model resolution. Each landed, yet stalls/loops recur in
  different forms (`BUG-016`, `BUG-023`, `BUG-028`, `BUG-043`, `BUG-046`).
- Many defects were closed with **no live reproduction** because the headless QA environment cannot
  run a real `ort`/DeepSeek session (see the verification notes on `BUG-038`, `BUG-034`, `BUG-023`).
  That leaves a standing "fixed but unconfirmed on the model" gap.
- The failures cluster around a few layers, but there is no single place that tracks the whole class,
  so triage starts from scratch each time.

## Known issue classes (evidence)

1. **Stream termination / liveness.** A turn emits reasoning and then stops without a terminal
   record: `BUG-038` (mid-turn stall), `BUG-027` (thinking-mode freeze), `BUG-043` (run ends with no
   terminal state and is uninterruptible), `BUG-046` (stream with no idle gaps is unbounded).
2. **Tool-call / request protocol.** OpenAI-compatible chat shape issues that DeepSeek exercises:
   `BUG-006` (reasoning_content passback + split tool-call delta), `BUG-028` (recurring 400 when a
   `tool_calls` turn has no matching tool messages), `BUG-016`/`BUG-023` (plan-mode stall after tool
   calls with no results), `BUG-048` (bash-permission arity panic kills the run).
3. **Reasoning/thinking surface.** `BUG-022` (/thinking shows a line count, not content),
   `BUG-049` (long-session lag), `BUG-040` (grep blocks the async runtime and wedges the UI).
4. **Model registry / resolution.** `BUG-034` (default model filtered as deprecated after catalog
   refresh). Fixed, but shows how provider-catalog churn can break the default path.
5. **Persistence / continuation.** `BUG-047` (turn progress not persisted until run completes),
   `BUG-044` (continuation resumes from a stale prompt, not the latest tool call).

## Proposed approach

Four options, roughly independent. They are complementary; the recommendation is to sequence them,
not pick one.

### Option A (recommended first): Deterministic DeepSeek protocol fixture harness

Turn each historical defect into an offline regression test.

- Capture and store real DeepSeek SSE byte streams (or byte-accurate reconstructions) for:
  reasoning-only, reasoning -> tool call -> tool result -> final text, split tool-call deltas,
  argument-fragment deltas, idle gap, body-closed-without-finish, and reasoning-only-repeat.
- Drive `opencode-provider` (`stream.rs`, `deepseek.rs`, `openai_chat.rs`) and
  `opencode-session` (`prompt.rs` loop) against those fixtures with no network.
- Assert every fixture ends in a known terminal state (finish reason, usage, provider/model, error),
  never a silent `active`.

Why first: cheap, deterministic, runs in the headless environment, and it converts every past bug
into a permanent regression net. It directly closes the "fixed but never re-verified live" gap.

### Option B: Enforce a terminal-state / liveness invariant

- Make "every run reaches a known terminal state" an explicit invariant in
  `invariants/coding-session-behavior.md` and enforce it at the session loop
  (`crates/opencode-session/src/prompt.rs`): dispatch, retry, or surface an error and clear
  in-progress. This generalizes the `BUG-043` terminal-state work.
- Add a layer tag to each defect (transport/stream parser, session loop, TUI render, persistence) so
  new reports route to the right owner instead of being re-diagnosed from zero.

### Option C: Live validation gate for DeepSeek-affected changes

- Extend `scripts/qa/stream-smoke.sh` (built by `QA-001`, env-gated on `DEEPSEEK_API_KEY`) to cover
  reasoning + tool loop + interrupt/resume + a longer multi-turn session.
- Require it to pass before merging PRs that touch the provider/stream/session loop, so live
  confirmation is no longer optional. Where it cannot run, the card must say so explicitly.

### Option D: Model-capability gating and fallback

- Land `FEAT-013` (model capability gating / deprecated-model surfacing) so a catalog change cannot
  silently break the default the way `BUG-034` did.
- Consider per-model behavior config and a fallback-model path so the product is not fully hostage to
  one provider's streaming quirks. This is a product-direction decision, not just a bug fix.

## Recommended sequencing

1. Option A - fixture harness (unblocks deterministic QA for everything else).
2. Option B - terminal-state invariant + layer triage (prevents silent recurrence).
3. Option C - live gate on DeepSeek-affected PRs.
4. Option D - capability gating + fallback (follow-up, product decision).

## Locked plan (2026-10-07)

Approved direction: sequence A -> B -> C -> D as above; begin with Option A. Child work items are split
and tracked individually rather than folded into this epic.

- `QA-002` Deterministic DeepSeek stream/loop fixture harness (Option A) - **doing**.
- Option B child (terminal-state invariant + layer tags) - to be split after `QA-002`.
- Option C child (extend `stream-smoke.sh`) - to be split after `QA-002`.
- Option D child (capability gating + fallback decision) - to be split after `QA-002`.

## Open questions

- Which failures are genuinely DeepSeek-specific vs. shared OpenAI-compatible chat path (many apply
  to OpenRouter/Groq/etc. too)? The fixture harness should reveal the provider-agnostic subset.
- Is the recurring stall a loop-side defect (missing terminal record) or a provider-stream defect
  (no finish reason)? `BUG-038` bounded the wait but did not prove which layer owns it. Do not
  re-litigate; make the fixture harness answer it.
- Does the product want to commit to DeepSeek as the default, or keep the default swappable? This
  gates Option D.

## Done when

- Every known DeepSeek issue class above is either fixed with an offline regression test or
  explicitly deferred with a recorded reason.
- A run on `deepseek/deepseek-flash` can no longer end without a terminal state (or a visible error)
  across reasoning, tool-loop, interrupt, and long-session paths.
- New DeepSeek defect reports are routed to a layer and reproduced offline before a fix is attempted.
- The live smoke gate covers reasoning + tools + interrupt and is required for DeepSeek-affected PRs.

## Related items

- `BUG-006` DeepSeek reasoning passback and split tool call (done).
- `BUG-038` DeepSeek reasoning turn stalls mid-turn (done).
- `BUG-027` Session freezes during thinking mode (done).
- `BUG-034` Default DeepSeek model missing from provider registry (done).
- `BUG-028` Recurring 400 when a tool_calls turn has no tool messages (done).
- `BUG-016` Plan-mode session stalls after tool calls without results (qa).
- `BUG-023` Root-cause plan-mode tool-call stall (qa).
- `BUG-022` /thinking shows a line count, not content (qa).
- `BUG-043` Run can end without a terminal state (done).
- `BUG-046` Provider stream without idle gaps is unbounded (done).
- `BUG-047` Turn progress not persisted until run completes (done).
- `BUG-044` Continuation resumes from a stale prompt (qa).
- `BUG-048` Bash permission arity panics on short command nodes (qa).
- `BUG-049` Long-session lag / windowed render (qa).
- `FEAT-013` Model capability gating and deprecated-model surfacing (todo).
- `QA-001` Repeatable debug/QA verification suite (done) - owns `scripts/qa/stream-smoke.sh`.

## Notes

- Renumber/re-ID this card if the board adopts a different prefix for umbrella epics; existing epics
  use the `PHASE-` area (`boards/archive/phase-001..004-*.md`) and do not include `DeepSeek`.
- Keep the child work items in their own lanes/cards; this epic should not become an implementation
  dumping ground.
- `bd` could not run in this environment (`ModuleNotFoundError: No module named 'rich'` / `boards`),
  so this card was written directly in the board format. Re-check with `bd --check-duplicates` once
  the plugin environment is available.

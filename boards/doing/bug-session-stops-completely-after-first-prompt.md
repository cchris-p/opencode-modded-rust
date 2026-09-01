---
id: "BUG-003"
title: "BUG: Session stops completely after first prompt"
priority: "P1"
type: "bug"
area: "BUG"
spec: "wiki/v1.md"
status: "doing"
created: "2026-09-01"
---

# BUG: Session stops completely after first prompt

## Summary

The current product still has a session-blocking bug where a session completely stops after the first prompt instead of continuing through a normal request-response loop. Treat this as a primary workflow blocker until the exact failure point is identified and the same session flow is proven stable across Codex/OpenAI, Ollama, and Anthropic.

## Reported behavior

- The issue is currently reported as always reproducible.
- The session stops entirely after the first prompt.
- The bug has been observed on Codex/OpenAI, Ollama, and Anthropic.
- The current report does not yet pin down whether the backend session actually stops or whether only the TUI stops updating, so investigation must distinguish those cases explicitly.

## Why this exists

The product is currently being shaped as a personal daily-driver on a narrow workflow. A session that halts after one prompt breaks the core loop regardless of provider, model quality, or later UX polish. This bug therefore blocks confidence in the runtime as a serious everyday tool.

## Investigation goals

- Reproduce the failure on the current Rust product with a minimal prompt.
- Determine whether the stop happens in provider execution, runtime stage progression, server event streaming, session persistence, or only the TUI refresh path.
- Determine whether the bug is truly provider-agnostic or whether the same symptom has different causes on Codex/OpenAI, Ollama, and Anthropic.
- Identify whether the first prompt fully completes and the second prompt fails, or whether the very first prompt itself stalls before normal completion.
- Capture the exact user-visible symptom, any logs, session state transitions, and whether the server remains healthy after the stop.

## Scope

- Investigate the active session loop end to end: TUI, client API layer, server routes, session runtime, and provider execution boundaries.
- Reproduce the issue against Codex/OpenAI, Ollama, and Anthropic using the same or equivalent simple prompt where practical.
- Verify whether the bug affects only the reused local TUI server path or also a freshly started server.
- Verify whether the bug depends on a specific model, auth path, provider configuration source, or question/approval flow.
- Keep the first pass focused on root cause and the smallest correct fix.
- Do not broaden this item into general provider setup cleanup unless investigation proves the bug is caused by a provider configuration defect already tracked elsewhere.

## Failure boundaries to check

- Does the first prompt receive a complete assistant response, then later prompts fail?
- Does the first prompt itself stop before completion?
- Does the TUI stop accepting input, or does it accept input while no new work is executed?
- Does the server still emit session updates after the visible stop?
- Does the stored session status change to a blocked, error, or completed state unexpectedly?
- Does the issue reproduce only when `ort` reuses an existing local TUI server?
- Does the same session recover after restarting the app or reopening the session?

## Done when

- The root cause is identified clearly enough to explain why the session stops after the first prompt.
- The implemented fix addresses the actual failure point rather than masking the symptom.
- A session can handle repeated prompts without halting unexpectedly.
- The fix is verified locally against Codex/OpenAI, Ollama, and Anthropic.
- Verification explicitly confirms that the issue is no longer present for each provider path, not just for one successful provider.
- Any provider-specific residual gaps discovered during investigation are split into separate follow-up board items instead of being hidden inside this bug.

## Required verification

- Reproduce the bug before the fix with a minimal prompt and capture the exact visible failure.
- Test a fresh-launch path and, if relevant, a reused-local-server path.
- After the fix, verify at least one stable multi-prompt session with Codex/OpenAI.
- After the fix, verify at least one stable multi-prompt session with Ollama.
- After the fix, verify at least one stable multi-prompt session with Anthropic.
- For each provider above, explicitly have the user confirm that the session no longer stops after the first prompt.
- Record any provider/model-specific caveats discovered during verification.

## Suggested verification script

- Start a new session.
- Send a minimal first prompt such as `reply with exactly OK`.
- Wait for completion and confirm the session remains healthy.
- Send a second prompt such as `reply with exactly STILL OK`.
- Send a third prompt such as `summarize the last two answers in one line`.
- Repeat the same pattern for Codex/OpenAI, Ollama, and Anthropic.
- Note whether failure occurs on the first prompt, after first completion, or only on subsequent prompts.

## User validation requirement

Once fixed, have the user test all three provider paths personally:

- Codex/OpenAI
- Ollama
- Anthropic

The bug should not be considered closed until the user confirms the session no longer stops after the first prompt on all requested provider paths, or any remaining provider-specific failures are broken out into separate tracked bugs.

## Questions for the user

- Keep this section in the card even if the bug is not investigated immediately. These answers should be captured before implementation starts if they are still unknown.

- What are the exact steps you use when this happens?
- Does the first prompt fully finish, or does it stall mid-response?
- What exact text, status message, spinner state, or error do you see when the stop happens?
- Does the TUI freeze, or can you still interact with it after the session stops?
- If you reopen the same session, does it remain stuck or resume?
- Which exact model did you use for Codex/OpenAI when you saw it?
- Which exact Ollama model and endpoint did you use when you saw it?
- Which exact Anthropic model did you use when you saw it?
- Does the issue happen both on a fresh `ort` launch and when `ort` reuses an existing local TUI server?
- Did this start after a specific recent change, or has it been present the whole time you've been testing this path?

## Related Items

- `START-004` Assess current Rust state
- `START-005` Define V1 runtime loop
- `START-015` Mirror OpenAI auth configuration in settings
- `START-018` Complete TUI approval and question handling
- `START-019` Add native Ollama support for the local-model-first V1 path
- `FEAT-002` Keep sessions running after TUI exit

## Notes

- Treat this as a real runtime blocker until disproven, not as a minor UX glitch.
- Investigation should prefer evidence from real runs, session state, and logs over speculative fixes.
- If the bug turns out to be a reused-server state issue, document that explicitly and verify both reused and fresh-launch behavior after the fix.

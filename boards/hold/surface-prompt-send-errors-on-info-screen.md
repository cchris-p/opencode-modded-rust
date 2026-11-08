---
id: "BUG-035"
title: "Prompt send failures should surface full errors on Info"
priority: "P1"
type: "bug"
area: "BUG"
spec: "docs/opencode-session.md"
status: "hold"
created: "2026-09-23"
---

# Prompt send failures should surface full errors on Info

## Summary

When prompt submission fails, the TUI currently shows a clipped alert beginning with `Failed to send prompt to ...`. The alert is too small/transient to show the actionable HTTP status and server error body, so the user sees an unhelpful truncated message instead of the root cause.

Concrete observed error:

`Failed to send prompt to http://127.0.0.1:3187/session/ses_fd6cf4ebaa6e4123a381579f83e66847/prompt: 400 Bad Request - {"error":{"message":"Model `deepseek-v4-flash` not found for provider `deepseek`","type":"bad_request"}}`

The important part is the server message: `Model `deepseek-v4-flash` not found for provider `deepseek``. That should be recoverable from a durable or scrollable TUI surface even if the immediate alert is short.

## Scope

- Preserve the existing immediate alert/toast behavior for prompt-send failures.
- Add or route prompt-send failure details to the TUI Info screen or equivalent persistent diagnostic surface.
- Include the full server error body and enough request context to identify the failing session/model/provider path.

## Non-goals

- Fixing the specific missing default DeepSeek model; `BUG-034` owns that root cause.
- Reworking every alert/dialog in the TUI.
- Adding verbose logs for successful prompt sends.

## Acceptance Criteria

- When `ApiClient::send_prompt` returns a non-2xx response, the TUI records the full error in Info.
- The Info surface includes the HTTP status and server error message/body without clipping the actionable cause.
- The immediate prompt-send alert can remain concise, but it should point the user to Info when details are available.
- This behavior covers both first-prompt-from-Home failures and failures in an existing session.

## Verification

- Force a prompt-send 400 response and confirm Info displays the full server error.
- Confirm the TUI still shows an immediate failure notification.
- Confirm the Info entry is not lost when the prompt failure occurs before server prompt acceptance.

## Related Items

- `BUG-034` Default DeepSeek model is missing from provider registry.
- `BUG-033` First prompt failure leaves an exportable empty new session.
- `BUG-025` Concurrent server sync deletes sessions and messages: related class of swallowed or misleading session errors.
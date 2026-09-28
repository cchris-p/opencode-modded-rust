---
id: "BUG-053"
title: "Assistant turn footer shows the fallback \"Default\" instead of the active agent (build)"
priority: "P2"
type: "bug"
area: "BUG"
spec: ""
status: "todo"
created: "2026-09-27"
---

# Assistant turn footer shows the fallback "Default" instead of the active agent (build)

## Summary

The TUI assistant turn footer (`▣ <mode> · <model> · <duration>`) sometimes renders
`Default` where it normally renders `Build`. "Default"/"Build" is not a user-selected
mode: the footer label is the assistant message's `mode` metadata, which the session
runner sets to the **agent name**. When that metadata is absent the TUI falls back to
the literal string `default` (`crates/opencode-tui/src/components/session.rs:2108`,
rendered via `titlecase` at `:2118`).

The user wants the footer to stay on the active agent (`Build`) rather than showing the
fallback, since the default mode is already the intended behavior and the flip reads as
a state change that did not happen.

## Evidence

- Footer source: `crates/opencode-tui/src/components/session.rs:2088-2149`
  (`assistant_footer`); the label comes from `message.mode.as_deref().unwrap_or("default")`
  at `:2108` and is title-cased at `:2118`.
- `mode` is populated from the agent name only when a name is present:
  `crates/opencode-session/src/prompt.rs:1309-1311` inserts both `agent` and `mode`
  from `agent_name` inside `if let Some(agent) = agent_name`.
- The normal prompt path always resolves a concrete agent name:
  `crates/opencode-server/src/routes.rs:3944` (`resolve_agentic_context`) sets
  `resolved.agent_name`, which is defaulted to `build` by
  `crates/opencode-server/src/agentic.rs:49-53,103`; the run passes
  `agent: Some(resolved.agent_name.clone())` (`routes.rs:4459`).
- The real default agent is `build` (`crates/opencode-agent/src/agent.rs:188-214`,
  `default_agent()`); `Default` is only the TUI's missing-metadata fallback.
- Candidate sources of assistant messages that never set `mode`:
  - `crates/opencode-server/src/routes.rs:4811` (`execute_shell`) and `:1965`
    (`send_message`) create assistant messages with no `agent`/`mode`.
  - Error/abort paths create an assistant message and may set `agent` but not `mode`
    (`routes.rs:4430-4437`, `:4486-4505`).
  - `crates/opencode-session/src/prompt.rs:2249-2327` (`process_response`, non-stream
    helper) builds an assistant message whose metadata omits `agent`/`mode`.

## Why this matters

- `Default` is indistinguishable from a real mode the user could select, so it looks
  like the agent silently changed mid-session.
- It also makes the transcript/export inconsistent: `assistant_header` title-cases the
  actual `agent` metadata (`crates/opencode-tui/src/app/app.rs:4683-4717`), so the live
  footer and the exported header can disagree about the same turn.

## Scope

- Determine every assistant-message creation path that can omit `agent`/`mode`.
- Ensure an assistant turn that ran under an agent always carries that agent's name, so
  the footer and `assistant_header` read the same value.
- Decide the correct fallback when the metadata is genuinely unknown: show the active
  agent (`build`) rather than a synthetic `Default`, or hide the label, rather than
  inventing a mode name.
- Keep the footer label sourced from the real `mode`/`agent` metadata; do not string-sniff
  or hard-code `Build`.

## Non-goals

- Changing which agent is default, or any agent-selection behavior; the default
  (`build`) is already correct.
- Restyling the assistant footer beyond the label value.
- Changing the `/agent` picker or `Tab`/`Shift+Tab` agent cycling.

## Done when

- Running any normal turn (prompt, skill invocation, `!` shell, errored turn, resumed
  turn) shows the active agent name in the footer, never `Default`, unless the agent is
  genuinely unknown — in which case the fallback is deliberate and documented.
- The live footer label matches the exported transcript header for the same turn.
- A regression test covers an assistant message with missing `mode` metadata and asserts
  the intended label.

## Recommended verification

- `cargo test -p opencode-tui components::session` (footer label tests).
- Add a unit test around `assistant_footer` for `mode: None` and `mode: Some("build")`.
- Manual: `ort-build` then `ort`; run a normal prompt, a `/name` skill invocation, and a
  `!` shell command, and confirm the footer stays on `Build` in each case.

## Related Items

- `BUG-051` Continuation prompt flashes / unordered (adjacent session-footer rendering
  work on `development`).
- `BUG-047` Assistant turn progress is not persisted until the run completes (message
  metadata durability; the `mode` key rides the same metadata map).
- `FEAT-049` Agent-role and `@agent-name` mention parity (agent identity on turns).
- `SKILLS-006` Surface skills in the command menu (running a skill is one path that
  produces a turn whose footer label should stay `Build`).

## Notes

- Reported by the user 2026-09-27: "what is mode default? I keep seeing it switch from
  Build to Default when running commands."
- `Default` is not a selectable agent; it is `titlecase` of the `unwrap_or("default")`
  fallback for missing `mode` metadata. The active agent itself is `build`.

---
id: "FEAT-027"
title: "Add a permanent-allow permission option that writes project-local config"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "todo"
created: "2026-09-19"
---

# Add a permanent-allow permission option that writes project-local config

## Summary

Add a fourth choice to the TUI permission box — **Permanently allow** — with its own dedicated keyboard shortcut. Choosing it approves the current request and writes a matching permission rule into the workspace's project-local config so the grant survives restarts and travels with the repository to other machines.

## Why this exists

Today the permission box offers three choices:

- Allow (`y`/Enter) — approves only this one request.
- Deny (`n`/Esc) — rejects the request.
- Always allow (`a`) — approves matching requests for the current session only.

The `Always allow` approval is in-memory and session-scoped: it is stored in `PermissionEngine.approved` keyed by session id (`crates/opencode-permission/src/engine.rs:45-48`) and dropped by `clear_session` (`engine.rs:194-197`). Nothing is written to disk. So a repeated, deliberate approval such as "let this repo run `cargo test`" or "allow reads under this external path" has to be re-granted in every new session and can never be shared.

Permission rules are already a first-class project-config concept (`crates/opencode-config/src/schema.rs:613-637`), and the config loader already reads project `opencode.json{,c}` and `.opencode/opencode.json{,c}`. The missing piece is a durable, reviewable way to create that rule from the permission prompt itself.

Persisting to **project-local** config (inside the working directory) is the point: the file is checked into the same repository, so permissions set on one machine apply to every machine that clones the repo. This is deliberate sharing of a workflow contract, not a personal machine preference.

## What this means right now

- The permission box gains a fourth option, `Permanently allow`, with its own keyboard shortcut (proposed: `p`).
- Selecting it (a) answers the pending request as an allow and (b) merges a matching rule into the workspace's project-local OpenCode config.
- The rule written is the same permission/pattern pair the request was evaluated with, encoded in the existing config permission shape (for example `{ "permission": { "bash": { "cargo test": "allow" } } }`).
- The change is written to the project directory that the running server treats as the workspace, so it is relative to the repo, not the user's home.
- The TUI reports where the rule was written so the user can review and commit it.

## Scope

- Add a fourth `PermissionAction` (for example `ApprovePermanent`) to the TUI prompt model (`crates/opencode-tui/src/components/permission.rs:80-85`).
- Add the dedicated keyboard shortcut to the permission key handler (`crates/opencode-tui/src/app/app.rs:343-371`); must not collide with `y`, `n`, `a`, or `Esc`.
- Render the new option in the permission box hint row (`crates/opencode-tui/src/components/permission.rs:250-254`) and fix the mouse hit-testing so a fourth button is addressable — the current `handle_click` has three hardcoded column ranges and an `else` that swallows everything after the third (`permission.rs:184-197`).
- Extend the reply contract end to end: the TUI client (`crates/opencode-tui/src/api.rs:794-822`) and the server route must accept the new reply value. The server currently whitelists only `once`, `always`, and `reject` (`crates/opencode-server/src/routes.rs:3506-3513`).
- Persist to project-local config from the server side, where the workspace directory is known and config helpers live. Reuse or extend `update_config(project_dir, patch)` (`crates/opencode-config/src/loader.rs:1467-1487`) rather than writing JSON by hand in the TUI.
- Merge, never replace: adding a permanent rule must preserve all unrelated config keys and must not duplicate or clobber an existing rule for the same permission/pattern.
- Derive the stored rule from the pending request's permission type and normalized pattern so the grant matches the action the user actually approved (use the existing normalization in `crates/opencode-permission/src/ruleset.rs:91-101`, including `external_directory` boundary handling).
- Apply the grant to the current session as well, so the immediate request is approved and the same tool does not immediately prompt again.
- Surface the written file path (and the rule) in user-visible feedback (toast or equivalent).
- Keep the grant scoped to the workspace project config; do not write global config in this feature.

## Non-goals

- Writing to global/user config (`~/.config/opencode/opencode.json`), an enterprise managed config, or any location outside the workspace.
- Auto-committing or auto-staging the config file in git; writing the file is enough, the user owns the commit.
- A permission-management UI for reviewing or removing rules.
- Changing the default action for any permission; `Permanently allow` is opt-in per request.
- Reworking the existing `Always allow` session-scoped semantics.
- Broadening a specific approval into a blanket `"*"` grant automatically.

## Done when

- The permission box shows four options and the new `Permanently allow` option has a dedicated, documented shortcut.
- Pressing the shortcut approves the current request and writes a matching rule into the workspace's project-local config.
- The written config preserves unrelated keys and does not duplicate an identical existing rule.
- Re-launching a fresh session in the same workspace auto-approves matching requests without prompting, proving the grant was loaded from project config.
- The same repository on a second machine (or after deleting in-memory state) honors the committed grant.
- The user-visible feedback names the config file that changed.
- `Always allow` still only affects the current session and writes nothing to disk.

## Recommended verification

- Run `ort-build`, then launch `ort` in a scratch project that has a project-local `opencode.jsonc` with unrelated keys.
- Trigger an approval-gated action (for example a non-allowlisted bash command), choose `Permanently allow`, and confirm: the action runs, a toast/log names the config path, and the file gained the expected permission rule while unrelated keys are intact.
- Open a new session in the same workspace and repeat the same action; confirm it runs without prompting.
- Restart the TUI/server entirely and repeat; confirm it still runs without prompting.
- Commit the config file, clone the repo to a second location (or reset local state), and confirm the grant is honored there.
- Repeat the same approval twice and confirm the config keeps a single rule rather than accumulating duplicates.
- Select `Always allow` and confirm no file is written and a new session prompts again.
- Confirm `Deny` behavior and all existing shortcuts still work.
- Add unit tests for the config-merge path (preserve unrelated keys, idempotent on repeat) and for the reply-contract validation.

## TBD

- Exact shortcut: `p` (proposed, mnemonic for "permanent") vs `A`. Must be free of collisions and unambiguous in the hint row.
- Which project-local file to target when more than one exists: the repo already carries `opencode.jsonc`, while `update_config` writes `opencode.json`. Ideally merge into the file the loader actually resolves, and preserve JSONC comments; if comment-preserving merge is too costly for the first pass, document that the write targets a plain `opencode.json` and does not reformat an existing `.jsonc`.
- Whether the rule should ever be broadened (for example offering a `*` variant) or always stay exact to the approved pattern. Default: exact to the approved resource.
- Whether the server should hot-reload the ruleset after writing so the new rule applies to the current run, or whether applying the session approval plus next-request evaluation is sufficient.
- How to represent a permanent grant for permissions that only carry a coarse type (no resource), e.g. `*` patterns.
- Whether writing project config should itself be approval-gated or reversible with an undo affordance.

## Related Items

- `BUG-004` disable-tui-sidebar-by-default's sibling work established the agent/session permission overlay; the server ask callback and merged ruleset live at `crates/opencode-server/src/routes.rs:1811-1849` and `crates/opencode-server/src/agentic.rs:203-209`.
- `FEAT-022` Persist session workspace identity
- `FEAT-023` Filter session list and load by workspace
- `PHASE-001` V1 daily-driver hardening

## Notes

- This is a durable, shareable grant. Keep the default narrow (exact approved pattern), show what will be written, and require a dedicated key rather than reusing `a` or Enter.
- Project-local is the whole point: the config lives with the repo so `git commit` propagates permissions across machines. Do not fall back to global config.
- Reuse the existing config schema, loader search order, and permission normalization instead of introducing a parallel representation.
- Server owns the write because it knows the workspace directory and already links `update_config` for project config patches (`crates/opencode-server/src/routes.rs:2691-2698`).

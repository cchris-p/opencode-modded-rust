---
id: "FEAT-027"
title: "Add a permanent-allow permission option that writes project-local config"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "done"
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

## Implementation Notes

- Shortcut is `p`. The rule writes to the workspace's plain `opencode.json` via `update_config`; an existing `opencode.jsonc` is left untouched (load order reads `.jsonc` then `.json`, so the new rule wins). Comment-preserving JSONC writes remain deferred.
- `PermissionConfig` deep-merge was made pattern-aware (`crates/opencode-config/src/schema.rs`): merging rules under the same permission key now unions sibling patterns instead of replacing the whole rule. This is what makes a permanent grant preserve unrelated patterns and stay idempotent on repeat.
- TUI: `PermissionAction::ApprovePermanent`, `p` handler, a fourth hint-row button, and mouse hit-testing fixed to four column ranges (`crates/opencode-tui/src/components/permission.rs`, `crates/opencode-tui/src/app/app.rs`). The reply value is `permanent`.
- Server (`crates/opencode-server/src/routes.rs`): accepts `permanent`, derives exact-pattern `Allow` rules from the request (using `normalize_permission_pattern`, so `external_directory` boundaries are handled), writes project config, and records the grant in an in-memory per-session overlay merged into the ask evaluation. The overlay keeps an approved request from re-prompting before the next config load; config remains the durable source of truth.
- Reply response changed from `bool` to `PermissionReplyResponse { ok, path, error }` so the TUI can toast the exact config file written and surface write failures. `path` is also included in the `permission.replied` broadcast.

## Verification

- `cargo build` (full workspace)
- `cargo test -p opencode-config` (58 unit + 5 integration, including `permission_merge_preserves_sibling_patterns_and_is_idempotent` and `test_update_config_grant_preserves_keys_and_is_idempotent`)
- `cargo test -p opencode-permission`
- `cargo test -p opencode-server --lib` (including `permission_grant_tests`)
- `cargo test -p opencode-tui --lib app::app::tests` and the permission-focused `components::prompt` checks in isolation
- Deferred to manual QA: interactive `ort-build` + `ort` run confirming the fourth button, the toast path, cross-session/config reuse, and the second-machine/repo-clone grant.

## QA Report - 2026-09-21 (partial)

Environment: workspace `/Users/cchrisleepyles/repos/opencode-modded-rust`, uncommitted FEAT-027 working tree.

User-verified: the `p` (Permanently allow) hotkey works and approves the request.

Verified by closeout QA:

- Project-local write is real: `opencode.json` exists at the workspace root (untracked, created during the `p` test), not in `~/.config`.
- The file contains the expected `permission.external_directory` grant map:
  `/private/tmp/*`, `/Users/cchrisleepyles/.config/opencode/*`, `/private/*`, and
  `/Users/cchrisleepyles/repos/opencode-modded/packages/core/src/plugin/*`, all `allow`.
- Every saved rule's target directory exists (`/private/tmp`, `/private`,
  `~/.config/opencode`, and the opencode-modded `packages/core/src/plugin` path).
- Durable load proven with a fresh process: `opencode debug config` (cwd = workspace) merges
  `opencode.json` with `opencode.jsonc`; `model: deepseek/deepseek-v4-flash` is preserved and all
  four `permission.external_directory` rules are present in the resolved config.
- Tests pass: `opencode-config` 58+5 (incl. `permission_merge_preserves_sibling_patterns_and_is_idempotent`,
  `test_update_config_grant_preserves_keys_and_is_idempotent`), `opencode-permission` 9,
  `opencode-server` `permission_grant` 3 (incl. external-directory normalization).

Deferred acceptance criteria (review later, not yet verified):

- Fresh session in the same workspace auto-approves matching requests without prompting.
- Full TUI/server restart still honors the grant.
- Committing the config and cloning to a second location honors the grant.
- Repeat `p` on the same request keeps a single rule (idempotency proven by unit test only, not live).
- Toast names the exact config file path in the live TUI.

QA concern from live use: directory permanent-allow still feels too granular.

- The TUI later asked for permission around an individual skill directory/path after the user believes
  they had already used `p` / `Permanently allow` on a directory permission prompt.
- Treat this as an `external_directory` durable-allow concern, not a skill-specific product decision:
  if a user permanently allows a directory, covered child files, child directories, and sibling skill
  directories should not prompt again.
- Code-backed expectation: `external_directory` patterns normalize to directory-glob form in
  `crates/opencode-permission/src/ruleset.rs`, and the wildcard matcher treats a trailing `*` as a
  prefix match. A saved parent rule such as `/Users/cchrisleepyles/.config/opencode/*` should cover
  `/Users/cchrisleepyles/.config/opencode/skills/<skill-name>/*`.
- Current local evidence: project-local `opencode.json` contains a broad durable grant for
  `/Users/cchrisleepyles/.config/opencode/*`, but also contains multiple narrower per-skill grants
  under `/Users/cchrisleepyles/.config/opencode/skills/.../*`. That accumulation suggests the live
  behavior may be saving or re-prompting at too narrow a boundary, or a broader saved grant was not
  loaded/applied when later prompts occurred.
- Follow-up verification: restart `ort`/server so project config is freshly loaded, trigger access to
  a different path under `/Users/cchrisleepyles/.config/opencode/skills/...`, and confirm no new
  `external_directory` prompt appears while `/Users/cchrisleepyles/.config/opencode/*` remains in
  project config. If it still prompts, investigate config load order, session rule merging, and
  request-pattern normalization for `external_directory`.

Status: remains in `qa`. The feature is committed and merged (see Merge Closeout), but the deferred
acceptance criteria above are not yet verified, so the item cannot move to `done`.

## Closeout - 2026-09-21

- User re-tested the directory permanent-allow concern from live use and reported no issue with
  permanent allow: the deferred `external_directory` granularity concern does not reproduce.
- Closed as `done`; the remaining deferred acceptance criteria are accepted as-is rather than
  blocking.

## Merge Closeout

- Committed as `f7eaba6` on `feature/FEAT-027-permanent-allow-permission`.
- Merged via PR #50 into `development` (merge commit `f0dfb34`) on 2026-09-21.
- Local and remote feature branches deleted; `development` fast-forwarded to `f0dfb34`.
- Item remains in `qa` pending the deferred acceptance-criteria review.

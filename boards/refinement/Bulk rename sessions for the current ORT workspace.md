---
id: "CLI-018"
title: "Bulk rename sessions for the current ORT workspace"
priority: "P2"
type: "feature"
area: "CLI"
spec: "README.md"
status: "refinement"
created: "2026-11-16"
updated: "2026-11-16"
---

# Bulk rename sessions for the current ORT workspace

## Summary

Provide a one-off bulk-rename path for existing ORT sessions **belonging to the workspace where
ORT is instantiated** (the current directory / workspace identity), so older sessions stop
carrying misleading first-line-prompt or `New session - <timestamp>` titles. The owner asked
whether this can be done with a temporary command.

## Answer: is it possible?

Yes. Existing titles are plain `sessions.title` rows and ORT already has a manual rename API.
There are three viable shapes:

1. **Temporary CLI subcommand (recommended).** Add a thin
   `opencode-modded-rust session rename ...` command that filters by the current workspace and
   writes a supplied mapping. It can be kept as a small admin command or removed after the
   one-off pass. It can support `--dry-run` (default) and `--apply`.
2. **One-off SQLite script (fastest, riskier).** `UPDATE sessions SET title = ?, updated_at = ?`
   for rows whose `directory` / `workspace_identity` match. This bypasses the in-memory session
   manager, so ORT servers for that workspace must be stopped first: a live server holding the
   session with a pending `title_auto` marker could overwrite the title on its next persist.
3. **Manual rename (no code).** `Ctrl+R` per session in the TUI or the session-list rename
   path. Viable for ~10 sessions today but not repeatable.

The clean route is (1) or (2) with the server stopped; the durable behaviour must also mark the
title as user-owned so a later auto-title pass never replaces it.

## Verified facts (2026-11-16)

- The current workspace (`/home/matrillo/apps/opencode-modded-rust`) has **10 sessions** in the
  ORT DB (`~/.local/share/opencode-ort/opencode.db`); all are workspace-local. Titles are a mix
  of `New session - <rfc3339>` and the first line of the user prompt (some truncated/raw).
- Sessions carry `directory` and `workspace_identity`; the server list endpoint already filters
  on both (`SessionFilter { directory, workspace_identity }`), so workspace scoping is a
  first-class concept.
- Manual rename path: TUI `ApiClient::update_session_title` -> `PATCH /session/{id}` ->
  `set_session_title` / `update_session` -> `Session::set_title` + `mark_title_user_owned`.
  That clears the `title_auto` metadata marker so generated titles cannot overwrite it.
- `sessions.title` is persisted in SQLite; the `title_auto` marker lives in session metadata
  (in-memory) and is not a `sessions` column. A DB-only rename followed by a fresh load will
  not auto-regenerate a non-default title, but a **running** server with a pending marker can
  still overwrite it before reload.
- No rename CLI exists today: `SessionCommands` has `list`, `show`, `find`, `inspect`, `delete`.

## Scope (proposed)

- Workspace-scoped only: never touch sessions from other directories/workspaces.
- Accept an explicit mapping (for example `session-id-or-slug<TAB>new title`) so the owner
  controls the final names; `--dry-run` prints the resolution table before writing.
- Resolve sessions by the current workspace (`--workspace <dir>` defaulting to `.`) using
  `directory` and, when present, `workspace_identity`; include child sessions only if
  requested.
- Apply through the API when a workspace server is running, or directly through
  `SessionRepository` with the server stopped. Mark titles user-owned in both paths.
- Verify by re-listing sessions and confirming `session list`/TUI headers show the new names.

## Open questions (owner decisions)

1. **Where do the new names come from?** Curated mapping supplied by the owner, agent-proposed
   names from each transcript for approval, or a deterministic rule (e.g. board item ID +
   short summary from the first user message)?
2. **Should the command be permanent or removed after the one-off pass?**
3. **Include child/subagent sessions**, or rename only root sessions?
4. **Run with the workspace's ORT servers stopped, or require an API/attach path?** A stopped
   server makes the DB path safe; an API path avoids stopping anything but needs a target
   server URL.

## Non-goals

- Changing automatic title generation for new sessions (`FEAT-076` owns parity).
- Cross-workspace or fleet-wide session renames.
- Renaming sessions in the vanilla OpenCode DB (separate harness/storage).
- Deleting or rewriting session transcripts/messages.

## Related Items

- `FEAT-076` Session title generation parity with vanilla OpenCode (why new titles are noisy).
- `FEAT-029` Learn how manual session rename works end to end.
- `FEAT-080` / `FEAT-081` ORT update notices (current active work in this workspace).
- `CLI-017` `ort --version` identity (session/worktree context).

## Notes

- Created 2026-11-16 at the owner's request, clarified that the scope is all sessions belonging
  to the workspace where ORT is instantiated, not all sessions on the machine.
- Feasibility confirmed read-only against the ORT DB; no data changed.

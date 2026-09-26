---
id: "BUG-050"
title: "Permanent-allow permission grants sometimes fail to write the project opencode config"
priority: "P2"
type: "bug"
area: "BUG"
spec: ""
status: "todo"
created: "2026-09-26"
---

# Permanent-allow permission grants sometimes fail to write the project opencode config

## Summary

Selecting **Permanently allow** (`p`) on a permission prompt sometimes fails to persist the grant.
The approval still applies to the current request, but the durable project-config write does not
land, and the TUI shows a "Permission approved, but saving it failed: ..." toast. The user asked to
create this bug from live use: the write to the opencode project directory is unreliable and only
fails intermittently, which points at a path/race problem rather than a permanently unwritable
location.

This is an investigation-and-fix card for the server-side write in `reply_permission`
(`crates/opencode-server/src/routes.rs:5779`) and the config helper it calls
(`crates/opencode-config/src/loader.rs:1507`). Root cause is not yet confirmed; the hypotheses and
diagnostics needed to confirm it are recorded below.

## Reported behavior

- Pressing `p` (Permanently allow) on a permission prompt sometimes shows the write-failure toast:
  "Permission approved, but saving it failed: <error>".
- It is intermittent ("sometimes"), not a consistent failure for the same workspace.
- The request itself is still approved, so only the durable save fails; the same request may prompt
  again after the in-memory session overlay is gone (e.g. after restart).
- Exact reproduction conditions, the error string shown, the workspace path, and whether an
  `opencode.json` already existed are not yet captured.

## Why this exists

**Permanently allow** is the only way a user can make a deliberate grant survive restarts and travel
with the repo (`FEAT-027`). If the write silently fails intermittently, the user believes a durable
permission is in place while the grant is not actually recorded, which erodes trust in the
permission system and forces repeated re-approval. Durable permission persistence must be at least
as reliable as the prompt that creates it.

## Code evidence

Write path on the server:

- `reply_permission` handles `"permanent"` and resolves the target directory from the session's
  stored workspace directory, falling back to the server process CWD, then to an empty path:
  `crates/opencode-server/src/routes.rs:5806-5817`.
- It calls `update_config(&target_dir, &permanent_config_patch(&permission))` and, on `Err`, sets
  `response.ok = false` / `response.error`:
  `routes.rs:5819-5832`, response struct at `routes.rs:5718-5725`.
- The TUI surfaces that error in a toast:
  `crates/opencode-tui/src/app/app.rs:3628` (`respond_to_permission`), permanent branch at
  `:3640-3655`.

The config helper:

- `update_config` joins `project_dir` with `"opencode.json"`, reads and `parse_jsonc` the existing
  file, merges, then `fs::write`s the whole file:
  `crates/opencode-config/src/loader.rs:1507-1526`.
- It does **not** create the parent directory first, unlike `update_global_config`, which calls
  `fs::create_dir_all(parent)` (`loader.rs:1540-1542`).
- It is a plain truncate-and-write; there is no temp-file-and-rename and no lock:
  `loader.rs:1522`.
- It silently treats an unparsable existing file as empty via `parse_jsonc(&content).unwrap_or_default()`:
  `loader.rs:1510-1512`.

Other writers of the same file make a race plausible:

- Provider/model persistence also calls `update_config(&cwd, ...)`:
  `crates/opencode-server/src/routes.rs:4881`; model reset rewrites the same file:
  `routes.rs:4903` -> `crates/opencode-config/src/loader.rs:1577-1594`.
- None of these writers share a lock or write atomically.

Directory-resolution context:

- Session `directory` is populated at create time from the request query, the
  `x-opencode-directory` header middleware, or the server CWD, defaulting to `"."`:
  `crates/opencode-server/src/routes.rs:574-597`, middleware at `routes.rs:88-95`.
- `current_project_dir()` is just `std::env::current_dir()`:
  `crates/opencode-server/src/routes.rs:7654-7656`.

## Hypotheses (confidence-ranked, pre-measurement)

- **H1 (medium) - Non-atomic read-modify-write races another writer.** `update_config` reads,
  merges, and truncate-writes with no lock or atomic rename. A permission save can interleave with
  a model/provider save (`routes.rs:4881`, `routes.rs:4903`) or a second rapid `p`, causing a
  truncated/partial read (`parse_jsonc` fails -> `unwrap_or_default`) or a failed write. Intermittent
  failures fit a race. This would also explain silent loss of unrelated keys.
- **H2 (medium) - Target directory is empty or nonexistent.** When the session's directory is empty
  and `current_dir()` also fails (deleted/moved CWD), the fallback yields an empty `PathBuf`
  (`routes.rs:5814-5817`), so the write targets a relative `opencode.json` in the server process
  CWD rather than the workspace. If that CWD is not writable, or the resolved workspace directory no
  longer exists, `fs::write` returns NotFound/PermissionDenied. `update_config` never creates the
  parent (`loader.rs:1540-1542` is only in the global variant).
- **H3 (low-medium) - Filesystem-level failure.** Workspace on a read-only/removable volume, disk
  full, or `opencode.json` made read-only. Possible but should be consistent, not "sometimes",
  unless it correlates with a specific workspace.
- **H4 (low) - Path is a directory or otherwise invalid.** `opencode.json` exists as a directory, or
  the workspace path contains something `fs::write` rejects.
- **H5 (low) - Existing file parse failure causes a misleading failure/loss.** If the existing
  `opencode.json` is invalid JSONC, `unwrap_or_default()` silently discards it; if the read itself
  fails midway during a concurrent write, the whole save errors. Overlaps H1.

## Diagnostics to capture (do this before fixing)

1. Capture the exact error toast text and correlate with server logs; `update_config` includes the
   path in its context string (`loader.rs:1523`), so the failing path should be visible.
2. Record `target_dir` (session directory vs `current_dir()` fallback vs empty), whether
   `opencode.json` already existed, and the workspace filesystem type.
3. Check whether a model/provider reselection or another permission `p` happened near the failure
   (tests H1).
4. Reproduce under a scratch workspace with a deliberately unwritable `opencode.json` (e.g. `chmod
   0444` or a `opencode.json` directory) to separate H1/H2 from H3/H4.
5. Confirm whether unrelated keys in `opencode.json` were lost after any failure (tests H5/H1).

## Required solution direction

- Make the durable grant write reliable and correctly targeted:
  - Never write to an empty/relative path; if no workspace directory can be resolved, fail with a
    clear error instead of writing to the server CWD.
  - Ensure the parent directory is the resolved workspace and create it only if appropriate; do not
    silently write elsewhere.
  - Write atomically (temp file in the same directory + rename) so a concurrent reader never sees a
    partial file and a failure leaves the previous file intact.
  - Preserve unrelated config keys; never let a parse failure of the existing file silently reset it.
- Serialize concurrent writers of the same `opencode.json` (at minimum, don't lose a grant when a
  model/provider save happens at the same moment).
- Keep the server-side ownership of the write; the TUI already reports `path`/`error` and does not
  need to change its contract, though the failure message should name the path and reason.

## Scope

- Fix the permanent-grant write path in `crates/opencode-server/src/routes.rs`
  (`reply_permission`) and the config write helper in `crates/opencode-config/src/loader.rs`
  (`update_config`, and any shared atomic-write helper it should use).
- Harden directory resolution so an unresolvable workspace is a clear failure, not a CWD-relative
  write.
- Add tests for: atomic/durable write, concurrent-writer safety (no lost grant, no corruption),
  unwritable/nonexistent target produces a surfaced error, and unrelated keys are preserved on a
  failed or successful write.
- Record the confirmed root cause and evidence on this card.

## Non-goals

- Changing the permission reply contract or the TUI `p` option behavior (`FEAT-027`).
- Global/user config writes (`update_global_config`); this card is about the project-local grant.
- Reworking permission evaluation or the in-memory session overlay.
- A permission-management/undo UI, or broad permission-granularity behavior (the separate
  `external_directory` concern already closed in `FEAT-027`).
- Comment-preserving JSONC rewrites; that remains deferred unless it is the actual failure cause.

## Done when

- The permanent-allow write no longer fails intermittently in the reproduced scenario, or the exact
  external cause is documented and handled with a clear, actionable error.
- A grant is written atomically and unrelated `opencode.json` keys are preserved.
- Concurrent config writes (permission grant vs model/provider save) cannot corrupt the file or drop
  a grant.
- An unresolvable or unwritable workspace directory fails with the path and reason surfaced in the
  TUI toast, never by writing to the server CWD.
- `cargo test` passes for `opencode-config`, `opencode-server`, and `opencode-tui`, plus
  `cargo fmt --all -- --check` and `cargo clippy` for the touched crates.

## Recommended verification

- Unit/regression: atomic-write test, concurrent-writer test, unwritable-target test, and
  preserve-unrelated-keys-on-failure test.
- Live: `ort-build` then `ort` in a scratch workspace; trigger an approval-gated action, press `p`,
  confirm the toast names the correct project `opencode.json`, the rule is present, and unrelated
  keys are intact.
- Repeat `p` rapidly several times and simultaneously change the model/provider; confirm no missing
  grant and no corrupt/lost keys.
- Point a session at a moved/deleted workspace and confirm a clear failure toast naming the path,
  with no file created in the server CWD.
- Restart the TUI/server and confirm the grant is loaded and matching requests do not re-prompt.

## Related Items

- `FEAT-027` Add a permanent-allow permission option that writes project-local config (done) -
  introduced this write path and the `ok/path/error` reply response; this card fixes its reliability.
- `START-026` Make permission allow/deny config deterministic (done) - config precedence and
  permission merge semantics the write must preserve.
- `FEAT-015` Preserve manually selected model and provider across sessions (qa) - another writer of
  the same project `opencode.json` (`routes.rs:4881`, `:4903`); a likely concurrency partner.
- `START-032` Close out START-032 (qa) - adjacent config/board work, low coupling.

## Notes

- 2026-09-26: Created from a live-use report that permanent-allow sometimes fails to write to the
  opencode project directory. Framed as an investigation card because the failure is intermittent;
  root cause is not confirmed. Prime suspects are the non-atomic, unserialized read-modify-write in
  `update_config` and the empty-directory fallback in `reply_permission`.
- 2026-09-26: Re-IDed from a duplicate `BUG-044` to `BUG-050`. The cluster card `BUG-044`
  (`session-continuation-resumes-from-stale-prompt-not-latest-tool-call`, qa) already owned that ID;
  this permission-persistence card is unrelated to the run terminal-state cluster.
- The current failure is fail-safe from a security standpoint (the action is still approved just for
  this request), but it is not safe from a trust standpoint: the user is told the grant is durable
  when it may not be.
- `update_global_config` already creates its parent directory (`loader.rs:1540-1542`); the project
  `update_config` deliberately or accidentally does not. Any fix should not blindly create arbitrary
  workspace directories, which is why H2 needs the path recorded first.

---
id: "CLI-018"
title: "Bulk rename ORT sessions on this machine with LLM titles"
priority: "P2"
type: "feature"
area: "CLI"
spec: "scripts/rename-ort-sessions.py"
status: "done"
created: "2026-11-16"
updated: "2026-11-17"
---

# Bulk rename ORT sessions on this machine with LLM titles

## Summary

Automate the one-off cleanup of misleading ORT session titles. A standalone script reads every
session in the machine-local ORT database, samples the user inputs for each session, asks the
configured ORT model for a short descriptive title, and writes the result back to
`sessions.title`. No manual name mapping is required.

Scope is **all sessions in the ORT database on this machine**. The earlier workspace-only idea
was dropped because this machine's ORT DB currently holds only this workspace's sessions and a
machine-wide pass is simpler.

## Implementation

- Script: `scripts/rename-ort-sessions.py` (stdlib only; removable after the one-off pass).
- Inputs: `sessions` rows plus up to 3 user messages per session (400 chars each by default)
  from the `messages` table (`data.parts[].part_type.text`).
- LLM: `opencode-modded-rust run --dir <temp> --agent build` with an ephemeral
  `OPENCODE_DATA_DIR`, so title generation uses the configured product model and never writes
  into the ORT session database. The prompt requests one JSON object mapping session id ->
  title; titles are validated and cleaned, with a deterministic fallback if the model omits an
  id.
- Safety:
  - Dry run by default; `--apply` writes.
  - SQLite backup beside the DB before applying (`--no-backup` to skip).
  - Refuses to apply while ORT `serve`/`tui` processes are running unless `--force`; forced
    applies touch `updated_at` so a stale in-memory snapshot is skipped by the server's
    newer-wins persistence rule.
  - `--proposals-out` / `--proposals-in` allow reviewing or reusing titles without another
    model call; `--workspace` remains available for a scoped pass.
- DB default: `$OPENCODE_DATA_DIR/opencode.db`, else
  `~/.local/share/opencode-ort/opencode.db`, else the vanilla DB path.

## Usage

```bash
# Review proposals (no writes)
scripts/rename-ort-sessions.py --cli <opencode-modded-rust>

# Apply, saving the proposals first
scripts/rename-ort-sessions.py --cli <opencode-modded-rust> \
  --proposals-out /tmp/ort-titles.json --apply

# Re-apply a reviewed proposal set without another model call
scripts/rename-ort-sessions.py --apply --proposals-in /tmp/ort-titles.json
```

Restart ORT to see the new titles in the session list/header.

## Verification (2026-11-16)

- Dry run on the machine's ORT DB found 10 sessions and produced usable titles, for example:
  `Fix 1800s autonomous session timeout`, `Add loading indicator for tool calls`,
  `FEAT-015 show selected model in menu`, `Transcribe TikTok video for ORT integration`,
  `Implement automated deployment pipeline to prod`.
- Applied all 10 titles with `--force` (ORT serve processes were running). Backup:
  `~/.local/share/opencode-ort/opencode.db.rename-backup-20261116-150003`.
- Re-queried the DB: all 10 new titles persisted.
- `python3 -m py_compile scripts/rename-ort-sessions.py` passes.

## PR

- https://github.com/cchris-p/opencode-modded-rust/pull/197
- Release distribution follow-up: every release now also attaches
  `rename-ort-sessions.py` and its `.sha256`, so binary-only machines can run it without a repo
  checkout (`gh release download v0.1.5 --pattern 'rename-ort-sessions.py*'`).

## Fix - 2026-11-16 (macOS DB auto-detection)

- Running the release script on macOS failed with `no such table: sessions`: the default DB
  search only knew Linux paths and picked the vanilla `~/.local/share/opencode/opencode.db`.
- The script now probes, in order: `$OPENCODE_DATA_DIR`, `$OPENCODE_ORT_DATA_DIR`,
  `~/Library/Application Support/opencode-ort` and `.../opencode` on macOS, then the
  XDG/`~/.local/share` equivalents. It only selects a candidate that actually has a `sessions`
  table and prints every candidate plus its state when none is usable.
- `--db <path>` remains the explicit override; the macOS ORT launcher path is
  `~/Library/Application Support/opencode-ort/opencode.db`.

## Fix - 2026-11-16 (provider environment and auth)

- Running from a plain shell on macOS failed with `Provider 'opencode-go' is not available for
  model 'deepseek-v4.1-flash'`: the script's `run` subprocess did not inherit the managed
  provider keys that `opencode-config` normally exports.
- A second failure (`DeepSeek ... 402 Insufficient Balance`) happened because the subprocess
  used an isolated `OPENCODE_DATA_DIR`, so the machine's `auth.json` was invisible, and the
  no-`--model` path could still resolve to the DeepSeek config/fallback instead of the
  OpenCode Go key.
- The script now:
  - loads `$OPENCODE_CONFIG_DIR/.env` into the subprocess environment (missing keys only);
  - seeds the temporary data dir from every readable `auth.json` (`opencode-ort`, `opencode`,
    and XDG data dirs), so a key set through the TUI works;
  - **explicitly pins `opencode-go/deepseek-v4.1-flash`** when `OPENCODE_API_KEY` is present in
    the env or `opencode-go` is present in `auth.json`;
  - falls through a provider chain on failure: opencode-go -> DeepSeek -> Anthropic -> OpenAI ->
    Google -> Novita, and only uses the product/config default if none of those keys exist.
- Verified on Linux with `OPENCODE_API_KEY` and `DEEPSEEK_API_KEY` unset in the parent shell:
  the dry run still resolved OpenCode Go from `auth.json` and produced titles.

## CI note (2026-11-16)

- The required `test` job can hit the repository-wide pre-existing
  `components::prompt::tests::tab_autocomplete_uses_first_candidate` flake, which also poisons
  the shared prompt test env lock and cascades into other prompt tests. Re-run the job when this
  happens; it is unrelated to this script.

## Safety fix - 2026-11-17 (ORT databases only)

- A fleet run exposed two safety gaps: the default search fell back to the generic
  `~/.local/share/opencode/opencode.db` when no `opencode-ort` DB existed, and legacy message
  payloads stored as a bare list crashed `extract_text_parts`.
- Default search is now **ORT-only**: `$OPENCODE_DATA_DIR`, `$OPENCODE_ORT_DATA_DIR`,
  `~/Library/Application Support/opencode-ort`, and `$XDG_DATA_HOME`/`~/.local/share/opencode-ort`.
  Generic OpenCode DBs are only touched with an explicit `--db`.
- `extract_text_parts` now accepts both `{"parts": [...]}` and legacy bare-list payloads.
- On `webappssub1` the generic DB was briefly selected before this fix; its 3 session titles were
  restored from the pre-change session-archive snapshot and verified identical. `matrillo` and
  `matrillosub2` had no ORT DB and were skipped with no changes.

## Fleet run - 2026-11-17

- Reachable active machines updated to ORT `0.1.5` via `ort-install`.
- Applied: `jasper` 14, `webapps` 4, `matrillosub1` 16; `matrillobase1` had already been
  applied (10). All had backups beside their ORT DB.
- No ORT DB (skipped): `webappssub1`, `matrillo`, `matrillosub2`.
- Unreachable from matrillobase1: `j8rsxy`, `moldavite`, `xp-farmer`; `webappssub2` is disabled.
- Live ORT sessions were present on `jasper` (2), `webapps` (4), and `matrillosub1` (6);
  `--force` was used and `updated_at` touched. Restart those sessions to show the new titles;
  an active session can still revert its own title on its next update.

## Known limitations

- Titles generated for sessions with no user input fall back to `Empty session <date>`.
- The current live ORT session may not be in the DB yet; it is picked up on a later run.
- A forced apply while an active session is still being used can have its title reverted by
  that server's next update; restart ORT after applying.
- New sessions still get the noisy auto-title until `FEAT-076` lands.

## Related Items

- `FEAT-076` Session title generation parity with vanilla OpenCode (future titles).
- `FEAT-029` Learn how manual session rename works end to end.
- `FEAT-080` / `FEAT-081` ORT update notices (current active work in this workspace).

## Notes

- Created 2026-11-16 at the owner's request; owner explicitly chose fully automated LLM naming
  over a manual mapping and accepted a DB script instead of a permanent command.

## QA Pass - 2026-11-17

Owner accepted the fleet rename run as the QA pass.

- ORT `0.1.5` installed on all reachable active machines.
- Applied ORT titles: `jasper` 14, `webapps` 4, `matrillosub1` 16, `matrillobase1` 10
  (earlier). Backups remain beside each ORT database.
- No ORT DB, skipped: `webappssub1`, `matrillo`, `matrillosub2`; `webappssub2` disabled.
- Unreachable from `matrillobase1`: `j8rsxy`, `moldavite`, `xp-farmer`.
- Live ORT sessions were forced (`--force`); restart them to display titles. `webapps` retained
  3/4 titles at the last check because one live session rewrote its own title.
- `webappssub1` generic vanilla DB titles were restored from the pre-change session-archive
  snapshot after the first pass; PR #205 restricts the script to ORT data dirs only.
- Release asset `rename-ort-sessions.py` in `v0.1.5` is the fixed ORT-only version.

Status: done.

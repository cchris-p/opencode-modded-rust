---
id: "CLI-018"
title: "Bulk rename ORT sessions on this machine with LLM titles"
priority: "P2"
type: "feature"
area: "CLI"
spec: "scripts/rename-ort-sessions.py"
status: "qa"
created: "2026-11-16"
updated: "2026-11-16"
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

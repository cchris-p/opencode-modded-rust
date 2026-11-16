---
id: "BUG-075"
title: "ORT force-ends long sessions at a 30-minute run budget that vanilla does not have"
priority: "P1"
type: "bug"
area: "BUG"
spec: "invariants/coding-session-behavior.md"
status: "qa"
created: "2026-11-13"
updated: "2026-11-13"
---

# ORT force-ends long sessions at a 30-minute run budget that vanilla does not have

## Summary

`ort` was ending otherwise-healthy long turns with
`run exceeded its time budget` after exactly 30 minutes (1800s). opencode
vanilla has no whole-run wall-clock cap, so the same session keeps working
there. The hard cap is an ORT-only addition and is the source of the
"1800s timeout" that makes sessions feel unusable: a run that is still making
progress is dropped and the turn is finalized as an error.

## Evidence

- Runtime source of the cap: `crates/opencode-server/src/routes.rs`
  `drain_session_queue` selected the run future against
  `run_timeout_budget()`, which defaulted `OPENCODE_RUN_TIMEOUT_MS` to
  `30 * 60 * 1000` (1,800,000 ms).
- Firing in the wild: `~/.local/share/opencode/traces/server.log`,
  `2026-11-13T08:31:55Z`:
  `ERROR opencode_server::routes: prompt run exceeded the run budget session=ses_82c503af759e488eaf0057fe9d1010d2`.
- Same error previously recorded in `BUG-063` evidence
  (`run exceeded its time budget`, `novita-ai/deepseek/deepseek-r1-0528`) and
  in `BUG-071` (a 10-tool-call run ended at exactly 30 min).
- Vanilla reference (`$HOME/repos/opencode-modded`, `dev` @ `f54ce313b`):
  `packages/opencode/src/session/*` and `packages/opencode/src/server/*` have no
  whole-run wall-clock timeout. The only bounds are per-provider fetch timeouts
  (`headerTimeout`/`chunkTimeout`, default `300_000` ms, `false` disables) in
  `packages/opencode/src/provider/provider.ts`.

## Root cause

ORT added three non-vanilla bounds:

1. a 30-minute whole-run budget (`OPENCODE_RUN_TIMEOUT_MS`, default on),
2. a 15-minute whole-step stream budget (`OPENCODE_STREAM_BUDGET_MS`, default on),
3. a 90-second stream idle timeout (vanilla's provider chunk timeout is 300s).

These were added as robustness backstops (`BUG-038`, `BUG-043`, `BUG-046`) but
changed default product behavior away from vanilla. Long autonomous or
reasoning-heavy turns are cut off even when the provider and model are healthy.

## Fix

Default ORT to one-to-one vanilla behavior, keeping the backstops as explicit
opt-ins:

- `DEFAULT_RUN_TIMEOUT_MS` is now `0` (no whole-run budget). Set
  `OPENCODE_RUN_TIMEOUT_MS` to opt into a backstop.
- `DEFAULT_STREAM_BUDGET` is now zero (no whole-step budget). Set
  `OPENCODE_STREAM_BUDGET_MS` to opt in.
- `DEFAULT_STREAM_IDLE_TIMEOUT` is now `300s`, matching vanilla's provider
  chunk timeout, and is overridable/disable-able with
  `OPENCODE_STREAM_IDLE_TIMEOUT_MS` (`0` disables). `with_idle_timeout` treats a
  zero timeout as a no-op.
- `opencode run --attach` client wait no longer defaults to a 30-minute abort.

Abort (`Esc`), panic handling, and the durable terminal-state contract are
unchanged, so a run still cannot leave a session silently `busy`.

## Verification

- `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo check -p opencode-provider -p opencode-session -p opencode-server -p opencode-cli` - clean.
- `cargo test -p opencode-provider --lib` - 155 passed.
- `cargo test -p opencode-session --lib` - 184 passed.
- New provider tests: `zero_idle_timeout_is_disabled`,
  `zero_stream_budget_is_disabled`.

## Related Items

- `BUG-063` (`boards/qa/deepseek-novita-native-tool-call-leak-resurfaced-on-stale-build.md`)
  - a leaked native tool call can keep a run burning until a wall-clock budget
  fires; this card removes the default budget so the leak (already fixed on
  synced builds) is not masked by a forced error.
- `BUG-071` - 30-minute budget observed ending a 10-step run.
- `BUG-038`, `BUG-043`, `BUG-046` - the bounds this change makes opt-in.

## Remaining

- Merged into `development` (see Merge Closeout). Remains in `qa` until a live
  post-merge QA pass is recorded.
- Production deploy is a separate release step (`development -> main`) and needs
  explicit owner confirmation. Until then the fleet still runs v0.1.3 with the
  30-minute cap.
- If a specific wedge persists with the caps disabled, capture a fresh
  reproduction; the current server log shows only the budget firing, not a
  provider leak on the installed build.

## Merge Closeout - 2026-11-13

- Branch `bug/BUG-075-session-timeout-vanilla-parity` (commits `1360b2a`,
  `1387c5d`, `ecc24c6`) merged into `development` at the owner's request.
- Card moved `doing -> qa`.
- Follow-up needed: `development -> main` release + fleet `ort-install` for the
  fix to reach all machines.

## Release v0.1.4 - 2026-11-13

- Released in **`v0.1.4`** (`main` `cc9befa`, release PR #194) and deployed to
  7/10 active machines: `webapps`, `webappssub1`, `matrillobase1`, `matrillo`,
  `matrillosub1`, `matrillosub2`, `j8rsxy`.
- `jasper`, `moldavite`, and `xp-farmer` were Tailscale-offline and still need
  `ort-install --rev cc9befa...` when reachable. `webappssub2` is disabled.

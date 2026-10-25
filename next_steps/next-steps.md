# Where We Left Off - 2026-10-25 05:35 EDT

## All Recommended Next Steps Completed - 2026-10-25 05:35 EDT

The initial report's recommended order was executed end to end. The durable record of what landed:

## Completed

- `BUG-063` moved `doing` -> `qa` (work was already complete; fix `79bc847` + tests in `development`).
- Local `development` fast-forwarded to `origin/development` (`13336f2`).
- `INFRA-007` + `INFRA-006` self-QA recorded; both moved to `done`. Published linux asset rev
  `bebe3ab`: sha256 OK, runs `0.1.0`, max GLIBC `2.34`; `ort-install` works standalone; `webapps`
  is checkout-free; `ensure-native-artifacts --check` resolves the prebuilt bundle.
- `BUG-065` implemented -> `qa`: fallback prompt now requires tool grounding and skill execution and
  forbids fabricated state; the skill tool emits an execution directive. Deterministic tests added.
- `FUTURE-PROOFING-002` implemented -> `qa`: `opencode_core::decision` boundary + deterministic stub;
  `crates/opencode-core/tests/architecture_layers.rs` enforces the crate map; wiki + invariant updated.
- `BUG-064` investigated -> `qa`: found and fixed the abort/drop race that could orphan a bash child
  process after an interrupt (captured run token, 250 ms cooperative grace, `kill_on_drop(true)`);
  regression test fails on pre-fix code and passes after.
- Board hygiene: duplicate `RESEARCH-007` resolved; the RuneBench card is now `RESEARCH-010`.

## Next Agent Instructions

**Current state:** all recommended steps are implemented, committed, and pushed on `development`:

- `08aa144` fix(session): ground the fallback prompt and require skill execution (`BUG-065`)
- `9d8e566` fix(server): keep abort from orphaning bash children (`BUG-064`)
- `7ca4d2b` feat(core): add the engine-neutral decision boundary (`FUTURE-PROOFING-002`)
- `d24f6ec` docs(wiki): record the crate boundary map and enforced layer rule
- `c45b893` board(BUG,INFRA,FUTURE-PROOFING,RESEARCH): lane moves, QA evidence, RESEARCH-010 renumber

`main` untouched. `qa` lane now holds `BUG-063`, `BUG-064`, `BUG-065`, `FUTURE-PROOFING-002`;
`INFRA-006`/`INFRA-007` are `done` with recorded QA.

**Start here:** read `boards/qa/fallback-prompt-drives-toolless-answers-and-fabrication.md` and
`boards/qa/interrupted-session-keeps-running-after-user-stops-it.md` for the two open verification
gates, then `boards/qa/Define stable-core and volatile-policy boundaries.md`.

**Verify:**
```bash
cd ~/apps/opencode-modded-rust
SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test -p opencode-core -p opencode-tool -p opencode-server \
  -p opencode-session -p opencode-provider
cargo fmt --all --check
bd -ca
```
Expected: all suites green (core 16, tool 68, server 75, session/provider suites), fmt clean, and
`bd -ca` reports only the pre-existing `CLI-007` post-merge-QA flag.

**Remaining gates (QA, not done):**
1. `BUG-065` live replay on `ollama/qwen3:30b` and one `deepseek/deepseek-flash` turn: load
   `board-where-left-off` and require actual tool calls. Deterministic tests only prove the prompt
   conflict is gone and the directive is present.
2. `BUG-064` live `ort` interrupt during a running bash tool: confirm no post-interrupt file mutation
   and a durable interrupted terminal state. Build with `ort-build` first (`ort` serves stale binaries).
   The original 2026-10-12 session is not recoverable on this host.
3. `FUTURE-PROOFING-002` follow-through: the V1 task-stage runtime claim is documented, not exercised.
4. `BUG-057` (hold): live TUI abort capture to distinguish "abort never sent" from the fixed class.

**Next board action:** after the two live QA passes, promote `BUG-065` and `BUG-064` to `done` with
the recorded evidence; otherwise leave them in `qa` with the specific failed step.

**Do not:** push `main`, promote anything to `done` without the live evidence above, or kill the
`next dev` processes on PID 3825226/382528 (not started by this session).

**References:** `next_steps/next-steps.md` (this file), `wiki/harness-substrate.md`,
`invariants/future-proofing.md`, `crates/opencode-core/src/decision.rs`,
`crates/opencode-core/tests/architecture_layers.rs`.

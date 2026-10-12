---
id: "BUG-063"
title: "Novita DeepSeek native tool-call leak resurfaced from a build without the BUG-060/BUG-061 normalization"
priority: "P1"
type: "bug"
area: "BUG"
spec: "invariants/providers.md"
status: "doing"
created: "2026-10-12"
---

# Novita DeepSeek native tool-call leak resurfaced from a build without the BUG-060/BUG-061 normalization

## Summary

Follow-up to `BUG-060` (fix, done) and `BUG-061` (guard, done). On 2026-10-12 a live local `ort`
session leaked DeepSeek native tool-call tokens into assistant `content` again, on the same provider
model `BUG-060` covers (`novita-ai/deepseek/deepseek-r1-0528`), so the tools did not execute.

Investigation shows this is **not a regression in the normalization code**: the running binary was
built from the standards main checkout `$HOME/standards/vendor/opencode-modded-rust`, whose
`development` diverged at `7742c11` (2026-10-08 10:32 -0400) and never received the `BUG-060`/`BUG-061`
merges (which live only on `origin/development`). The binary therefore contains **no**
`DeepSeekNativeToolCallExtractor`, no `QwenNativeNormalizer`, and no
`OPENCODE_DISABLE_DEEPSEEK_NATIVE_TOOLCALLS` kill switch. The leak is the pre-fix product behavior.

The card is open to (a) prove the resurfacing disappears on a build from a synced `development`, and
(b) close the stale-build gap that caused a supposedly fixed P1 to reappear in daily use.

## Reported behavior

Operator exported the transcript as `~/apps/tss-notes/create-a-board-item-for-this-issue.md` and
reported the tool-call issue "resurfacing". The same transcript also anchors the interrupted-session
report already captured as `BUG-061`; this card covers only the tool-call leak.

## Evidence (2026-10-12)

Source of truth: the Rust product's SQLite store
`$HOME/Library/Application Support/opencode/opencode.db` (macOS), session
`ses_c1250ed0689f44a7b06eb0be783a7a53`, directory `/Users/cchrisleepyles/apps/tss-notes`.
The session is present in the **Rust** store (not the vanilla `~/.local/share/opencode` store), so the
producing runtime was this product.

Two assistant messages on `novita-ai` / `deepseek/deepseek-r1-0528` leaked the native tokens:

1. `msg_ecf060ff1ffeHICbFfB0Y1TJJe` - `finish_reason: error`, `error: "run exceeded its time budget"`,
   `usage {prompt 17412, completion 4912}`, created `2026-10-12T16:37:28Z`. Content ends with:

   ```
   ...configuration files:<|tool▁calls▁begin|><|tool▁call▁begin|>function<|tool▁sep|>glob
   ```json
   {"path": "/Users/cchrisleepyles/apps/tss-notes", "pattern": "board.toml|boards/*"}
   ```<|tool▁call▁end|><|tool▁calls▁end|>
   ```

2. `msg_ecee176a1ffek0UhHUu0sHRPd8` - `finish_reason: stop`, created `2026-10-12T17:17:27Z`:

   ```
   ...a proper pattern:<|tool▁calls▁begin|><|tool▁call▁begin|>function<|tool▁sep|>glob
   ```json
   {"pattern": "board.toml|boards/"}
   ```<|tool▁call▁end|><|tool▁calls▁end|>
   ```

Both messages persisted the raw tokens as `text` parts with **no** `tool_call`/`tool_result` part, so
the tool never ran (the operator's follow-up prompt was literally "Did that tool execute successfully
or fail?").

Supporting runtime signals:

- `traces/server.log`: `ERROR opencode_server::routes: prompt run exceeded the run budget
  session=ses_c1250ed0689f44a7b06eb0be783a7a53` (`2026-10-12T17:07:22Z`).
- `traces/tui.log`: an interrupt/abort was issued for the same session (`interrupt: issuing abort for
  session ses_...`). The leaked call is a downstream consequence: with the call rendered as text the
  model never received a tool result and kept burning the run budget.

An all-session scan of the Rust store found leaked native-token assistant messages in exactly this one
session (2 messages), so this is a single live occurrence, not a fleet-wide pattern.

## Root cause (confirmed)

The binary that served the session lacked the `BUG-060`/`BUG-061` normalizer.

- Local `development` is `8caf076` and diverged from `origin/development` at `7742c11`
  (`git rev-list --left-right --count development...origin/development` -> `2 10`).
- `BUG-060`'s fix (`81989bc`, PR #138) and `BUG-061`'s guard (`a57ada8`, PR #139) are ancestors of
  `origin/development` but **not** of local `development`.
- `crates/opencode-provider/src/{openai,stream}.rs` in this checkout contain only the older
  `normalized_reason` handling; `rg -i 'DeepSeekNativeToolCallExtractor|QwenNativeNormalizer|native'`
  finds none of the fix symbols.
- `strings -a target/debug/opencode` contains neither `DeepSeekNativeToolCallExtractor` nor
  `OPENCODE_DISABLE_DEEPSEEK_NATIVE_TOOLCALLS`, confirming the built artifact predates the fix, while
  the same binary does contain `OPENCODE_RUN_TIMEOUT_MS`/`OPENCODE_STREAM_BUDGET_MS`.

`ort-build` defaults to `$OPENCODE_RUST_REPO` = this checkout, so `ort` served the stale binary.

The fix on `origin/development` does cover this exact byte shape: the legacy (novita) parser chains
`DeepSeekNativeToolCallExtractor` + `QwenNativeNormalizer` and flushes deferred calls at finish, and
`crates/opencode-provider/tests/fixtures/deepseek/native_toolcall_tokens.sse` uses the same
`<｜tool▁calls▁begin｜>` / `function<｜tool▁sep｜>bash\n```json\n{...}\n```` /
`<｜tool▁calls▁end｜>` delta split that leaked here. The `BUG-061` kill switch
(`OPENCODE_DISABLE_DEEPSEEK_NATIVE_TOOLCALLS`) is unset in the environment, so a synced build would
have normalized it.

## Why this is a follow-up, not a re-open

`BUG-060`/`BUG-061` are correctly implemented and QA'd on `origin/development` (live `odn`
`novita-ai/deepseek/deepseek-r1-0528` turns executed 5/5 with no raw tokens on 2026-10-10). The defect
only reappears because the checkout the binary was built from never advanced past the fix.

## Verification - synced build replay (2026-10-12)

Synced this checkout to `origin/development` (merge `ffe2bf4`; card commit `ae8a401`), so local
`development` now contains `BUG-060`'s fix (`81989bc`) and `BUG-061`'s guard (`a57ada8`). Rebuilt with
`ort-build`.

- Source commit `ae8a401`; binary `target/debug/opencode` (built 2026-10-12 18:39 local). `strings` on
  the rebuilt binary now contains `OPENCODE_DISABLE_DEEPSEEK_NATIVE_TOOLCALLS`; it previously did not.
- Offline: `SCOPEMUX_SKIP_NATIVE_BUILD=1 cargo test -p opencode-provider --test deepseek_fixture_replay`
  -> 9 passed, including `native_toolcall_tokens_in_content_become_a_tool_call` and the char-boundary
  split test, run against the exact leaked token shape
  (`crates/opencode-provider/tests/fixtures/deepseek/native_toolcall_tokens.sse`).
- Live: `opencode run -m novita-ai/deepseek/deepseek-r1-0528 --dir
  $HOME/worktrees/opencode-modded-rust/bug063-qa` with a prompt forcing the `bash` tool
  (`touch /tmp/bug063-marker && echo EXECUTED`) -> **6/6 runs executed the tool** (marker files
  created), and no `<｜tool▁...` native tokens appeared in any assistant output.

Result: PASS. On the synced build the DeepSeek novita native-token leak no longer reproduces, and the
original report is fully explained by the stale binary.

Sync side effect resolved: the local-only interrupted-session card collided with `origin`'s
`BUG-061` id; it was renumbered to `BUG-064` during the merge.

## Flush-path findings (2026-10-12)

Both parsers (stateful `parse_openai_sse_stateful` and the `novita-ai` legacy `parse_legacy_sse_data`)
were probed with an intentionally cut native-token stream. Regression coverage added in
`crates/opencode-provider/tests/native_abort_flush.rs`.

- **Budget/timeout/abort (no terminal frame):** the buffered native call is **dropped**, and no raw
  tokens reach the assistant text. This is the path the first leaked transcript message exercised
  (`finish_reason: error`, "run exceeded its time budget") - on the synced build it does **not** leak.
- **Complete native block + `finish_reason`/`[DONE]`:** normalized to a real tool call, no leak
  (control).
- **Unterminated native block + `finish_reason`/`[DONE]`:** previously leaked the partial native
  tokens as `TextDelta`; now the raw markup is discarded and a visible `StreamEvent::Error` is
  surfaced (see Fix below). Covered by `partial_native_block_then_finish_does_not_leak_and_reports`.

Conclusion: the budget/abort path is safe, and the truncated-block-on-finish leak is fixed.

## Fix - 2026-10-12

`DeepSeekNativeToolCallExtractor` (`crates/opencode-provider/src/stream.rs`) no longer emits the
remainder of an unterminated calls block as text. On flush it discards the raw token markup, and
`finish()` appends a `StreamEvent::Error` ("incomplete DeepSeek native tool-call block (missing
closing token) was discarded; no tool was executed"). That error propagates as a hard, visible turn
failure (`crates/opencode-session/src/prompt.rs`), so the truncated call fails loudly rather than
leaking tokens or silently dropping the call. Both the stateful and `novita-ai` legacy parsers share
the extractor, so both are covered; `OPENCODE_DISABLE_DEEPSEEK_NATIVE_TOOLCALLS` still restores
pass-through.

Tests: updated `unterminated_native_block_is_dropped_and_reported` (unit) and
`partial_native_block_then_finish_does_not_leak_and_reports` (integration); the previously ignored
residual test is now active. `cargo test -p opencode-provider` - 124 lib + 9 fixture + 4
`native_abort_flush` + 7 integration, all green.

## Remaining investigation

1. (done) Synced-build replay - see Verification above.
2. (done) Budget/abort flush path - see Flush-path findings above (no leak on abort; the
   truncated-block leak is a separate residual).
3. (done) Stale-build guard added in `~/standards/opencode-config` (see Fix below): `ort-build`
   warns when the checkout is on `development` and behind `origin/development`, and refuses under
   `OPENCODE_RUST_STRICT_CHECKOUT=1`.
4. (done) Truncated-block leak fixed - see Fix above.

## Stale-build guard - 2026-10-12

To stop a build from silently missing merged fixes, `opencode-rust-build`
(`~/standards/opencode-config`) now calls `opencode-rust-warn-stale-checkout` before building. When
`$OPENCODE_RUST_REPO` is checked out on `development` and is behind `origin/development`, it prints a
loud warning naming the behind-count and the fetch/merge command, and refuses to build when
`OPENCODE_RUST_STRICT_CHECKOUT=1`. No warning on feature branches/worktrees (legitimately not
fast-forwarded) or when up to date.

Committed in the standards repo (`34e3c26`); the guard is process tooling, so it lives there, not in
the product. This is distinct from `INFRA-003`, which is a CI clean-clone build guard.

## Scope

- Investigation plus a synced rebuild/replay; provider parser only if a residual code gap is proven.
- The interrupt/keep-running behavior is `BUG-061`'s class and is out of scope here except where it
  masks or delays the tool-call flush.

## Non-goals

- Re-opening `BUG-060`/`BUG-061` unless the synced rebuild still leaks.
- Reworking provider architecture (`FEAT-011`), Qwen handling (`BUG-062`), or model capability gating
  (`FEAT-013`).
- Committing the exported transcript or any TSS/personal data into this repo.

## Done when

- A synced-build replay is recorded: commit, binary path, harness/prompt, the executed `bash` tool
  side effect, `finish_reason`, and no raw tokens in output.
- The budget/interrupt flush path is verified: no leak on abort, and the truncated-block-on-finish
  leak is fixed with provider tests.
- The stale-checkout cause is explained with a concrete guard or an explicit process note, and the
  local checkout no longer serves a fix-missing binary.

## Related Items

- `BUG-060` Novita DeepSeek R1 leaks native tool-call tokens; tools not executed (done) - the fix this
  follow-up tracks the reappearance of.
- `BUG-061` Bound false positives for DeepSeek native tool-call normalization (done) - the guard +
  kill switch; the deferred-flush path is what item 2 exercises.
- `BUG-062` Qwen/Ollama tool-loop investigation and fix (done) - sibling tool-call-not-executed class
  in the same shared parsers; coordinate before touching them.
- `BUG-057` Interrupt still does not stop a running session; deepseek keeps thinking after Esc (done) -
  same provider; the leaked call is what kept the run alive.
- `QA-002` Deterministic DeepSeek stream/loop fixture harness (qa) - extend if a new parser gap lands.
- `boards/todo/add-ci-clean-clone-native-build-guard.md` - clean-clone build guard; same
  stale/misbuilt-artifact family.
- `wiki/deepseek-native-tool-call-normalization.md` and `invariants/providers.md` - the decision record
  and rule the fix implements.

## Notes

- Created on 2026-10-12 from the operator report and the exported tss-notes transcript; card authored
  on a checkout (`8caf076`) that predates the `BUG-060`/`BUG-061` merges, so those done cards were read
  from `origin/development`.
- Do not paste TSS candidate/appointment payloads from the transcript into this card; reference the
  transcript path only.

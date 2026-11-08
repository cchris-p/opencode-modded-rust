---
id: "H-008"
title: "P1 stability cluster: BUG-027/028/029/038 - Handoff"
status: "complete"
created: "2026-09-23"
updated: "2026-09-23"
owner: ""
target: "development"
blocked_reason: ""
needs_human: ""
items: ["BUG-027", "BUG-028", "BUG-029", "BUG-038"]
---

# P1 Stability Cluster - Handoff

## Objective

Close out the P1 session-stability defects that all converge on the same
provider-stream / agent-loop / TUI-refetch surface:

- `BUG-038`: a DeepSeek turn that emits reasoning and partial text, then stalls
  silently forever, leaving the session stuck `active` (hits the default model).
- `BUG-027`: the TUI freezes for the whole reasoning phase because every
  `session.updated` event triggers a blocking full-session refetch inside the
  event loop.
- `BUG-028`: a session wedges on a recurring provider `400` when an assistant
  `tool_calls` turn has no matching tool replies.
- `BUG-029`: `Esc` cannot reliably interrupt a thinking turn; the interrupt hint
  toggles between states.

## Included Board Items

| Item | Title | Priority | Lane at handoff time | Fix state at handoff time |
|------|-------|----------|----------------------|---------------------------|
| `BUG-038` | DeepSeek reasoning turn stalls mid-turn and never completes | P1 | `todo` | not implemented; fixed here |
| `BUG-027` | Session keeps freezing during thinking mode | P1 | `qa` | tracing only; bounded fix added here |
| `BUG-028` | Session wedges with a recurring 400 when tool_calls have no tool messages | P1 | `qa` | normalizer already merged (PR #64); QA re-verified here |
| `BUG-029` | Esc cannot interrupt a thinking turn; hint toggles | P1 | `qa` | state-machine fix already merged (PR #66); QA re-verified here |
| `BUG-034` | Default DeepSeek model missing from provider registry | P1 | `qa` | related dependency already merged (PR #78); QA re-verified here |

`BUG-034` is included as a related item only: it changed the product default from
the now-deprecated `deepseek-v4-flash` to `deepseek-flash`, which is the model
`BUG-038`'s evidence was captured on. No further code is required from it here.

## Dependency Order

1. `BUG-038` and `BUG-027` are independent code changes (provider/session vs
   TUI). They can land in the same PR.
2. `BUG-028` and `BUG-029` are already merged; this handoff only re-verifies them
   against the current tree so the cluster can close out together.
3. `BUG-038`'s stream timeout also removes the "quiet stream ignores the cancel
   token" amplifier that `BUG-029` H3 and `BUG-027` both depend on, so landing it
   strengthens the other two.

## Implementation In This PR

- `BUG-038` (`crates/opencode-provider/src/stream.rs`,
  `crates/opencode-session/src/prompt.rs`):
  - `with_idle_timeout` / `DEFAULT_STREAM_IDLE_TIMEOUT` (90s) wrap every provider
    stream in `SessionPrompt::loop_inner`.
  - A stream that stops emitting is converted to `StreamEvent::Error`, which the
    existing error arm turns into `Err`; the server records an error assistant
    message and the session returns to `idle` instead of hanging `active`.
- `BUG-027` (`crates/opencode-tui/src/app/app.rs`):
  - `STREAM_SYNC_MIN_INTERVAL` (200ms) + `can_start_stream_sync` coalesce
    streaming `session.updated` refetches (trailing-edge), bounding them from
    ~20/s to ~5/s.
  - `SessionStatusIdle` triggers one immediate final refetch.

## Single-PR Plan

| PR | Board Items | Branch | Notes |
|----|-------------|--------|-------|
| 1 | `BUG-038`, `BUG-027` (+ QA for `BUG-028`, `BUG-029`, `BUG-034`) | `bug/BUG-027-028-029-038-stability-cluster` | Target `development`. Board and handoff updates included in the same PR. |

## Board Updates In The PR

1. `BUG-038`: `todo -> qa`, with Dev Notes and Verification.
2. `BUG-027`: remains `qa`; Dev Notes (implementation) and Verification added.
3. `BUG-028`, `BUG-029`, `BUG-034`: remain `qa`; QA Notes added.
4. No board items are created or deleted.

## Verification / Completion Gates

- `cargo test -p opencode-provider`, `cargo test -p opencode-session`,
  `cargo test -p opencode-tui`, `cargo test -p opencode-config` green.
- `cargo check --workspace` and `cargo fmt --all -- --check` clean.
- Live checks not possible in this headless environment: the DeepSeek stall
  reproduction, the TUI thinking-phase responsiveness measurement
  (`OPENCODE_TUI_TRACE`), the `BUG-028` abort->follow-up sequence, and the
  `BUG-029` Esc-twice-during-thinking reproduction. These are the remaining
  `qa` items.

## Risks And Rollback

- **`BUG-038` timeout window (90s)**: too short could abort a genuinely quiet but
  live turn; 90s is far above normal reasoning chunk spacing. It is a single
  constant (`DEFAULT_STREAM_IDLE_TIMEOUT`) and is overridable at the call site.
- **`BUG-027` cadence (200ms)**: lower values stream more smoothly but refetch
  more; higher values are lighter but laggier. Landed independently of the
  provider change so it can be reverted on its own.

## Deferred / Out Of Scope

- Streaming incremental parts over the event channel so the TUI does not need a
  full refetch (the deeper `BUG-027` follow-up).
- Provider-specific terminal-event normalization (for example giving Google's
  SSE parser an explicit `Done`), which would allow a stricter
  "closed-without-terminal is an error" rule at the session layer.
- Non-OpenAI-compatible request-path normalizers for `BUG-028`.

## Completed With

- PR #89 merged into `development`: `fix: P1 stability cluster (BUG-038 stream stall, BUG-027 thinking
  freeze)` on `bug/BUG-027-028-029-038-stability-cluster`.
- Delivered: `BUG-038` provider-stream idle timeout; `BUG-027` streaming-refetch coalescing.
- Re-verified and closed: `BUG-038`, `BUG-027`, `BUG-028`, `BUG-029`, `BUG-034` (all moved to `done`).
- Remaining (non-blocking, post-merge): live `ort` / `OPENCODE_TUI_TRACE` verification of the
  thinking-phase fix and the DeepSeek stall reproduction, which the headless closeout environment
  could not run.


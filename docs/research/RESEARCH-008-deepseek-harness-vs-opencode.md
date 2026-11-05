# RESEARCH-008 — DeepSeek Harness (dsh) vs vanilla OpenCode: architecture comparison

**Date:** 2026-11-05
**Card:** `RESEARCH-008` (`boards/todo/evaluate-deepseek-harness-integration-into-ort.md`)
**Status:** Research phase — source-level comparison recorded. The end-to-end A/B run
required by the card's `Done when` is **not** done yet.
**Non-goals honored:** no runtime dependency added, nothing vendored, no invariant or
disposition changed, no private content committed.

## Sources and revisions (pinned)

| Source | Revision | Notes |
|---|---|---|
| DeepSeek Harness (`deepseek-ai/deepseek-harness`) | `fcc0ad462172522aaf5c93a7b2e01f6b9f0b0159` (master, 2026-11-04), package `0.2.2-rc.1` | cloned with `git clone --depth 1 https://github.com/deepseek-ai/deepseek-harness.git /tmp/opencode/dsh`; MIT; developer preview |
| Vanilla OpenCode reference (`anomalyco/opencode` mirror) | `f54ce313b99a6661d7758ad042f7a6e05c8e0972` (branch `dev`, package `1.18.31`) | the pin recorded in this repo's `AGENTS.md` |

Read from each project's own checkout/docs; nothing below is from memory.

Important framing for the OpenCode side: the pinned snapshot contains **two session
stacks**. The live TUI/HTTP prompt path is the "V1" stack
(`packages/opencode/src/session`), while the Effect-native "V2" stack
(`packages/core/src/session*`, `packages/llm`) is partially wired. Comparisons
below describe the behavior that actually runs.

## 1. Executive summary

`dsh` and vanilla OpenCode solve the same high-level problem (an agent loop over a
durable session, tools, providers, subagents, MCP/LSP/skills) with **opposite
composition strategies**:

- **dsh — "everything is a plugin" on Cordis.** The model adapter, tool registry,
  session log, and agent loop are all plugins that can be replaced from
  configuration. A running harness is a *profile*: an ordered stack of plugin
  *bundle* patch layers under user overlays, with hot module replacement. There is
  no privileged core.
- **OpenCode — a fixed Effect service graph with declared seams.** Services are
  composed as `LayerNode`s; a plugin can add tools/providers/auth and intercept a
  fixed set of hooks, but cannot replace the session runner, tool registry,
  permission engine, database, or HTTP router at runtime. Component replacement is
  a build-time `LayerNode` substitution.

The other large differences cluster into four themes:

1. **Surface and intent.** OpenCode is TUI-first and coding-first. dsh is
   Web/Desktop-first (no first-party TUI), broader than coding (browser-use,
   computer-use, documents, deliverables), and DeepSeek-first.
2. **Composability.** dsh exposes composition as product configuration (profiles,
   bundles, patches, HMR, an agent-usable plugin manager/market). OpenCode exposes
   composition as a code-level service graph plus a bounded plugin hook API.
3. **Coordination primitives.** dsh ships agent teams with a durable shared task
   DAG, model-written workflow scripts, same-session goals, scheduled follow-ups,
   background jobs, and declared deliverables. OpenCode ships subagents (task tool)
   with a depth limit, background tasks, and a todo tool.
4. **Safety and egress.** dsh has an OS-level filesystem sandbox seam and a
   fail-closed one-shot approval seam; OpenCode has a rule-based permission engine
   and **no OS sandbox**. dsh also has DeepSeek-specific wire extensions that can
   upload the full session log and plugin inventory to the model endpoint by
   default on the official adapter.

## 2. Side-by-side comparison

| Dimension | dsh 0.2.2-rc.1 | OpenCode 1.18.31 (pin) |
|---|---|---|
| Core philosophy | Everything is a plugin; no privileged core | Fixed Effect service graph with declared hooks/seams |
| Extension substrate | Vendored **Cordis**: services on `ctx`, typed events, reversible effects | Effect `Context.Service` + `LayerNode`; V1 plugin hooks + V2 replayable transforms |
| Composition unit | **Profile** = ordered **bundles** + `cordis.patch.yml` + home patch + `--patch` overlays | `opencode.json{,c}` config + plugin list + file-based agents/skills/commands |
| Runtime reconfiguration | Hot module replacement (config-only or full recompose) | Restart (no HMR) |
| Arbitrary component replacement | Yes, from configuration (model adapter, tool registry, session log, loop, UI, …) | No; fixed runtime replacement set is build/bootstrap-time |
| Agent loop | `core/agent` interface + swappable `core/agent-loop`; turn/step event lifecycle | `SessionPrompt.runLoop`; one loop iteration = one provider turn (V1) |
| Loop interception | Typed events in 3 domains (`session/*`, `agent/*`, capability events), with `emit`/`waterfall`/`parallel`/`serial`/`bail` modes | V1 named plugin hooks; V2 transforms for a fixed domain set |
| Session truth | Append-only `SessionEvent` log is the single source of truth; model history derived | Event-sourced durable events + SQLite projections; V1 `message`/`part` tables |
| Persistence backend | JSONL (`session.vN.jsonl[.zstd]`) with generation/migration chain (`session-persistence-jsonl`) | SQLite (Drizzle/Effect SQLite), `~/.local/share/opencode/opencode.db` |
| Context assembly | `core/system-prompt` sections + tool schemas + dynamic runtime context | `SystemPrompt` environment/instructions/provider text + request prep (V1); V2 Context Sources/epochs |
| Compaction | Optional **capability seam** (`ctx.compaction`); log-recorded lock/summary events | Built-in hidden compaction agent turn + plugin hooks; V2 event-sourced rolling summary |
| Tool registry | `core/tools` on `ctx.tools`; canonical output schema + guarded pipeline; parallel-call opt-in | `Tool.define`/`Tool.make`; V1 registry + V2 scoped registry; output truncation |
| Providers | `ctx.llm` adapter seam; DeepSeek-first with official wire extensions | models.dev catalog + AI SDK (default) + native `@opencode-ai/llm` route/protocol layer |
| Permissions | `ctx.approval`: fail-closed `allowed-once`, policy `ask`/`never`, audit events | allow/ask/deny rules, wildcard, last-match-wins; session "always" approvals |
| OS sandbox | **Yes**: `ctx.sandbox`, per-call policy, bwrap/Landlock/Seatbelt/Windows ACL, `full`/`partial` enforcement reporting | **No** OS sandbox; "sandbox" means project worktree dirs |
| Subagents | Unified activation API; local + external backends; catalog, naming, depth, structured output | `task` tool; child session, depth default 1, foreground/background, resume |
| Multi-agent coordination | **Agent Teams**: durable shared task DAG, `blockedBy`, advisory `writeScopes`, task tools | Not built in beyond subagents + todo |
| Orchestration | **Workflow** seam: model-written JS script spawning subagents | Experimental code-mode interpreter over tools; no workflow-script equivalent |
| Long-horizon primitives | Goals, schedule, background jobs, deliverables (`present` tool + changed-file summaries) | Todo tool + background tasks |
| Browser/computer automation | Built-in seams with multiple providers (Playwright MCP, Chrome DevTools MCP, Stagehand; CUA driver) | Not built in |
| Remote execution | SSH filesystem/subprocess/sandbox providers; persistent terminals | PTY + workspace adapters; no unified SSH execution seam |
| Programmatic tool calling | PTC runtime (Node/TS VM; experimental Python backend) | Experimental `codemode` |
| Human commands | Plugin command registry, executes without a model turn | Built-in/config/MCP/skill commands |
| MCP | `ctx.mcp` client + resource service + Web/Desktop server manager | local/remote MCP, OAuth, prompts→commands, resources→tools |
| LSP | `ctx.lsp` seam, 4 semantic ops, provider registry | Per-file LSP servers, diagnostics appended to edit results |
| Skills | Skills package, loaded into prompt/tools | `SKILL.md` discovery, permissions, injected into prompt, slash-command bridge |
| UI surfaces | **Web** (primary), **Desktop** (Electron), headless CLI, ACP; **no shipped TUI** | **TUI** (primary), Web, Desktop, ACP, CLI `run` |
| Server/SDK | Remote BFF + connection service; TS SDK (JSON-RPC profiles) + Python SDK | Effect `HttpApi` + SSE; generated JS SDK + Effect client + in-process SDK-next |
| Plugin marketplace | Official catalog + plugin manager/market, agent-installable plugins | No marketplace; npm/file plugin specs only |
| Telemetry/egress identity | Anonymous Harness-home UUID; OTEL session telemetry; `dsh_plugin_packages` and `dsh_session_log` body extensions | Provider requests only; no equivalent session-log upload |
| Interop with other harnesses | Claude Code hooks + Codex hooks bridge packages | Not present |
| Maturity | Developer preview, explicit breaking changes promised | Stable release line (`1.18.31`) |

## 3. Detailed differences

### 3.1 Composition: Cordis profiles vs the Effect service graph

**dsh.** A running harness is composed at boot from an empty root by applying, in
order: each bundle's patch (`dsh.profile.bundles`), the profile's
`cordis.patch.yml`, `$DSH_HOME/cordis.patch.yml`, then `--patch` overlays
(`apps/cli/README.md`, `docs/architecture.md`). A bundle is a distribution format
for Cordis config rows plus the code they mount. Shipped profiles: `web`,
`headless`, `sdk`, `sdk-minimal`, `acp` (plus the reserved `desktop` profile owned
by the Electron app). `--dump-config` prints the composed tree so any row can be
patched. `dsh-hmr` can recompose on config/plugin change without a restart.

Because the components are plugin rows, the same mechanism that adds a tool can
replace the model adapter, session log backend, tool registry, or agent loop.
Registrations are reversible effects (`ctx.effect`, `ctx.on`), so unload unwinds
predictably. Per-agent scoping (`agent.ctx`, `isolate` realms in presets) lets one
session have a different capability set without changing the global tree.

**OpenCode.** Services are Effect `Context.Service` classes composed as
`LayerNode`s with explicit dependencies; global vs location nodes are built once or
per directory/workspace (`packages/core/src/effect/layer-node.ts`,
`packages/core/src/location-services.ts`). `LayerNode` supports replacements, but
they are applied by the app-node builder at **build/bootstrap** time
(`packages/core/src/effect/app-node-builder.ts`), not by a running plugin. V1's
runtime is a fixed `ManagedRuntime` with an explicit node list
(`packages/opencode/src/effect/app-runtime.ts`). So OpenCode is modular in source
but not reconfigurable at runtime at the same depth.

**Why it matters for research/vetting tooling:** dsh can be reshaped (e.g. swap
the loop, add a bespoke evidence-collection tool, change persistence) from config;
OpenCode requires a code change and a rebuild for the same class of change. That is
a real capability difference, not a cosmetic one.

### 3.2 Agent loop and event contract

**dsh** defines an `Agent` interface plus a default `agent-loop`. The loop runs a
turn (zero or more steps; a step = one model request + its tools) and emits
durable session events (`turn/*`, `step/*`, `user/message`, `assistant/message`,
`assistant/attempt`, `tool/*`) and live events (`agent/pre-step`, `agent/request`,
`agent/assistant-stream`, `agent/turn-stopping`). Waterfall events are
around-middleware; listeners must delegate with `next()`. `agent/pre-step` can
accept, rewrite, or reject claimed input, which makes policy and injection
first-class. The session log is the source of truth: "model-visible means logged",
and every request must be reconstructable from the log.

**OpenCode** runs `SessionPrompt.runLoop`; each iteration selects the latest
unfinished assistant/tool state, checks context overflow, resolves agent/model,
and calls `SessionProcessor.process` over `LLM.stream`. When the finish reason is
not tool-calling and no tool calls are outstanding, the loop ends. Tool-call turns
continue. `SessionRunState` serializes one runner per session. Interception is via
named plugin hooks (`chat.message`, `chat.params`, `chat.headers`,
`tool.execute.before/after`, `experimental.chat.*.transform`, etc.).

Both are event-sourced, but dsh's contract is **typed and merge-extensible**
(a plugin declares new `SessionEventMap` and event names by declaration merging),
whereas OpenCode's durable event set is defined by the product and plugins act
through hooks. dsh's multi-mode dispatch (`emit`/`waterfall`/`parallel`/`serial`/
`bail`) is also a richer interception vocabulary than OpenCode's sequential hook
trigger.

### 3.3 Persistence and compaction

**dsh** persists an append-only JSONL log (`session.jsonl[.zstd]` for v0,
`session.vN.jsonl[.zstd]` after), selects the numerically highest canonical
generation, and migrates through an adjacent `vN -> vN+1` chain. Committed
generations are never renamed/replaced/deleted. Every assistant message embeds the
exact compact timed stream that produced it; failed/retried/cancelled attempts are
kept as `assistant/attempt` without entering model history. Compaction is an
*optional capability seam* (`ctx.compaction`) with `compaction/start|summary|end`
log events, a log-recorded lock, and a surface replacement range. A tokenizer- or
template-based backend can replace the LLM summarizer.

**OpenCode** persists to SQLite with `session`, `message`, `part`, `todo`,
`event`, plus V2 `session_input` and `session_context_epoch`. Model history is
projected from messages; streaming deltas are live-only and the full part is
persisted at text/reasoning end. Compaction is a hidden agent turn with a summary
message and `tail_start_id`, plus pruning of old tool outputs; plugins can replace
the prompt/autocontinue. V2 moves to event-sourced structured rolling summaries.

The practical difference: dsh's durable format is file-oriented, generation-based,
and explicitly migration-chained; OpenCode's is relational and schema-migrated.
dsh is more auditable at the log-file level; OpenCode is more queryable.

### 3.4 Tools and provider seams

dsh's tool definition carries a **mandatory canonical output schema** plus
`render`, and optional `presentCall`/`presentResult` UI projections. The registry
whitelists model-facing fields so execution/output metadata cannot leak into the
request. Tool calls can opt into parallel dispatch with a pure
`isConcurrencySafe(args)` classifier, and `tools/pre-execute`,
`tools/execute`, `tools/post-execute` are waterfalls. Output is canonical JSON;
content projection is a separate, replayable step.

OpenCode's tool wrappers decode arguments, execute, truncate to 2000 lines / 50 KiB,
write the overflow to a tool-output file, and append LSP diagnostics after edits.
Parallelism and interception are product-level; the V2 registry adds scoped
registration and a central `ToolOutputStore`. OpenCode has model-dependent tool
selection (e.g. `apply_patch` for GPT) and injects structured output on demand.

Providers: dsh's `ctx.llm` adapter seam is provider-neutral but ships a
DeepSeek-first adapter; `llm-pi-ai` is the neutral path. OpenCode has a much
broader provider matrix via models.dev, plus a native route/protocol/framing layer
that models OpenAI Chat/Responses, Anthropic, Gemini, Bedrock, and provider
facades.

### 3.5 Safety: approval and sandbox

dsh has two independent safety seams:

- **Approval** (`ctx.approval`): a closed outcome
  `allowed-once | rejected | cancelled | unavailable`; callers fail closed unless
  the outcome is `allowed-once`. Per-session policy is `ask` or `never`, and
  `never` is enforced *before* answerers so even a `prepend` listener cannot
  bypass it. Every ask/decision is an audit event that stays out of the model
  transcript.
- **Sandbox** (`ctx.sandbox`): a per-call filesystem-effect policy
  (`read-only`, `workspace-write`, `danger-full-access`) enforced by bwrap/Landlock
  on Linux, Seatbelt on macOS, and restricted-token ACLs on Windows. Enforcement
  completeness is reported (`full`/`partial`) so consumers can refuse a partial
  boundary. Network and process visibility are explicitly outside this vocabulary.

OpenCode has a rule-based permission engine (allow/ask/deny with wildcards,
last-match-wins, per-agent defaults, command-prefix shell matching, external-
directory checks) and **no OS-level sandbox**; the shell runs directly. V2 adds a
persisted `permission_saved` table. This is the largest safety gap between the two.

### 3.6 Multi-agent and long-horizon work

dsh ships, as first-class optional capabilities:

- **Subagents** with a unified activation API, local child sessions and external
  backends, naming, depth limits, per-child presets, structured output schemas.
- **Agent Teams**: a durable shared task DAG (`blockedBy`, compare-and-set
  revisions, advisory `writeScopes`) plus tools, enabled per root session.
- **Workflow**: a model-written JS orchestration script that spawns subagents,
  run under the calling session's file policy.
- **Goals**: a durable same-session objective with `active/paused/blocked/complete`
  phases and process-local activation.
- **Schedule**: scheduled follow-ups.
- **Jobs**: a background-job runtime with owner-scoped records.
- **Deliverables**: a `present` tool plus per-turn changed-file summaries derived
  from git snapshots and whole-file captures.

OpenCode has subagents (task tool, depth default 1, resume, background), a todo
tool, and background tasks. It does **not** have a shared task board, workflow
scripts, goals, schedules, or deliverable manifests. For a research department that
must decompose a question, fan out evidence gathering, and assemble a provenance-
bearing synthesis, dsh's coordination primitives are meaningfully richer.

### 3.7 Surfaces, SDK, and interop

dsh's primary client is the Web UI (browser client plugins, Remote/BFF
architecture), with an Electron desktop app and headless/SDK/ACP profiles. There is
no shipped TUI (the docs mention a `tui` profile only as an example of an installed
profile). OpenCode is TUI-first and additionally ships Web, Desktop, ACP, and a
`run` CLI.

Both expose ACP for editor automation and both have MCP and LSP integration. dsh
adds **Claude Code hook** and **Codex hook** bridges, i.e. it can consume another
harness's hook protocol. OpenCode's plugin model is native-only.

### 3.8 Egress, identity, and plugin supply chain

Two dsh behaviors matter for this repo's trust/egress posture:

- **`dsh_session_log`**: on the official DeepSeek adapter, the harness can attach a
  contiguous suffix of the canonical session log to the model request by default
  (8 MiB per request), advancing an accepted-watermark event. This exposes cwd,
  system-prompt snapshots, user/assistant content, embedded assistant streams,
  failed-attempt output, tool arguments/results, compaction summaries, feedback,
  and plugin events to the endpoint. Adapter API keys are not included.
- **`dsh_plugin_packages`**: active npm plugin package names/versions are attached
  to every request by default.
- A stable anonymous `x-deepseek-harness-user-id` is sent on authorized requests.

These are configurable and are provider-specific (the neutral adapter does not
implement them), but they are on by default for DeepSeek. OpenCode has no
equivalent transcript upload.

dsh's plugin manager can **install npm/git/path bundles and run their Host code
in-process, outside the workspace sandbox**; the operation requires
`danger-full-access` or an explicit approval, and build-script approval is a
separate step. OpenCode plugins also run in-process, but there is no agent-usable
installer/marketplace.

## 4. What is genuinely different (not just naming)

1. **Replaceability depth.** dsh lets configuration replace the loop, tool
   registry, session log, and model adapter. OpenCode's equivalent requires source
   changes and a rebuild. This is the core "everything is a plugin" claim and it
   holds at the architecture level.
2. **Composition as a product surface.** Profiles + bundles + patches + HMR +
   dump-config + an agent-usable plugin manager/market is a productized
   composition system. OpenCode has config + plugins, but no comparable layering,
   live recompose, or market.
3. **OS sandbox + fail-closed approval.** dsh treats confinement and approval as
   swappable seams with explicit enforcement facts. OpenCode's permission engine is
   richer in rule ergonomics (wildcards, per-agent defaults, shell arity) but does
   not confine the process.
4. **Coordination primitives.** Agent Teams, workflow scripts, goals, schedules,
   jobs, and deliverables have no OpenCode equivalent.
5. **Breadth beyond coding.** Browser-use, computer-use, document handling, and
   SSH execution are first-class seams; OpenCode is coding-focused.
6. **DeepSeek wire integration.** Session-log upload, plugin inventory, anonymous
   identity, and official-API extensions are unique to dsh.

## 5. What is the same or comparable

- Event-sourced durable session with derived model history.
- Compaction with a summary/replacement mechanism.
- Subagents with depth limits and background execution.
- MCP client, LSP integration, skills, human/slash commands.
- ACP editor integration and a generated client SDK.
- Rule-based permissions (OpenCode) / approval seam (dsh).
- Provider abstraction with streaming and tool calls.
- Web + Desktop + CLI surfaces.

This matters: dsh is not a different *category* of harness. It is the same category
with a more radical composition model and a wider capability surface.

## 6. Locked decision (2026-11-05)

**Do not integrate dsh, and do not run it as a Research Department dependency.**
Adopt a narrow set of DeepSeek-API efficiency capabilities product-side in `ort`.
This is the recommended integration shape: *borrowed capability, not a harness
dependency*. It is consistent with `invariants/future-proofing.md` (absorption-
resistant core, no second plugin runtime) and `invariants/research-department.md`
(no harness/provider dependency; dsh output never canonical).

The following dsh ideas are explicitly rejected:

- the Cordis plugin runtime / profiles / bundles / HMR as a product substrate;
- `dsh_session_log` and `dsh_plugin_packages` egress (privacy/trust conflict);
- DeepSeek account auth and Files-API image plumbing (not needed for the coding
  workflow);
- code-level copying of dsh packages.

The following are adopted as implementation scope:

1. **Correct DeepSeek model metadata and live listing.**
2. **Reasoning effort / thinking control** on the DeepSeek path.
3. **Cache-safe prompt/tool prefix handling** (do not rewrite the leading prefix
   when only dynamic context changed).
4. **Evaluate the Anthropic-Messages transport** (`api.deepseek.com/anthropic`)
   as the evidence-gated umbrella for the documented `in-history` /
   `addition-only` semantics; make it the default only if measured.

### Live-probe evidence (2026-11-05)

`GET https://api.deepseek.com/models` with `DEEPSEEK_API_KEY` returned:

| Model | context_window | max_output_tokens | effort | `api_capabilities.anthropic_messages` |
|---|---|---|---|---|
| `deepseek-flash` | 1,048,576 | 393,216 | `low`/`high`/`max`, default `high` | `system_prompt_update: in-history`, `tool_update: addition-only` |
| `deepseek-v4-pro` | 1,048,576 | 393,216 | `low`/`high`/`max`, default `high` | `system_prompt_update: leading-only`, `tool_update: leading-only` |

`deepseek-flash` also declares text+image input. The models.dev catalog agrees on
1,048,576 context / 393,216 output and marks both models as reasoning + tool-capable.

Minimal live probes (single tiny request each, both HTTP 200):

- `POST https://api.deepseek.com/anthropic/v1/messages` with
  `output_config.effort: "low"` returned a `thinking` content block with a
  signature and `cache_creation_input_tokens` / `cache_read_input_tokens` usage
  fields.
- `POST https://api.deepseek.com/chat/completions` with `reasoning_effort: "low"`
  was accepted and returned `reasoning_content` plus
  `prompt_cache_hit_tokens` / `prompt_cache_miss_tokens`.

Observed `ort` gaps at the pinned revision:

- The bundled fallback catalog declares `deepseek-flash` at **128k context /
  32k output** (`crates/opencode-provider/src/bootstrap.rs`), and the dedicated
  `DeepSeekProvider` struct declares older `deepseek-chat`/`deepseek-reasoner`
  models at 64k/8k (`crates/opencode-provider/src/deepseek.rs`). Both are stale
  against the live listing. The models.dev path is correct, so the bug surfaces
  when the cache is absent/corrupt.
- `crates/opencode-provider/src/transform.rs` refuses to generate reasoning
  variants for any model id containing `deepseek`, so no effort selection exists.
- The default output cap is `OUTPUT_TOKEN_MAX = 32_000`, far below the model's
  393,216 output limit.
- The system prompt is rebuilt as leading system messages on every request
  (`crates/opencode-session/src/llm.rs`); there is no `in-history` update mode, so
  a dynamic-context change rewrites the cached prefix.

### Why this satisfies `RESEARCH-008`

The card's question is whether dsh should be integrated. The answer is **no**:
its harness-level advantages do not justify a second runtime, its DeepSeek
advantages are protocol-level and can be borrowed without it, and its default
egress conflicts with this repo's trust posture. The card's `Done when` A/B-run
requirement is superseded by explicit operator direction (2026-11-05): the
implementation of the adopted DeepSeek capabilities is the card's deliverable.
The comparison and decision are recorded here; implementation lives in
`handoffs/H-017-deepseek-api-efficiency-handoff.md`.

## 7. Residual open questions (not blockers)

1. Does the OpenAI-compatible `reasoning_effort` field change model behavior, or
   is it only accepted/ignored? (Probe with a bounded task before trusting it.)
2. Does the Anthropic transport measurably improve cache hit rate and latency over
   `/chat/completions` for the same workload?
3. Can cache-safe system updates be expressed on `/chat/completions` (append a
   system message) without switching transports?
4. Do in-history / addition-only semantics apply to `deepseek-v4-pro` (which
   declares `leading-only`), or only to `deepseek-flash`?

## 9. Provenance

- dsh docs read: `README.md`, `AGENTS.md`, `docs/architecture.md`,
  `docs/cordis-primer.md`, `docs/capability-seams.md`,
  `docs/deepseek-llm-api-wire-extensions.md`, `apps/cli/README.md`,
  `docs/subsystems/{core,session,tools,approval,sandbox,subagent,agent-team,workflow,goal,jobs,deliverables,compaction,lsp,mcp}.md`,
  `packages/boot/plugin-manager/README.md`, `packages/{bundle,core,session}/README.md`.
- OpenCode references: the `explore` pass over
  `packages/opencode/src/{session,server,tool,plugin,permission,acp}`,
  `packages/core/src/{session,tool,plugin,effect}`, `packages/llm/src`, and
  `packages/plugin/src`.
- No files were modified in either external checkout.

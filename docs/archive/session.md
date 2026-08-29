> Archived source material: this file is preserved as brainstorming/reference input for later planning extraction and may contain legacy names, outdated assumptions, or superseded direction.

*I like the idea that ollama supports opencode natively but I feel like opencode
may be a bottleneck because I cannot understand if the opencode agent harness
itself is as efficient as it can be. Can you please provide insight? For
example, I'm running Qwen3 30B on mac m4 and it cannot be used for complicated
spec-driven development. I am wondering if improving the agent harness portion
can help in this regard. For example, we can queue up several sub agent
processes at various parts of the session just to arrive at a better result and
eventually start managing context effectively with improvements to the
harness.*


Yes. Improving the harness can materially improve results from Qwen3 30B, particularly for spec-driven work. However, the likely ceiling is a combination of model capacity, context handling, and orchestration—not OpenCode alone.

One important detail is that `qwen3:30b` is Qwen3-30B-A3B: roughly 30.5B total parameters but only about 3.3B parameters activated for each token. ([Hugging Face][1]) That architecture gives excellent compute efficiency, but complicated software-engineering work can expose weaknesses that are less apparent from the “30B” label. A better harness can compensate significantly for those weaknesses, but it cannot completely transform the model into a frontier-scale reasoning model.

### Where OpenCode can become the bottleneck

For complicated development, the important unit is not:

> model → prompt → code

It is closer to:

```text
specification
     ↓
orchestrator
     ↓
task decomposition
     ↓
context acquisition
     ↓
reasoning / implementation
     ↓
verification
     ↓
critique
     ↓
repair
     ↓
state compression
     ↓
next task
```

A mediocre harness asks one model instance to continuously perform all of these roles inside one increasingly polluted context.

That approach is particularly punishing for smaller local models.

OpenCode already has some of the primitives required for a stronger architecture. Its current agent system supports primary agents and child-session subagents; subagents receive fresh context and can execute in the foreground or background. ([OpenCode][2]) There are also automatic compaction facilities, where old session history is replaced with a structured checkpoint while retaining a recent tail. ([OpenCode][3])

So the underlying machinery is reasonably capable. The question is more about **orchestration policy** than whether OpenCode has subagents at all.

### The architecture described would probably outperform a normal OpenCode session

For Qwen3 specifically, a structure resembling this would likely work much better:

```text
                         ┌──────────────┐
                         │ Specification│
                         └──────┬───────┘
                                │
                         ┌──────▼───────┐
                         │ Coordinator  │
                         │   Qwen3      │
                         └──────┬───────┘
                                │
              ┌─────────────────┼───────────────────┐
              │                 │                   │
       ┌──────▼──────┐   ┌──────▼─────┐    ┌──────▼──────┐
       │ Repo Scout  │   │ Spec Analyst│    │Dependency   │
       │ read-only   │   │ read-only   │    │Researcher   │
       └──────┬──────┘   └──────┬─────┘    └──────┬──────┘
              │                 │                   │
              └─────────────────┼───────────────────┘
                                │
                       structured findings
                                │
                         ┌──────▼───────┐
                         │ Planner      │
                         └──────┬───────┘
                                │
                       implementation DAG
                                │
                  ┌─────────────┼─────────────┐
                  │             │             │
             Worker A      Worker B      Worker C
                  │             │             │
                  └─────────────┼─────────────┘
                                │
                         ┌──────▼───────┐
                         │ Integrator   │
                         └──────┬───────┘
                                │
               ┌────────────────┼────────────────┐
               │                │                │
          Test Agent      Review Agent     Spec Checker
               │                │                │
               └────────────────┼────────────────┘
                                │
                           repair loop
```

The big improvement here is **not parallelism by itself**.

It is **context isolation**.

A repo scout does not need implementation history. A reviewer does not need every shell command from implementation. A worker modifying authentication does not need 30,000 tokens concerning database migrations.

Each subagent can therefore operate on perhaps 4–12K high-quality tokens instead of asking one Qwen context to distinguish useful information from 50K tokens of accumulated history.

That distinction is extremely important for smaller models.

### Think of context as working memory, not storage

A common mistake with local agents is increasing the context window indefinitely.

Qwen3-30B-A3B supports substantial context lengths—32K natively on the original model and larger windows using extended configurations. ([Hugging Face][1]) Ollama's current Qwen3 listing advertises a larger context configuration for its packaged 30B version. ([Ollama][4])

But:

```text
available context ≠ effectively usable context
```

For agentic coding, filling a 64K or 128K context can actually make a smaller model worse.

For example:

```text
SYSTEM
AGENTS.md
SPEC.md
architecture docs
20 source files
git diff
previous failed attempt
compiler output
test output
tool call history
conversation history
previous plan
updated plan
...
```

The model now has to continually determine which portions matter.

A better harness turns that into external state:

```text
.task/
    specification.md
    architecture.md
    decisions.json
    task-graph.json
    current-task.md
    findings/
        repo-map.md
        auth-analysis.md
        database-analysis.md
    verification/
        tests.md
        reviewer.md
```

The active model might then receive only:

```text
SYSTEM
current-task.md
relevant specification section
architecture constraints
3 relevant files
relevant previous decisions
```

That can be dramatically easier than presenting the whole session.

### This is also where OpenCode's default compaction is not necessarily sufficient

OpenCode's V2 compaction is fairly sensible. It creates a structured summary containing objectives, important details, completed/active work, blockers, next steps, and relevant files, while retaining recent context. ([OpenCode][3])

But compaction is inherently lossy.

More importantly, generic conversational compaction is not the ideal memory system for software engineering.

The stronger approach is **semantic state extraction**:

```json
{
  "requirements": [],
  "constraints": [],
  "architectural_decisions": [],
  "completed_tasks": [],
  "modified_files": [],
  "remaining_tasks": [],
  "known_failures": [],
  "test_status": {},
  "unresolved_questions": []
}
```

That state becomes authoritative.

Conversation history becomes disposable.

OpenCode actually exposes a compaction hook allowing plugins to inject additional context or replace the compaction prompt entirely. Its documentation even shows a multi-agent-session example that preserves modified files, blockers, dependencies, and next steps. ([OpenCode][5])

That is one of the first places worth experimenting.

### Parallel subagents should primarily be used for diversity, not speed

The proposed idea of queueing several subagents is especially interesting with Qwen3.

Suppose a difficult architectural decision arises.

Instead of:

```text
Qwen → decide architecture
```

run:

```text
Qwen A → propose implementation
Qwen B → look specifically for flaws
Qwen C → derive implementation from specification independently
Qwen D → inspect existing repository conventions
```

Then provide the coordinator with:

```text
A: proposed solution
B: objections
C: independent solution
D: compatibility findings
```

and ask for synthesis.

This is effectively **inference-time scaling**.

Multiple modest reasoning trajectories can outperform one long trajectory because correlated mistakes become easier to detect.

For local models this can be especially effective because token generation is inexpensive relative to cloud API usage.

There is an important limitation, though:

> Five instances of the same model do not automatically produce five independent expert opinions.

Prompts need deliberately differentiated roles.

Bad:

```text
Agent 1: solve this.
Agent 2: solve this.
Agent 3: solve this.
```

Better:

```text
Agent 1: derive the simplest implementation satisfying the spec.

Agent 2: assume Agent 1's likely approach is wrong; identify edge cases
and architectural violations.

Agent 3: inspect the repository and infer existing implementation patterns.
Do not design anything.

Agent 4: translate the specification into machine-checkable acceptance
criteria.
```

That creates useful diversity.

### A particularly good architecture for Qwen3 would be asymmetric

Not every agent needs equal authority.

A design like this is preferable:

```text
ORCHESTRATOR
    ↓
DISCOVERY SWARM
    ├─ repo structure
    ├─ relevant implementation
    ├─ tests
    └─ dependencies

    ↓

SPEC COMPILER
    ↓
requirements + acceptance criteria + constraints

    ↓

PLANNER
    ↓
dependency DAG

    ↓

EXECUTOR
    ↓
one bounded task

    ↓

VERIFICATION SWARM
    ├─ compiler/tests
    ├─ spec compliance
    ├─ regression inspection
    └─ code review

    ↓

REPAIR
```

Most importantly:

**The executor should not decide whether the executor succeeded.**

Smaller models are particularly susceptible to accepting their own flawed implementation once they have committed to a reasoning trajectory.

A fresh-context reviewer is therefore disproportionately valuable.

### OpenCode already gets surprisingly close to this

Its current documentation explicitly describes `general` subagents for multi-step work and `explore` as read-only repository exploration. Subagents run in separate child sessions with fresh context. ([OpenCode][6])

So OpenCode itself is not fundamentally incompatible with the proposed architecture.

There are nevertheless some reasons to consider building an orchestration layer around it.

One recent OpenCode issue, for example, reports `qwen3-coder:30b` through Ollama producing valid OpenAI-style tool calls that OpenCode Desktop displayed rather than executed. The reporter's direct Ollama tests worked correctly, suggesting at least that particular failure occurred in the harness/integration layer. ([GitHub][7])

Another reported problem involved orchestration prompt injection affecting a custom Qwen3-Coder 30B routing setup. ([GitHub][8])

Those are individual issue reports rather than proof of systemic inefficiency, but they demonstrate that the harness can absolutely affect observed model quality.

### There is another likely bottleneck on the M4: effective context configuration

Ollama currently recommends **64K+ context for coding tools**, and its OpenCode integration documentation explicitly says OpenCode requires a sufficiently large context window. ([GitHub][9])

On Apple Silicon, though, large context comes with KV-cache memory costs.

So there is an unpleasant tradeoff:

```text
larger context
      ↓
larger KV cache
      ↓
higher unified-memory pressure
      ↓
possible CPU spill / lower throughput
```

That means harness engineering offers a particularly attractive alternative:

```text
128K monolithic context
```

versus:

```text
8K coordinator
+
4 × 8K specialized child contexts
+
persistent structured state
```

The second system can consume more aggregate inference while requiring substantially less useful working context per reasoning step.

### A useful way to think about model + harness capability

A rough model is:

```text
Agent performance
≈
base reasoning
× context quality
× decomposition quality
× tool reliability
× verification quality
```

Consider illustrative numbers:

```text
Qwen3 reasoning             0.70
bad context management      0.55
weak decomposition          0.60
tool reliability            0.90
self-verification           0.55

effective reliability ≈ poor
```

Improving orchestration:

```text
Qwen3 reasoning             0.70
focused context             0.90
explicit decomposition      0.90
tool reliability            0.95
independent verification    0.85
```

does not increase the model's intelligence.

It increases the probability that available intelligence is applied to the right problem.

That distinction explains why harness engineering can produce surprisingly large gains.

### For complicated spec-driven development, the target should probably be a state machine rather than an autonomous chat agent

Something resembling:

```text
INGEST_SPEC
     ↓
DISCOVER_REPO
     ↓
NORMALIZE_REQUIREMENTS
     ↓
BUILD_TASK_GRAPH
     ↓
SELECT_TASK
     ↓
BUILD_CONTEXT
     ↓
IMPLEMENT
     ↓
TEST
     ↓
REVIEW
     ↓
   pass? ── no ──→ REPAIR
     │
    yes
     ↓
COMMIT_STATE
     ↓
COMPACT
     ↓
SELECT_TASK
```

The LLM makes bounded decisions inside that machine.

The harness owns the lifecycle.

That is almost the inverse of the common agent design where the LLM owns the lifecycle and occasionally invokes tools.

For a 3.3B-active MoE model, that architectural inversion could be very significant.

### The most promising experiment

Before abandoning OpenCode, building a fairly small experimental harness **on top of its existing agent/plugin system** would provide a useful comparison.

Start with only four roles:

| Agent       | Context               | Responsibility       |
| ----------- | --------------------- | -------------------- |
| Coordinator | spec + task state     | select next task     |
| Explorer    | fresh                 | locate relevant code |
| Implementer | task + relevant files | make change          |
| Reviewer    | fresh                 | test against spec    |

Then enforce:

```text
max task size:
    approximately one coherent code change

review:
    mandatory fresh context

state:
    persisted outside conversation

compaction:
    after every completed task

exploration:
    parallel where possible

implementation:
    serialized unless files are independent
```

No recursive swarm initially.

That alone would answer a major question:

> Is Qwen3 incapable of the workload, or is the monolithic agent loop incapable of reliably extracting Qwen3's capability?

The expectation is **a substantial improvement**, especially in adherence to long specifications, avoiding forgotten requirements, and recovering from mistakes.

The expectation would *not* be that Qwen3-30B-A3B suddenly behaves like Claude Opus/Codex-class models on arbitrary multi-hour autonomous work. Its 3.3B-active architecture still places a real reasoning ceiling on difficult ambiguity, architectural judgment, and novel debugging. ([Hugging Face][1])

But the gap between **“Qwen3 cannot do this”** and **“Qwen3 can do this when the problem is decomposed into independently verified bounded tasks”** could be quite large.

A particularly interesting next step would be designing a minimal **local coding-agent harness specifically optimized for Qwen3/Ollama/M4**, including the task DAG, context compiler, subagent queue, persistent memory format, reviewer loop, and OpenCode integration points. That would make it possible to compare the proposed architecture against stock OpenCode rather than discussing the difference abstractly.

[1]: https://huggingface.co/Qwen/Qwen3-30B-A3B?utm_source=chatgpt.com "Qwen/Qwen3-30B-A3B · Hugging Face"
[2]: https://opencode.ai/v2/docs/agents?utm_source=chatgpt.com "Agents | OpenCode"
[3]: https://opencode.ai/v2/docs/compaction?utm_source=chatgpt.com "Compaction | OpenCode"
[4]: https://ollama.com/library/qwen3?utm_source=chatgpt.com "qwen3"
[5]: https://dev.opencode.ai/docs/plugins/?utm_source=chatgpt.com "Plugins | OpenCode"
[6]: https://dev.opencode.ai/docs/agents/?utm_source=chatgpt.com "Agents | OpenCode"
[7]: https://github.com/anomalyco/opencode/issues/33959?utm_source=chatgpt.com "Bug: OpenCode Desktop does not execute valid OpenAI tool calls from qwen3-coder:30b (Ollama) · Issue #33959 · anomalyco/opencode · GitHub"
[8]: https://github.com/anomalyco/opencode/issues/27449?utm_source=chatgpt.com "Plan Mode Injection · Issue #27449 · anomalyco/opencode · GitHub"
[9]: https://github.com/ollama/ollama/blob/main/docs/integrations/opencode.mdx?utm_source=chatgpt.com "ollama/docs/integrations/opencode.mdx at main · ollama/ollama · GitHub"


*In that case, should I ditch the current opencode implementation, create my own
fork (either Rust or C++ for example I can fork this
https://github.com/ChrisFeldmeier/OpenCodeRust) and start from there?*

Yes. If committing to a custom runtime, developing the Rust implementation alongside a forked TypeScript OpenCode baseline is probably the strongest approach.

The important distinction is that the TypeScript fork should be treated as a **reference implementation and experimental control**, not as a second product that must receive every new feature.

Current OpenCode is mature enough that it contains a large amount of accumulated behavior that would be expensive to rediscover. The upstream repository now has more than 15,000 commits, and the current v2 work is explicitly separating domain state, typed contracts, plugins, providers, server behavior, and legacy compatibility. ([GitHub][1]) That makes it unusually useful as an executable specification.

A structure like this would work well:

```text
agent-project/
├── opencode-ts/
│   └── fork of upstream OpenCode
│
├── harness-rs/
│   ├── crates/
│   │   ├── agent-core/
│   │   ├── scheduler/
│   │   ├── context/
│   │   ├── memory/
│   │   ├── inference/
│   │   ├── tools/
│   │   ├── eval/
│   │   └── runtime/
│   └── ...
│
├── benchmarks/
│   ├── specs/
│   ├── repositories/
│   └── expected/
│
└── compatibility/
    ├── fixtures/
    ├── traces/
    └── protocol-tests/
```

The workflow would then be:

```text
                    same model
                    same repo
                    same spec
                    same tools
                       │
          ┌────────────┴────────────┐
          │                         │
   OpenCode TypeScript         Rust harness
          │                         │
          └────────────┬────────────┘
                       │
                    compare
                       │
       ┌───────────────┼────────────────┐
       │               │                │
   correctness      efficiency      behavior
```

This provides something extremely valuable: **differential testing**.

For example, suppose Qwen produces a tool call.

The same request can be sent through both runtimes:

```text
input
  ↓
┌────────────────────────────┐
│ TypeScript OpenCode        │
│ tool normalization         │
│ permissions                │
│ execution                  │
│ result serialization       │
└────────────────────────────┘

versus

┌────────────────────────────┐
│ Rust implementation        │
│ tool normalization         │
│ permissions                │
│ execution                  │
│ result serialization       │
└────────────────────────────┘
```

Any divergence immediately becomes visible.

That is much safer than trying to reproduce OpenCode from documentation.

## The TypeScript fork also becomes an architecture laboratory

There is another advantage.

Agent ideas can be implemented rapidly in TypeScript first:

```text
new scheduler policy
new compaction strategy
new context compiler
new reviewer loop
new task state model
```

Then benchmarked.

Only successful ideas need to move into Rust.

So development becomes:

```text
idea
 ↓
TS prototype
 ↓
benchmark
 ↓
worthwhile?
 ├── no → discard
 └── yes
      ↓
  Rust implementation
```

That prevents Rust from becoming the place where both **agent research** and **systems engineering** happen simultaneously.

That separation matters.

Otherwise every failed experiment raises an ambiguous question:

```text
Was the idea bad?

or

Was the Rust implementation wrong?
```

With the TypeScript control, that ambiguity disappears.

## Avoid making the Rust implementation a literal port

The Rust project should not attempt:

```text
packages/opencode/src/session/foo.ts
        ↓
crates/session/src/foo.rs
```

That would inherit architectural decisions that may be precisely what the new project is intended to investigate.

Instead, establish behavioral contracts.

For example:

```rust
pub trait ModelProvider {
    async fn generate(
        &self,
        request: ModelRequest,
    ) -> Result<ModelResponse, ProviderError>;
}

pub trait ToolExecutor {
    async fn execute(
        &self,
        invocation: ToolInvocation,
    ) -> Result<ToolResult, ToolError>;
}

pub trait ContextCompiler {
    async fn compile(
        &self,
        task: &Task,
        role: AgentRole,
        budget: TokenBudget,
    ) -> Result<CompiledContext, ContextError>;
}
```

Then OpenCode tells us what behavior needs compatibility.

Rust decides how that behavior should be implemented.

This is especially appropriate because upstream OpenCode itself is moving toward smaller typed core services with policy pushed outward into plugins. ([GitHub][2]) There is therefore little benefit in tightly cloning implementation details that upstream is already changing.

## Maintain three compatibility levels

Not every subsystem needs parity.

### Level 1 — exact compatibility

These should probably behave identically:

```text
provider request formatting
stream parsing
tool schemas
tool-call decoding
filesystem semantics
shell execution semantics
permissions
model metadata
session/event protocol where consumed externally
```

These are infrastructure.

Unexpected differences produce hard-to-debug model behavior.

### Level 2 — semantic compatibility

These need equivalent outcomes, but implementation can differ:

```text
sessions
message storage
MCP
LSP
configuration
project discovery
git integration
```

For example, the Rust implementation does not need OpenCode's exact database schema.

It needs equivalent capabilities.

### Level 3 — deliberately incompatible

This is where the project should innovate:

```text
agent lifecycle
context management
subagent scheduling
memory
planning
task decomposition
verification
compaction
inference scheduling
```

These areas are the reason to build the project.

Trying to maintain OpenCode parity here would defeat the experiment.

## Keep the TypeScript fork close to upstream

A fork can become a maintenance disaster if modified indiscriminately.

A better model is:

```text
upstream/opencode
      │
      │ merge/rebase regularly
      ↓
our/opencode-reference
      │
      ├── minimal instrumentation
      ├── experimental harness hooks
      └── benchmark adapters
```

Avoid large unrelated changes.

The ideal fork contains patches such as:

```text
+ structured tracing
+ deterministic benchmark mode
+ context dump hooks
+ inference event capture
+ scheduler experiments
```

rather than:

```text
- replace UI
- rewrite config
- change database
- restructure packages
- rename everything
```

That keeps upstream merges manageable.

## Instrumentation should be shared between both implementations

A common event schema would be useful:

```json
{
  "run_id": "run-42",
  "agent_id": "reviewer-1",
  "task_id": "task-7",
  "event": "model.completed",
  "model": "qwen3:30b",
  "context_tokens": 9142,
  "output_tokens": 1834,
  "duration_ms": 12743
}
```

Other events:

```text
agent.started
agent.completed

context.compiled
context.compacted

model.started
model.completed

tool.requested
tool.completed
tool.failed

task.created
task.started
task.completed
task.reopened

verification.started
verification.failed
verification.passed
```

Then traces can be compared automatically:

```text
benchmark run
      │
      ├── ts.trace.jsonl
      └── rs.trace.jsonl
               │
               ↓
          trace analyzer
```

This would expose things such as:

```text
TS:
43 model calls
721K aggregate input tokens
17 context compactions
3 failed tool calls

Rust:
31 model calls
402K aggregate input tokens
8 context projections
1 failed tool call
```

That is much more useful than subjective impressions of which agent "felt smarter."

## There is another important reason to keep OpenCode around

Some apparent model deficiencies are actually harness bugs.

OpenCode's own issue history contains examples where diagnostics, tool handling, server protocol behavior, and integration details affected what the agent received or could execute. For instance, a reported TypeScript diagnostics issue showed file reads and patches succeeding while diagnostics were not surfaced into agent context. ([GitHub][3]) A recent Desktop regression also involved client/server protocol handling rather than the model. ([GitHub][4])

Having both implementations allows an immediate question:

```text
Does this fail in Rust too?
```

If yes:

```text
probably model / prompt / task
```

If no:

```text
probably harness / integration
```

That is a powerful debugging capability.

## The Rust project can eventually become the primary system

The likely evolution would be:

```text
Phase 1

OpenCode TS       ████████████████████
Rust              ████
```

Rust implements only:

```text
Ollama
basic tools
scheduler
context compiler
state store
```

Then:

```text
Phase 2

OpenCode TS       ████████████████████
Rust              ████████████
```

Rust gains:

```text
MCP
LSP
permissions
provider abstraction
session persistence
```

Eventually:

```text
Phase 3

OpenCode TS       reference/testing only
Rust              ████████████████████
```

At that point the TypeScript fork becomes something like Chromium's reference behavior during an engine implementation: still extremely valuable, but not part of the normal execution path.

## OpenCode's current v2 direction actually makes this easier

There is a useful alignment with upstream.

OpenCode's current architectural documentation says `packages/core` is intended to hold domain schemas, typed errors, state containers, events, and plugin contracts, while application-specific behavior moves outward. ([GitHub][2]) Its client-contract documentation similarly separates lightweight schemas, protocol definitions, server adaptation, and core behavior. ([GitHub][5])

That suggests an architectural boundary for the Rust project:

```text
OpenCode concepts

Schema
Protocol
Core
Server
Providers
Tools

        │
        │ compatibility boundary
        ▼

Rust infrastructure

        │
        │ innovation boundary
        ▼

Agent Kernel
Scheduler
Context Compiler
Memory
Task Graph
Verification
```

In other words, borrow the mature **plumbing contracts**, but reconsider the **cognitive architecture**.

## Recommendation

The strongest route would be:

```text
Fork current OpenCode TypeScript
        +
start a clean Rust implementation
        +
build one shared benchmark suite
        +
build one shared event/trace schema
        +
maintain behavioral compatibility tests
```

Then use the TypeScript implementation for two purposes:

```text
1. Golden/reference behavior for infrastructure.
2. Fast experimental environment for agent ideas.
```

The Rust implementation becomes:

```text
1. The long-term runtime.
2. The place for scheduling/context innovations.
3. The optimized local-model architecture.
```

That approach avoids throwing away several years of OpenCode engineering while also avoiding the trap of inheriting every OpenCode architectural decision.

For this particular project, that is substantially stronger than either **“fork OpenCode and keep modifying TypeScript forever”** or **“throw OpenCode away and rebuild everything in Rust from memory.”**

[1]: https://github.com/anomalyco/opencode?utm_source=chatgpt.com "GitHub - anomalyco/opencode: The open source coding agent. · GitHub"
[2]: https://github.com/anomalyco/opencode/blob/dev/specs/v2/instructions.md?utm_source=chatgpt.com "opencode/specs/v2/instructions.md at dev · anomalyco/opencode · GitHub"
[3]: https://github.com/anomalyco/opencode/issues/16880?utm_source=chatgpt.com "TypeScript diagnostics are not surfaced to agent/tool context on read or patch · Issue #16880 · anomalyco/opencode · GitHub"
[4]: https://github.com/anomalyco/opencode/issues/38789?utm_source=chatgpt.com "[Bug] Desktop v1.18.5: UnsupportedContentType error on project reload after update · Issue #38789 · anomalyco/opencode · GitHub"
[5]: https://github.com/anomalyco/opencode/blob/dev/CONTEXT.md?utm_source=chatgpt.com "opencode/CONTEXT.md at dev · anomalyco/opencode · GitHub"


*I also have a separate project scopemux (native C, treesitter, and python
bindings), could that be leveraged in the custom rust ?*

Yes. A native-C/tree-sitter/Python project like ScopeMux could be very useful inside a custom Rust harness, especially for **context construction, code navigation, symbol extraction, and structural retrieval**.

The strongest role would be to make ScopeMux the harness’s **code intelligence substrate**, while Rust owns orchestration.

```text
Rust agent runtime
    |
    +-- scheduler
    +-- task graph
    +-- memory/state
    +-- model providers
    +-- tool execution
    +-- context compiler
            |
            v
        ScopeMux
            |
            +-- tree-sitter parsing
            +-- symbol extraction
            +-- scope relationships
            +-- structural queries
            +-- source slicing
```

That combination is potentially much stronger than letting the LLM search the repository primarily through `grep`, file reads, or broad text retrieval.

### Where ScopeMux could add the most value

The most important use case is probably **context compilation**.

Instead of an agent saying:

```text
read src/auth/service.ts
```

and receiving the entire file, the Rust harness could request:

```text
symbol: AuthService.login
include:
  - containing type
  - referenced local symbols
  - interface definitions
  - callers within depth 1
  - associated tests
token_budget: 6000
```

ScopeMux could return a structurally meaningful slice.

That changes context building from:

```text
file retrieval
```

into:

```text
program-structure retrieval
```

For smaller local models, that distinction can be extremely valuable.

### A useful API boundary

The Rust side should ideally not know tree-sitter implementation details.

A compact ScopeMux API might expose concepts such as:

```rust
pub struct Symbol {
    pub id: SymbolId,
    pub name: String,
    pub kind: SymbolKind,
    pub file: PathBuf,
    pub byte_range: Range<usize>,
}

pub struct Scope {
    pub id: ScopeId,
    pub parent: Option<ScopeId>,
    pub symbols: Vec<SymbolId>,
}

pub struct Reference {
    pub source: SymbolId,
    pub target: SymbolId,
    pub kind: ReferenceKind,
}

pub struct ContextSlice {
    pub source: String,
    pub symbols: Vec<SymbolId>,
    pub estimated_tokens: usize,
}
```

And operations such as:

```rust
find_symbol(...)
find_references(...)
find_callers(...)
find_callees(...)
containing_scope(...)
related_tests(...)
slice_for_symbol(...)
slice_for_task(...)
```

The C library could remain responsible for parsing and structural analysis.

Rust would consume the resulting graph.

### FFI is probably the right integration path

Since ScopeMux is native C already, rewriting it in Rust would likely provide little benefit unless the current API is unsuitable.

A reasonable stack would be:

```text
ScopeMux C library
       |
       | C ABI
       v
Rust scopemux-sys
       |
       v
Rust scopemux
       |
       v
agent-context
```

For example:

```text
crates/
    scopemux-sys/
        bindings.rs

    scopemux/
        repository.rs
        symbols.rs
        queries.rs

    context/
        compiler.rs
        ranking.rs
        budget.rs
```

`scopemux-sys` remains unsafe and thin.

`scopemux` exposes safe Rust types.

The rest of the harness never interacts with raw pointers or C allocation semantics.

### Python bindings could still remain useful

The Python bindings do not need to participate in the production runtime.

They could instead become extremely useful for:

* prototyping retrieval algorithms
* building benchmark datasets
* inspecting symbol graphs
* experimenting with ranking heuristics
* generating repository analysis reports
* evaluating context-selection quality

That gives a convenient research loop:

```text
Python
  ↓
experiment with ScopeMux query
  ↓
validate retrieval strategy
  ↓
move stable strategy into Rust
```

This is similar to the earlier TypeScript/Rust split: fast experimentation in a high-level environment, stable runtime behavior in Rust.

### ScopeMux could become the basis of a repository graph

This is where the project gets more interesting.

If ScopeMux can identify scopes, symbols, and references, the harness can construct something like:

```text
RepositoryGraph

File
 |
 +-- Scope
      |
      +-- Symbol
           |
           +-- references
           +-- callers
           +-- callees
           +-- implementations
           +-- tests
           +-- imports
```

Then task context can be graph-driven.

Example:

```text
Task:
"Add refresh-token rotation"

Seed symbols:
AuthService
TokenRepository
RefreshToken

Expand:
1 hop callers
1 hop implementations
related interfaces
related tests

Exclude:
unrelated UI
generated files
vendor code

Budget:
10,000 tokens
```

That is a much better retrieval mechanism than dumping every file containing `token`.

### It could also improve task decomposition

ScopeMux does not need to be limited to context retrieval.

Suppose a specification mentions:

```text
authentication service
token storage
API handler
database model
tests
```

The planner could query ScopeMux and receive:

```text
AuthController
    -> AuthService
        -> TokenRepository
            -> RefreshTokenModel

Tests:
AuthControllerTest
AuthServiceTest
```

The scheduler can then derive a task graph:

```text
migration
   ↓
repository
   ↓
service
   ↓
controller
   ↓
tests
```

So ScopeMux can inform both:

```text
what code matters
```

and:

```text
what implementation order makes sense
```

### Another strong use case: change-impact analysis

After an implementation agent modifies a symbol:

```text
TokenRepository.rotate()
```

ScopeMux could calculate affected structural neighborhoods:

```text
direct callers
interface implementations
test references
serialization consumers
public API exposure
```

The reviewer receives that information before verification.

Example:

```text
Changed:
TokenRepository.rotate

Potentially affected:
AuthService.refresh
AuthController.refresh
TokenRepositoryMock
AuthServiceTest
RefreshTokenIntegrationTest
```

That can make fresh-context review much more effective.

### Context could become hierarchical

One particularly useful architecture would be a three-level retrieval model.

```text
Level 1: repository map

src/auth/
  controller
  service
  repository

src/user/
...

Level 2: symbols

AuthService
  login
  refresh
  revoke

Level 3: source

exact function/type bodies
```

The model first receives the coarse map.

Only relevant branches get expanded.

That resembles progressive disclosure:

```text
repo
 ↓
module
 ↓
symbol
 ↓
source
```

Smaller models benefit because irrelevant source is never introduced into context.

### Tree-sitter has an important limitation

Tree-sitter provides syntax exceptionally well, but it is not automatically a complete semantic engine.

For many languages:

```text
foo.bar()
```

cannot always be resolved purely from syntax into:

```text
crate::auth::AuthService::bar
```

without type information.

So ScopeMux should probably distinguish clearly between:

```text
syntactic relationship
```

and:

```text
resolved semantic relationship
```

For example:

```rust
enum ReferenceConfidence {
    Exact,
    Inferred,
    Ambiguous,
}
```

This matters because the agent harness should not treat heuristic reference resolution as ground truth.

For languages with mature LSPs, a hybrid architecture could work extremely well:

```text
Tree-sitter / ScopeMux
    ↓
fast local structural index

LSP
    ↓
semantic resolution when required
```

ScopeMux handles the common case cheaply.

LSP handles expensive semantic questions.

### This could remove a large amount of LLM work

Without ScopeMux:

```text
Agent:
search
read
search again
read another file
inspect imports
read interface
search tests
```

Several model calls may be spent simply figuring out repository topology.

With ScopeMux:

```text
Harness:
query repository graph

Agent receives:
- target symbol
- containing module
- interface
- callers
- implementation
- relevant tests
```

That can reduce both token consumption and reasoning burden.

This is exactly the sort of harness improvement that can disproportionately help Qwen3-30B.

### ScopeMux could become a first-class tool

Instead of exposing only generic shell/file tools:

```text
read_file
grep
glob
shell
```

the custom harness could expose:

```text
find_symbol
inspect_symbol
find_callers
find_references
find_implementations
find_related_tests
inspect_scope
build_context
analyze_change_impact
```

The agent then operates on programming-language concepts instead of filesystem primitives.

That is a significant architectural upgrade.

### Potential architecture

A full system could eventually resemble:

```text
                  SPEC
                   |
                   v
            Requirement Engine
                   |
                   v
              Task Graph
                   |
                   v
              Scheduler
                   |
        +----------+----------+
        |                     |
        v                     v
  Context Compiler       Verification
        |                     |
        v                     |
     ScopeMux ----------------+
        |
   +----+-----+
   |          |
Tree-sitter   LSP
   |
Repository Graph
        |
        v
    Agent Run
        |
        v
      Ollama
        |
        v
   Qwen3 / others
```

In that system, ScopeMux is not merely another tool.

It becomes part of the **agent operating system**.

### One design choice worth making early

Avoid allowing agents to directly determine arbitrary ScopeMux queries indefinitely.

Prefer the harness to provide higher-level operations.

For example, instead of:

```text
query AST node type X where child Y...
```

expose:

```text
build_context(task_id)
```

Internally:

```text
task requirements
    ↓
seed symbols
    ↓
graph expansion
    ↓
relevance ranking
    ↓
token budgeting
    ↓
source extraction
```

This keeps repository intelligence deterministic and reduces wasted model turns.

### Recommended division of responsibilities

A clean boundary would be:

| Component             | Responsibility                                     |
| --------------------- | -------------------------------------------------- |
| ScopeMux C            | Parsing, scopes, symbols, structural relationships |
| ScopeMux Python       | Research, analysis, retrieval experiments          |
| Rust wrapper          | Safe API and lifecycle                             |
| Rust repository index | Persistent project graph                           |
| Rust context compiler | Select relevant information under token budget     |
| Rust scheduler        | Determine which agent receives which context       |
| LLM                   | Reasoning, planning, implementation, review        |
| LSP                   | Semantic/type-aware enrichment where needed        |

That would reuse ScopeMux in a way that directly addresses the earlier concern: **making a smaller local model operate on higher-quality, narrower, structurally meaningful context rather than asking the model to discover everything through conversation.**

If ScopeMux already exposes symbol/scope/reference information beyond raw tree-sitter AST traversal, it may end up being one of the most strategically valuable pieces of the custom harness.



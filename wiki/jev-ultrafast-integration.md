# Jev UltraFast Integration

## Purpose

Define what `Jev UltraFast` is, what an interactive browser-automation capability could offer `scopemux-code`, the candidate integration surfaces, and the boundary and guardrails that would apply if it were ever adopted. This document is recorded for triage only.

## Status: Not Planned for Integration

Jev is **not planned for integration** at this time.

- No phase (V1, V2, V3, or later) commits to browser automation, and no board item tracks Jev adoption.
- MCP is an explicit V1 and V2 non-goal (`wiki/v1.md`, `wiki/v2.md`), which removes the lowest-coupling integration path.
- This document exists so the option is documented and comparable later. It does not create work and does not reserve scope.
- Adopting Jev would require an explicit product decision and a new board item; until then the runtime has no Jev dependency of any kind.

## What Jev UltraFast Is

Jev is a **browser agent with a dynamic, indexed action space**, built by Browser Use on top of TypeSafe's speculative fan-out.

- One natural-language goal drives the whole run. There are no site-specific scripts and no prepared field strings in the policy.
- Every page observation builds an indexed element table. One TypeSafe request then picks an **operation** and an operation-specific **target** in a single round trip.
- Operations: `CLICK`, `TYPE_TEXT`, `SELECT`, `SCROLL_UP`, `SCROLL_DOWN`, `WAIT`, `DONE`, `BLOCKED`.
- A small OpenAI-compatible LLM writes text **only** when the operation is `TYPE_TEXT`.
- Reference demo: Zürich → London on Google Flights in about 7.1 s, including generated text and loading waits.

Reference: `~/repos/jev-ultrafast` (fork of upstream `https://github.com/browser-use/jev-ultrafast`), package `0.1.0`, MIT (Copyright Browser Use), Python `>=3.12`, inspected at commit `1231850` (2026-09-18, branch `main`).

Key files:

| File | Job |
| --- | --- |
| `jev_ultrafast/agent.py` | the complete loop and text-helper handoff |
| `jev_ultrafast/snapshot.js` | atomic DOM snapshot, indexed controls, freshness guards |
| `jev_ultrafast/browser.py` | Browser Harness/CDP connection, geometry, execution |
| `jev_ultrafast/model.py` | TypeSafe operation/target heads and text generation |
| `jev_ultrafast/questions.py` | policy instructions; `MAX_STEPS = 60` |
| `jev_ultrafast/demo.py` | local loopback-only inspector |

## How It Works

The loop is `observe → choose → act`, and the executor only ever acts on code-owned observed state:

- **Observe.** One browser-side DOM snapshot supplies roles, names, values, visible text, and executable targets. A `WeakMap` gives each real DOM node a code-owned identity; replaced nodes get new identities and navigation starts a new cache. Geometry is re-read immediately before input.
- **Choose.** `model.py` asks TypeSafe at `https://api.typesafe.ai/v1/systemone` (`TYPESAFE_MODEL`, default `jev-latest`) for an `operation` choice plus one target head per available operation. Only the target head matching the selected operation is consumed.
- **Act.** `browser.py` rechecks target visibility, enabled state, geometry, and click occlusion, then dispatches input through Browser Harness (one CDP session, no per-step subprocess). Mutations are never retried by transport recovery; execution is logged before the next observation.
- **Text.** `TYPE_TEXT` calls an OpenAI-compatible chat endpoint (default `https://api.deepseek.com/v1`, `deepseek-chat`; the README example uses `inception/mercury-2.5` over OpenRouter) whose output must parse as a small JSON object with exactly one `text` key.

Model output never becomes selectors, coordinates, shell commands, or executable JavaScript. A `DONE` choice is not treated as proof of success; the example verifies the final route independently.

## Why It Might Matter Here (Hypothetical)

`scopemux-code` has read-only web access (`webfetch`, `websearch` in `crates/opencode-tool`) but no way to drive an interactive browser. A browser agent could theoretically serve workflows such as:

- filling and submitting web forms as part of a task
- navigating authenticated or multi-step sites that `webfetch` cannot
- independently confirming that an external web outcome actually occurred

None of these are part of the narrow V1 coding workflow. They are recorded as hypothetical value, not as a committed capability.

## Candidate Integration Surfaces

| Jev capability | Possible product need | How it could plug in |
| --- | --- | --- |
| Jev as an agent tool | let the agent perform an interactive web task | wrap Jev behind the tool registry (`crates/opencode-tool`) as a new browser tool |
| Jev as a local sidecar | runtime-driven browser actions without model-in-the-loop tools | Rust client calls a Jev HTTP process behind a browser-provider boundary |
| MCP wrapper | expose Jev over MCP | run a wrapper server and register it via `crates/opencode-mcp` (blocked: MCP is a V1/V2 non-goal) |
| Dynamic operation + target policy | choose an action from an indexed, constantly changing option space | borrow the *pattern* for runtime action selection (no Jev dependency) |

Jev ships no MCP server and no Rust binding, so any path beyond conceptual borrowing requires wrapping a Python 3.12 process.

## Why It Does Not Fit the Current Stance

- **Hosted dependency.** The core policy is a hosted TypeSafe API (`api.typesafe.ai`), and text generation defaults to a hosted chat model. This conflicts with the product's local-first posture; the product-owned default remains `deepseek/deepseek-flash`.
- **Heavy browser runtime.** Jev needs Chrome plus Browser Harness with remote debugging, and it drives an owned tab in the user's existing Chrome profile. That is a real operational and isolation cost inside a Rust product.
- **Language and packaging.** There is no Rust binding; every path runs a Python `>=3.12` sidecar with `browser-harness==0.1.13`.
- **Scope.** Interactive browser automation is far outside the narrow V1 coding daily-driver, and the nearest clean boundary (MCP) is explicitly deferred.
- **Security surface.** It feeds untrusted page text to models, handles credentials/sessions, and shares a real browser profile with the user.

## Required Abstraction Boundary If Ever Adopted

If Jev were ever adopted, it must enter behind the existing tool and permission boundaries, never as a hard-wired dependency:

1. The runtime defines a bounded browser request from task state (goal, target URL or starting page, allowed scope, timeout/action budget).
2. A browser provider returns an observed result plus provenance (final URL, visible evidence, actions taken, failures).
3. The runtime decides how to act on the result — the browser provider never owns task state, lifecycle, verification, or completion.

The rule is that orchestration code consumes browser results; it must not embed Jev-specific calls throughout lifecycle, verification, or permission logic. The provider must be optional and non-blocking, so the product behaves identically when Jev is absent.

## Guardrails and Invariants

Jev output is **evidence, not authority**. The following stay with the runtime:

- `invariants/runtime-lifecycle.md`: the runtime owns lifecycle transitions; a browser provider only performs a bounded action inside it.
- `invariants/task-state.md`: objective, criteria, stage, verification plan, and review result remain runtime-owned structured state.
- `invariants/verification.md`: a Jev `DONE` is not proof. The final web outcome must be verified independently.
- Permission handling: any browser tool is subject to the configured ruleset (`crates/opencode-permission`); it can never auto-approve an action the ruleset would `Ask` or `Deny`.
- Secrets: credentials and profile data stay server-side and must not enter model context.
- Untrusted content: page text is data, never instructions.

Additional operational rules if adopted:

- keep Jev optional and non-blocking; no browser provider is started unless explicitly configured
- pin the Jev fork commit; do not track upstream automatically
- treat page/model actions as untrusted and log execution before observing results

## Honest Limits

- TypeSafe is a hosted service; the text helper is a second hosted dependency.
- Name resolution covers common labels, ARIA references, and text, not the full accessibility algorithm.
- Shadow roots, frames, canvas, uploads, nested scrolling, pop-up tabs, and complex keyboard widgets are outside the current MVP.
- Runs are bounded to 60 browser actions and 120 decision requests (`docs/design.md`).
- Performance evidence is three repeats of one browser profile on one task, not a general reliability benchmark.
- A valid action can still be wrong, and owned tabs share the existing Chrome profile.

## Phase Guidance

### V1

- Do not add Jev, Browser Harness, or any browser provider.
- Keep `webfetch`/`websearch` as the only web surfaces.
- Do not build a browser-provider boundary speculatively.

### V2

- Still not planned. MCP remains a non-goal, and reliability work stays focused on coding tasks.

### V3+

- Reconsider only if a real personal workflow requires interactive browser automation and it can meet the local-first and security bar.
- Any adoption requires an explicit board item, a provider boundary, and comparison against the no-Jev baseline using `wiki/agent-evaluation-strategy.md`.
- No current phase reserves scope for it.

## Non-Goals

- adding Jev as a V1/V2 dependency
- an MCP wrapper while MCP remains deferred
- interactive browser automation as a core coding feature
- a native Rust port of the Jev policy or TypeSafe client
- shipping Python, Chrome, and Browser Harness as hard requirements
- automatic upstream tracking of the Jev fork
- replacing `webfetch`/`websearch` for simple reads

## Bottom Line

Jev UltraFast is an impressive, compact browser agent, but it is **not planned for integration**. It carries a hosted policy dependency, a Chrome/Browser-Harness runtime, and a Python sidecar, and it targets a workflow outside the narrow V1/V2 coding focus. It is documented here so a future decision can be made deliberately, behind the tool and permission boundaries and with runtime authority over verification and completion intact.

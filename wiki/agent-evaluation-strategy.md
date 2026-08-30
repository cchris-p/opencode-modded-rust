# Agent Evaluation Strategy

## Purpose

Define how `scopemux-code` judges whether the runtime is getting better at real coding work without depending on TypeScript-vs-Rust benchmark comparisons.

## Principles

- Evaluate the Rust product against its own stated goals in `wiki/v1.md`, `wiki/v2.md`, and `wiki/v3.md`.
- Prefer bounded real-work tasks over synthetic benchmark prompts.
- Treat verification and fresh-context review as primary quality gates, not optional afterthoughts.
- Measure reliability first, then depth and workflow breadth, and only then speed.
- Record failures in a way that creates follow-up engineering work instead of hand-wavy impressions.

## What gets evaluated

### V1

V1 evaluation should focus on the narrow daily-driver loop:

- inspect a local repository
- choose one bounded implementation task
- build focused context for that task
- make the change
- run task-relevant verification
- run a fresh-context review
- decide whether the task is complete or must be reopened

The V1 task set should be small, repeatable, and grounded in real repository work such as:

- a focused bug fix
- a small feature completion
- a docs-plus-code alignment task
- a targeted test repair or missing-test addition

### V2

V2 evaluation should extend the same task family to larger chains:

- tasks that require decomposition into substeps
- tasks that need repair after an initial failed attempt
- tasks that require better repository discovery before editing
- tasks that exercise cleaner handoff between planning, implementation, and review

### V3

V3 evaluation should expand coverage only where real use has shown demand:

- wider workflow types that matter in everyday use
- selected higher-value reference capabilities adopted into the Rust product
- broader repository-intelligence and ergonomics improvements

## Task corpus shape

The evaluation corpus should be organized as named task packs rather than one-off anecdotes.

Each task should include:

- a clear objective
- explicit completion criteria
- the repository or fixture it runs against
- expected verification commands
- expected review focus
- a short note describing why the task belongs in V1, V2, or V3

The corpus should stay intentionally small at first. A smaller stable set is more useful than a large drifting set that no one reruns.

## How task success is judged

Task success is binary at the top level: pass or fail.

A task passes only when all of the following are true:

- the produced result satisfies the stated task objective
- required verification actually ran
- verification results were interpreted correctly
- fresh-context review does not identify a blocking problem
- the runtime reaches completion through its explicit lifecycle rather than by informal human judgment alone

Useful partial signals should still be recorded, but they do not convert a failed task into a pass. Examples:

- correct diagnosis but incomplete implementation
- correct code change with missing verification
- successful implementation with a review-found regression

## How review quality is judged

Review quality should be evaluated separately from implementation success.

A review run is strong when it:

- uses fresh context instead of leaning on the implementation transcript
- checks output against the task objective and completion criteria
- identifies real regressions, missing tests, or scope mistakes
- avoids flooding the user with low-value commentary
- correctly allows clean work to pass when no blocking issue exists

Review quality should be tracked with a simple rubric:

- caught a real blocking issue
- caught a real non-blocking issue
- missed a later-confirmed issue
- raised a false blocking issue
- raised only low-value noise
- correctly approved a sound change

This keeps review evaluation tied to outcomes instead of style preferences.

## How regressions are detected

Regression detection should happen at three levels.

### Task-pack reruns

Re-run a stable evaluation task pack after meaningful runtime changes and compare:

- pass/fail outcome
- where the task failed in the lifecycle
- whether review quality improved or regressed

### Workflow-stage failure tracking

Record the failure stage for each failed run, such as:

- repository discovery
- task framing
- context construction
- implementation
- verification
- review
- repair loop

This helps distinguish model weakness from runtime orchestration weakness.

### Real-use sampling

Periodically sample real personal-use tasks and classify them with the same evaluation language used by the task packs.

This prevents the system from optimizing only for a canned benchmark set.

## Signals that matter more than speed

These signals outrank raw speed for V1 through V3:

- completion rate on bounded real tasks
- correctness after verification and review
- ability to recover from an initial failed attempt
- reviewer usefulness in fresh context
- stability of task-state and lifecycle transitions
- context quality relative to task scope
- consistency across repeated runs of the same task pack

Speed is still useful, but mainly as a secondary tie-breaker once reliability is acceptable.

## Suggested scorecard

Each evaluation run should produce a compact scorecard containing:

- task ID and version target
- pass or fail
- failure stage if failed
- verification status
- review outcome
- reopen required or not
- short notes on the root cause of failure or success

This should be simple enough to maintain manually at first and easy to structure later.

## Versioned expectations

### V1 success pattern

- most evaluation effort is manual and small-scale
- bounded tasks complete reliably enough for real personal use
- verification and fresh-context review are consistently part of the loop

### V2 success pattern

- multi-step tasks pass more often without collapsing into transcript sprawl
- repair loops improve failed tasks instead of compounding errors
- review catches more real issues with less noise

### V3 success pattern

- broader workflow coverage remains reliable under the same evaluation language
- new adopted capabilities are added to the corpus only when they improve real use
- evaluation evidence drives feature uptake and prioritization

## Immediate follow-up work

This strategy implies at least four concrete follow-ups:

- define the first stable V1 evaluation task pack
- define a reusable review-quality rubric and review fixtures
- persist structured evaluation results so regressions can be compared over time
- add a lightweight evaluation runner or operator workflow for repeated reruns

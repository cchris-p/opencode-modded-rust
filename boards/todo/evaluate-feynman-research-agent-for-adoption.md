---
id: "FEAT-065"
title: "Explore Feynman research-agent uses and decide whether to adopt into ort"
priority: "P3"
type: "research"
area: "FEAT"
spec: ""
status: "todo"
created: "2026-09-26"
---

# Explore Feynman research-agent uses and decide whether to adopt into ort

## Summary

Document what the `feynman` research agent actually does, how its research workflows and skills are exercised in practice, and whether any of that should be planned for adoption in `ort` (this Rust product). This is a learn-and-decide item, not an implementation item.

The immediate and currently only concrete use case is research work against the private `aa-studies` repository. Any adoption question is secondary to proving the tool is useful for that one workflow first.

## Why this exists

`feynman` is a separate, upstream TypeScript research agent that overlaps with the "research/planning" surface this product already gestures at (skills, subagents, verification). `cchris-p/feynman-modded` is a custom fork that is synced periodically from upstream, so it is a moving target and not a dependency. Before treating any of it as a source of product direction, we need one grounded write-up of what it offers and a small, real evaluation against `aa-studies`.

This item also records and exercises a standing verification habit: external-project facts must be checked with `gh` rather than trusted from memory or stale local checkouts.

## Standard practice: verify external projects with `gh`

All facts about `feynman`, its fork, and the use-case repos must be verified through the GitHub API via `gh`, and the exact command plus observed result recorded in this item. Do not rely on the README, local checkout, memory, or a stale branch for external-project claims.

- Existence, visibility, and ownership: `gh repo view <owner>/<repo>`.
- Fork relationship and lineage: confirm `isFork` and `parent` (do not assume the fork tracks the repo named in its README).
- Activity and freshness: `updatedAt`, latest release/tag, and default branch.
- Upstream license before any adoption discussion: `licenseInfo`.
- The use-case repo must be verified as reachable by the authenticated `gh` user (`cchris-p`).
- Record the command and its observed output (or a tight summary) at the point of each claim; re-verify when this item is revisited, since the fork is synced periodically.

Facts verified for this item on 2026-09-26 (re-verify before reuse):

- `gh repo view Companion-Inc/feynman --json name,url,description,stargazerCount,licenseInfo,updatedAt` → upstream "The open source AI research agent.", MIT, ~9,795 stars, active.
- `gh repo view cchris-p/feynman-modded --json name,isFork,parent,url,description,updatedAt` → `isFork: true`, parent `Companion-Inc/feynman`; local checkout version `0.5.8`.
- `gh repo view cchris-p/aa-studies --json name,isPrivate,defaultBranchRef,url` → private, default branch `main`; the current sole use case.
- `gh auth status` → authenticated as `cchris-p` with `repo` and `read:org` scopes.

## Questions to answer

- What distinct capabilities does `feynman` expose (workflows such as `/deepresearch`, `/lit`, `/audit`, `/replicate`, `/recipe`; the bundled researcher/reviewer/writer/verifier agents; the skill library; alphaXiv and literature-database tools)?
- Which of those have a plausible analogue or gap in `ort` today, and which are out of scope?
- For the `aa-studies` workflow specifically, what does `feynman` actually do well, what is missing, and what is the repeatable command sequence a user follows?
- Is there any candidate worth promoting from "nice external tool" to "planned feature" in this product, and if so, what is the smallest useful slice?
- What is the cost/risk of an adoption (fork divergence, upstream sync, TypeScript/Pi stack vs the Rust product, provider/model assumptions, telemetry)?

## Scope

- Enumerate and describe Feynman's workflows, agents, skills, and tools with references into the `cchris-p/feynman-modded` checkout.
- Run at least one real research task against `aa-studies` and document the invocation, the artifacts produced, and the result quality.
- Map Feynman capabilities against current `ort` capabilities and identify genuine gaps.
- Produce a recommendation: adopt, partially adopt, watch, or no action, with reasoning.
- If adoption is recommended, split the smallest concrete slice into a separate implementation-ready board item; do not implement it here.

## Non-goals

- Implementing any Feynman feature in `ort`.
- Making `feynman` a dependency of this product or changing the ScopeMux/provider scope.
- Syncing, patching, or re-forking `feynman-modded` as part of this item.
- Committing research outputs, prompts, or secrets from `aa-studies` into this repository.

## Done when

- Feynman's capabilities are written down with file references from the fork checkout.
- At least one `aa-studies` research run is documented end to end, with the exact command and observed outputs.
- A capability map against `ort` and a clear adopt/partial/watch/no-action recommendation exist.
- Any adoption candidate is either split into its own board item or explicitly deferred.
- Every external-project claim carries a `gh` command and observed result.

## Recommended verification

- Re-run the `gh repo view` commands above and confirm the fork is still `isFork: true` with parent `Companion-Inc/feynman` and the use-case repo is still reachable.
- Confirm the local `cchris-p/feynman-modded` checkout matches the recorded version and that `ort` capabilities were read from this repo, not assumed.
- Smoke-test one Feynman workflow against `aa-studies` and confirm outputs are usable before drawing any adoption conclusion.

## Related Items

- `FEAT-031` Investigate the builtin "general" agent and decide whether to remove it
- `SKILLS-001` Align skills with current and reference OpenCode behavior
- `SKILLS-002` Plan URL-backed skills parity
- `SKILLS-005` Compare skill referencing and per-step reinjection parity with vanilla
- `START-004` Assess current Rust state

## Notes

- Treat the fork as a periodically synced external reference, not as a stable dependency; re-verify facts on each revisit.
- The `aa-studies` use case is the gating criterion: if Feynman is not clearly better than the current workflow there, adoption should not be planned.

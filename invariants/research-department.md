# Research Department Invariants

- A **department** is a persistent, named separation-of-concern area with its own authority boundary and provenance rules.
- Departments are defined by concern, not by model, provider, or tool. A single model or agent may operate in more than one department; model choice never defines a department boundary.
- The **Research Department** is the department that consults **external/remote evidence** (papers, literature databases, web sources, and similar) to supplement research, vetting, and evaluation work.
- The Research Department applies across **every domain**, not only finance or aa-studies.
- External evidence is **advisory only**: it is confidence-labeled and provenance-bearing, and it is **never canonical**.
- Canonical authority stays with the owning area's local artifacts; Research Department output **informs but never overrides** area authority such as a gate's own rules.
- Separation of concern is what makes the Research Department safe to consult: advisory external evidence must never silently become local canonical truth.
- The department boundary and its authority rules hold regardless of how the department is operated (an external research harness or product-native tooling).
- This invariant does not make any specific research harness, model, or provider a dependency; which tooling serves the department is a separate, evidence-gated decision.
- The product may provide the **utilities and standards** that make instantiating the department easy in any workspace (for example skills/source plumbing, retrieval-provider integration, and an instantiation standard). Providing that surface does not make a research harness a runtime dependency or grant it canonical authority.
- The **internal research step** (`project-principles-redesign`) — deriving a causal hypothesis from local canonical sources and pre-registering a quantitative evaluation before trialing a rule — and this external-evidence department (`research-department`) are **two sequenced sub-steps of one research loop**: derive from local canonical sources first, then consult external evidence, then evaluate locally. They remain distinct concerns — neither is satisfied by the other (deriving locally does not consult external evidence, and consulting external evidence does not derive) — and the derivation's soundness remains an evidence-gated decision in its own right (`RESEARCH-005`).
- The external consultation is **consultative**: it records a disposition (`support` / `refute` / `ambiguous` / `nothing` / `error`), may **sharpen or refute** the pre-registered premise, and **never green-lights** it; a `nothing` or `ambiguous` disposition means "no external refutation", not confirmation. It never becomes canonical.
- The separation-of-concern, advisory, and never-canonical rules above apply to **both** sub-steps equally.

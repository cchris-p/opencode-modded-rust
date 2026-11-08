# Where We Left Off - 2026-11-08

## Done

`H-020` (persistent workspace map prerequisite program) and `H-018` (remaining ScopeMux
program) are **complete**. All steps are merged into `development` with QA evidence:

- H-020: `SCOPE-018` contracts -> `SCOPE-016` safe engine -> `SCOPE-017`
  supervisor/watcher -> `SCOPE-019` provider/tool integration.
- H-018: `SCOPE-003`, `SCOPE-012`, `SCOPE-013`, `SCOPE-014`, `SCOPE-004`, `SCOPE-010`,
  `SCOPE-006`, `SCOPE-009`.

PRs: #163, #164, #166, #167, #168, #169 (all targeted `development`; branches deleted).
The four prerequisite cards, plus `SCOPE-006` and `SCOPE-009`, are in `done` with QA
sections on the cards.

Final merged verification: native scopemux 38 passed; non-native types 9, session 177
(+2, +11), scopemux 3; grep 5; fmt clean; clippy exit 0 (pre-existing warnings only).
Provider QA against the LAN Ollama (`qwen3:30b`): `provider=scopemux
representation=Search candidates=5` (PASS).

## Constraints Still In Force

- Future-proofing remains on hold: do not touch `FUTURE-PROOFING-*` or `DECISION-*`.
- Do not start `SCOPE-007`, `SCOPE-008`, or `SCOPE-011`.
- The map is evidence-only; the runtime keeps task stage/verification/review/completion
  authority. The generic provider remains the default/fallback. Qwen-only activation is
  unchanged.

## Next Work

No remaining steps in this program. Independent work: `H-019` main-release fleet
deployment program and `INFRA-008` portable ScopeMux query bundle.

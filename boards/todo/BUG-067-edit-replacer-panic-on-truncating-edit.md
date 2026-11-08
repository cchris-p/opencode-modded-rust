---
id: "BUG-067"
title: "Edit replacer panics on a truncating model edit (range end index out of range)"
priority: "P1"
type: "bug"
area: "BUG"
spec: "invariants/coding-session-behavior.md"
status: "todo"
created: "2026-11-08"
---

# Edit replacer panics on a truncating model edit (range end index out of range)

## Summary

During a `qwen30-eval` run, `qwen3:30b` issued an edit that truncated
`docs/guide.md` to a single line. The edit tool then panicked instead of
returning an error:

```
thread 'main' panicked at crates/opencode-tool/src/edit/replacers.rs:309:34:
range end index 2 out of range for slice of length 1
```

The run aborted with exit code 101 and left the destructive partial edit in the
workspace.

## Evidence

- Sandbox repo: `~/apps/qwen30-eval`, run
  `20261108T190714Z-T04-medium-3` (T04 docs edit, medium context).
- Transcript:
  `~/apps/qwen30-eval/results/evidence/20261108T190714Z-T04-medium-3/transcript.log`.
- Result:
  `~/apps/qwen30-eval/results/20261108T190714Z-T04-medium-3.yaml`.
- The workspace diff reduced the whole guide to:
  `Raises ValueError if b is zero.` (no trailing newline).
- Model: `qwen3:30b` `Q4_K_M`, digest
  `ad815644918f0eaab341c12b67837cc6dd4562342cdaf118f83d5d554cb37226`.
- Harness: `opencode-modded-rust` commit
  `5536f89bf71ba330fa5e56f38c4e7a53d77e8950`.

## Impact

- A model-produced edit can abort the whole run with a panic instead of a
  recoverable tool error.
- The destructive edit is left in place, so a parent agent that trusts the
  tool's failure mode can lose file content.
- Blocks reliable delegation of write tasks to local models, independent of the
  model's own edit quality.

## Expected

- Validate the replacement range before slicing.
- Return a structured tool error when the range does not match the current
  content.
- Never panic on malformed or truncating model output.
- Prefer failing without applying a destructive partial edit.

## Reproduction

Observed once in three T04 medium runs; not yet reduced to a deterministic unit
test. Repro path:

1. Prepare a `qwen30-eval` T04 medium run:
   `scripts/prepare-task.sh T04 medium <run-id>`.
2. Run the copy-tests/report prompt against `ollama/qwen3:30b`.
3. When the model emits a whole-file replacement that collapses the guide,
   observe the panic at `replacers.rs:309`.

The failing transcript is the authoritative evidence until a fixture is
captured.

## Notes

- This is a harness defect, not a qwen capability result. The model's
  destructive edit is recorded separately in the qwen30-eval matrix.
- Related evaluation work: `qwen30-eval` `Q30-005`.
- Do not copy the sandbox workspace or private user data into this repository;
  reference the paths above.

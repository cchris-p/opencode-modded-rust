---
id: "FEAT-069"
title: "Profile ORT CPU hotspots with Flameox perf capture"
priority: "P3"
type: "experiment"
area: "FEAT"
spec: ""
status: "todo"
epic: "STD-153"
created: "2026-11-07"
updated: "2026-11-07"
---

# Profile ORT CPU hotspots with Flameox perf capture

## Summary

Use Flameox `perf` capture plus `rank_cpu_hotspots` to rank ORT CPU time for
startup and agent-turn workloads, so optimization work targets real hotspots
instead of guesses. Part of the cross-project pilot `STD-153` (standards).

## Approach

Run on a **Linux** host with `perf` and profiling permission:

```bash
uvx flameox capture --provider perf --cwd "$PWD" --preserve -- \
  ./target/release/opencode <representative-args>
uvx flameox analyze rank_cpu_hotspots --evidence <evidence_id>
```

MCP equivalent: `capture_and_rank_cpu_hotspots` with `provider.kind = "perf"`.

- Capture a representative startup path and one agent-turn path separately.
- For Rust, keep default `call_graph = "dwarf"` and build with debug info so
  symbols resolve; `fp` only if frame pointers are enabled.
- Compare two builds by running one capture per build and diffing the ranking
  tables. Flameox only supports multi-source ranking for V8 CPU profiles, so
  there is no single-call native A/B.

## Acceptance Criteria
- [ ] One representative startup workload and one agent-turn workload profiled.
- [ ] Top frames identified and recorded; plausible hotspot(s) named.
- [ ] Evidence preserved and `evidence_id` recorded on this card.
- [ ] Note whether the evidence changed a real optimization decision.

## Dev Notes

- `perf` is Linux-only. On macOS only `xctrace` trace metadata is available, not
  ranked native hotspots, so do not try to run this on `lapis`.
- Needs Linux `perf` and `perf_event_paranoid` permission.
- Flameox is a 0.x pilot: pin the version used and record it here.

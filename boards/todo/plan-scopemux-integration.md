# Plan ScopeMux integration

## Summary

Define how `ScopeMux` should integrate with the Rust runtime as an early but non-blocking architecture target.

## Why this exists

`ScopeMux` is strategically important for structural retrieval and context quality, but it is still early and should not destabilize V1 scope.

## Questions to answer

- What abstraction boundary should the runtime preserve so `ScopeMux` can be integrated cleanly later?
- Which retrieval responsibilities stay generic in V1?
- Which capabilities become `ScopeMux` responsibilities once it is mature enough?
- What confidence model is needed for heuristic structural relationships?

## Done when

- The integration boundary is documented.
- The minimum deferred contract for future `ScopeMux` support is defined.
- Follow-up tasks exist for any required runtime abstractions.

## Notes

- This is a planning item, not a requirement to ship `ScopeMux` in V1.
- The goal is to avoid painting the runtime into a corner before `ScopeMux` is ready.

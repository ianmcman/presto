---
phase: 04-data-layer-and-cache
plan: 01
subsystem: testing
tags: [mock-engine, ipc, faults]
requires: []
provides:
  - rate_limited and signed_out mock faults
  - mock routes for recent/played, recommendations, search hints, topResults, library search
  - --storefront and --library-songs mock flags
affects: [04-data-layer-and-cache]
key-files:
  modified:
    - crates/presto-ipc/src/fault.rs
    - crates/presto-ipc/tests/snapshots/schema__snapshot.snap
    - docs/PROTOCOL.md
    - crates/presto-engine-mock/src/fault.rs
    - crates/presto-engine-mock/src/main.rs
    - crates/presto-engine-mock/src/catalog.rs
    - crates/presto-engine-mock/tests/faults.rs
decisions:
  - Search hints match on word prefix, not substring, so "ne" does not hit "Mock Artist One"
metrics:
  duration: 15m
  completed: 2026-10-08
---

# Phase 4 Plan 01: Mock faults and routes Summary

Mock engine now supports `rate_limited[=ms]` and live `signed_out` faults, Apple-shaped Home and search routes, a storefront check, and a generated 10k-song library. PROTO stays 1.1.

## Deviations from Plan

- [Rule 1 - Bug] Hints use word-prefix matching: substring "ne" also matched "mock artist one", contradicting the specified `["neon static"]` result.
- Test fix: `rate_limited_flag_and_clear` uses `SetQueue{play:false}` for the cmd check because bare `Play` errors without a queue.

## Commits

- 12ae1cb: faults (Task 1)
- e640f14: mock routes and flags (Task 2)

`cargo test --workspace` passes. No stubs.

## Self-Check: PASSED

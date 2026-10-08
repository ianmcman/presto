---
phase: 04-data-layer-and-cache
plan: 04
subsystem: data
tags: [api-client, storefront, gating]
requires: ["04-01", "04-03"]
provides: ["ApiClient", "catalog_req", "MAX_IN_FLIGHT"]
key-files:
  modified:
    - crates/presto-core/src/data/client.rs
    - crates/presto-core/tests/data_client.rs
requirements-completed: [DATA-04, DATA-05]
duration: 10min
completed: 2026-10-08
---

# Phase 4 Plan 04: ApiClient Summary

`ApiClient` is the single path to Apple: catalog requests get the account storefront prefix, calls fail fast on engine/auth state, at most 4 are in flight, and IPC errors map to `UiErrorKind`.

## Commits

- 10bffbb: ApiClient with unit tests (4 tests)
- 43b3121: mock integration tests (5 tests: storefront applied, wrong storefront not_found, signed-out and offline fail fast under 50 ms, rate_limited keeps retry_after_ms)

## Deviations

None.

## Self-Check: PASSED

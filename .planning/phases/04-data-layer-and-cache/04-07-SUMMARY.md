---
phase: 04-data-layer-and-cache
plan: 07
status: done
requirements-completed: [DATA-01, DATA-03, DATA-05, DATA-06]
key-files:
  modified:
    - crates/presto-core/src/data/mod.rs
    - crates/presto-core/tests/data_library.rs
    - crates/presto-core/tests/data_errors.rs
---

# Phase 4 Plan 07: DataHandle

`DataHandle` serves per-view `watch` state with cache-then-revalidate, one-page `load_more`, Home shelves, and rate-limit auto-retry (3 tries via the 5/10/20 s ladder or `retry_after_ms`, then manual Retry). 6 library tests, 5 error tests; full `cargo test -p presto-core` is green.

## Commits

- dce228b: DataHandle (open, revalidate, load_more, shelves, fail/retry; the error logic landed in this commit with the rest of mod.rs)
- 105a8b2: data_errors tests

## Decisions

- Slots are generic over `Slotted` (Item, Shelf); `gen` is a reserved word in edition 2024, so the field is `gn`.
- `load_more` is a no-op after a failed page until Retry or the countdown fires (avoids a second concurrent fetch).
- Re-opening a view with an error and no countdown triggers a fresh fetch.

## Deviations

- Tasks 1 and 2 share one source file; the error/retry code was written with Task 1 and committed there. Task 2 commit holds the tests only.
- Error tests that expect 100 items pass `--library-songs 300` (mock default library has 6 songs).

## Self-Check: PASSED

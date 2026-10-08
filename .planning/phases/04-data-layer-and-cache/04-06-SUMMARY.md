---
phase: 04-data-layer-and-cache
plan: 06
status: done
requirements-completed: [DATA-01, DATA-03, DATA-05]
key-files:
  modified:
    - crates/presto-core/src/data/models.rs
    - crates/presto-core/src/data/view.rs
    - crates/presto-engine-mock/src/catalog.rs
---

# Phase 4 Plan 06: Models and view policy

Tolerant hand-parsed models (Item, Shelf, SearchResults, Page, Rows) and view policy (ViewKey, SORT_SUPPORT from the live probe, ListState, is_stale, next_retry). 9 model tests, 7 view tests.

## Decisions

- Library page size 100, others 10, per `ViewKey::page_size`.
- `Shelf.see_all` is always None (04-02: contents are inline, no paged route). `ViewKey::Shelf` stays unused.
- `parse_search` reads `topResults` (live) or `top`, and plain or `library-` group keys.

## Deviations

- [Rule 1] Mock search emitted `results.top`; live uses `results.topResults`. Changed the mock route and its test in `catalog.rs`.

## Self-Check: PASSED

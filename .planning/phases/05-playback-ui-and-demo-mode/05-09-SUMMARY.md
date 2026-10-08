---
phase: 05-playback-ui-and-demo-mode
plan: 09
subsystem: ui
tags: [egui, home, library, search, settings]
requires: [05-07]
provides: [home view, library tabs, search view, settings view, list_states helper]
affects: [05-08, 05-10]
key-files:
  modified:
    - crates/presto/src/ui/home.rs
    - crates/presto/src/ui/library.rs
    - crates/presto/src/ui/search.rs
    - crates/presto/src/ui/settings.rs
  created:
    - crates/presto/tests/ui_browse.rs
decisions:
  - "Views clone the watch receiver before borrowing so app.act stays callable"
  - "Shared helpers in library.rs: list_states, item_card, open_item, song_rows, error_copy"
metrics:
  completed: 2026-10-08
requirements: [PLAY-03, IPC-04]
---

# Phase 5 Plan 09: Browse views Summary

Home, Library (lazy paging at 100), Search and Settings (confirm-gated Clear cache) views with shared loading/error/offline/empty rendering. 7 headless tests in `ui_browse` pass.

## Deviations from Plan

- Tests for Home and Albums render on a 1200x3000 screen: shelves and cards below the fold are culled from the 800 px frame.
- `demo_app` takes fault names only, so the rate-limit test passes `"rate_limited=100"`.
- Search song results use `song_rows` in a 5-row capped area.

## Deferred

`ui_detail` tests (artist_page, artist_all, long_title) fail; they belong to the in-progress detail plan.

## Self-Check: PASSED

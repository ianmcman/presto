---
phase: 05-playback-ui-and-demo-mode
plan: 10
subsystem: ui
tags: [egui, album, playlist, artist]
requires: [05-07]
provides: [album/playlist page, artist page, See all grid]
affects: [crates/presto/src/ui/detail.rs, crates/presto/src/ui/artist.rs]
key-files:
  created: [crates/presto/tests/ui_detail.rs]
  modified: [crates/presto/src/ui/detail.rs, crates/presto/src/ui/artist.rs]
decisions:
  - "Track list is windowed by viewport inside one page-level ScrollArea instead of show_rows, so the hero scrolls with the rows"
  - "Detail.rs exports ids, year, play_buttons, track_rows and blocked for the artist page"
metrics:
  completed: 2026-10-08
requirements-completed: [PLAY-03, PLAY-04]
---

# Phase 5 Plan 10: Album, playlist and artist pages Summary

Album/playlist pages (hero, Play/Shuffle, windowed numbered rows, lazy paging at 100, unavailable dimming) and the artist page (Top Songs with Show more, Albums and Singles & EPs shelves, See all grid, library-to-catalog Replace).

## Commits
- Task 1: album and playlist page (detail.rs)
- Task 2: 164a914 artist page and tests

## Deviations
None to the plan. Two test-only choices: the artist tests use a 1200x1600 screen (shelves sit below the 800 px fold and egui culls off-screen text) and long_title uses a 700 px wide screen so the row title truncates.

## Known Stubs
None. Play on a playlist over 100 tracks queues only the loaded pages (marked ponytail in the plan).

## Verification
`cargo test -p presto --test ui_detail`: 7 passed. `cargo test -p presto` all green.

## Self-Check: PASSED

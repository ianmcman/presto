---
phase: 05-playback-ui-and-demo-mode
plan: 02
subsystem: mock-engine
tags: [mock, demo, catalog, player]
requires: []
provides:
  - mock catalog with album/playlist/artist detail, paged /tracks routes, artist views
  - unavailable track s7 and runtime-failing track s8 in the mock player
affects: [05-playback-ui-and-demo-mode]
key-files:
  modified:
    - crates/presto-engine-mock/src/catalog.rs
    - crates/presto-engine-mock/src/player.rs
    - crates/presto-engine-mock/tests/protocol.rs
    - crates/presto-core/tests/data_library.rs
    - docs/PROTOCOL.md
decisions:
  - "Playback failure is derived from the track (playable flag, id s8), no new fault kind or protocol change"
metrics:
  tasks: 2
  completed: 2026-10-08
requirements-completed: [IPC-04, PLAY-04]
---

# Phase 5 Plan 02: Mock catalog and playback failure Summary

Mock catalog now covers every view (14 songs, 6 albums, 4 playlists incl. a 150-track one, 3 artists), with `/tracks` paging, artist `views`, `include=catalog`, and generated library ids queueable. The player emits `Error` then `Stopped` for the unavailable track s7 (and `i.00007`) and for s8 (upstream 503).

## Commits
- 34b3f46: extended catalog, routes, test updates, PROTOCOL.md line
- b3d7d09: player failure path and tests

## Deviations from Plan
- Existing `library_pagination` test offset changed from 4 to 12 (song count grew from 6 to 14), same intent.

## Self-Check: PASSED
`cargo test -p presto-engine-mock` and `cargo test -p presto-core` green.

---
phase: 05-playback-ui-and-demo-mode
plan: 03
subsystem: presto-core data layer
tags: [detail, tracks, playable, engine-error]
requires: [05-02]
provides: [DataHandle::detail, ViewKey::Tracks, ViewKey::Detail, tracks_key, detail_key, Item.playable, CoreState.engine_errors]
affects: [crates/presto-core]
key-files:
  modified:
    - crates/presto-core/src/data/models.rs
    - crates/presto-core/src/data/view.rs
    - crates/presto-core/src/data/mod.rs
    - crates/presto-core/src/state.rs
    - crates/presto-core/src/supervisor.rs
    - docs/DATA-CACHE.md
  created:
    - crates/presto-core/tests/data_detail.rs
    - crates/presto-core/tests/engine_error.rs
decisions:
  - "{sf} placeholder in view paths is substituted in data::req; unknown storefront fails as Internal"
  - "Detail is a third Slotted type; load_more/refresh/retry dispatch by key variant"
metrics:
  tasks: 2
  completed: 2026-10-08
---

# Phase 5 Plan 03: Detail data, track lists, playable flag, engine errors

Album, playlist and artist detail load through the existing cached, paged, retrying pipeline. Songs without playParams are marked unplayable. Engine `Error` events now land in `CoreState`.

## Commits

- 9642c9d: models, view keys, engine errors in CoreState
- 5837503: DataHandle::detail, `{sf}` substitution, data_detail tests, DATA-CACHE note

## Deviations from Plan

None. `cargo test -p presto-core` is green (82 lib tests plus all integration tests).

## Known Stubs

None. Track page size 100 and the playParams-absent rule are unverified live (05-11).

## Self-Check: PASSED

---
phase: 05-playback-ui-and-demo-mode
plan: 06
subsystem: ui
tags: [backend, demo, queue, tokio]
requires: [05-01, 05-03, 05-04, 05-05]
provides: [Backend, Page, Action, Event, demo_config, real_config]
affects: [05-07, 05-08, 05-09, 05-10]
key-files:
  created:
    - crates/presto/src/model.rs
    - crates/presto/src/launch.rs
    - crates/presto/src/backend.rs
    - crates/presto/tests/common/mod.rs
    - crates/presto/tests/demo_boot.rs
    - crates/presto/tests/backend_mock.rs
  modified:
    - crates/presto/src/lib.rs
key-decisions:
  - "edit_queue captures position before SetQueue, then seeks after Loading ends (<= 5 s wait)"
  - "demo socket lives at runtime/presto/engine-demo.sock; demo_config creates the dir 0700"
requirements-completed: [IPC-04, PLAY-01, PLAY-02]
duration: 15min
completed: 2026-10-08
---

# Phase 5 Plan 06: Backend and demo launch Summary

Backend owns a tokio runtime over presto-core, with demo/real launch configs and the Page/Action/Event contract types.

## Commits
- a4d6f3f model.rs and launch.rs
- see git log for the backend.rs commit

## Verification
- `cargo test -p presto --lib -- launch:: model::`: 6 passed
- `cargo test -p presto --test demo_boot --test backend_mock`: 11 passed (9 + 2)
- `cargo test -p presto`: all green; acceptance greps all match

## Deviations from Plan
None. Test count split (commands into two tests, mock_path and demo tests into separate cases) to meet the minimums.

## Known Stubs
None.

## Self-Check: PASSED

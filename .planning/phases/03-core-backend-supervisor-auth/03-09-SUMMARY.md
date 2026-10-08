---
phase: 03-core-backend-supervisor-auth
plan: 09
subsystem: core
tags: [supervisor, restore, mock-engine]
requires: []
provides:
  - restore that waits out loading and holds the target state before Ready
affects: [03-10]
key-files:
  modified:
    - crates/presto-engine-mock/src/player.rs
    - crates/presto-engine-mock/src/main.rs
    - crates/presto-core/src/supervisor.rs
    - crates/presto-core/tests/recover.rs
    - .planning/REQUIREMENTS.md
key-decisions:
  - "verify_step never sends Play/Pause/Seek while loading; seek and state are fixed separately; Done needs a 2 s hold"
requirements-completed: []
completed: 2026-10-08
---

# Phase 3 Plan 09: Paused restore vs MusicKit load Summary

Mock now models MusicKit load (Loading state, dropped Play/Pause, autoplay 800 ms after load and 400 ms after seek), and `verify_step` waits out loading, fixes seek and state separately, and requires a 2 s hold before Done.

## Commits
- 85d8eda test(03-09): mock models autoplay after load
- 7f64981 fix(03-09): restore waits out loading and holds paused before Ready

## RED (Task 1)
`quirky_crash_restores_paused_when_paused` (recover.rs:188) and `quirky_second_crash_restores_paused` (recover.rs:202) failed against the old supervisor; `quirky_crash_restores_position` passed.

## GREEN (Task 2)
`cargo test -p presto-core` (recover: 13 passed), `cargo test -p presto-engine-mock` (16 unit, all pass), clippy `-D warnings` clean.

## Deviations
None. `T` in recover.rs raised to 15 s as planned. CORE-01 reverted to Pending; plan 03-10 marks it on a live pass.

## Known Stubs
None.

## Self-Check: PASSED

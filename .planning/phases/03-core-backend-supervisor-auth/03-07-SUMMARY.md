---
phase: 03-core-backend-supervisor-auth
plan: 07
subsystem: core
tags: [supervisor, restore, mock-engine]
requires: [03-06]
provides: [verified crash restore against a quirky engine]
affects: [03-08]
key-files:
  modified:
    - crates/presto-engine-mock/src/player.rs
    - crates/presto-engine-mock/src/main.rs
    - crates/presto-core/src/supervisor.rs
    - crates/presto-core/tests/recover.rs
decisions:
  - Restore always ends with an explicit Play or Pause after a load (D-01, D-04)
  - Seek and play state are verified after restore, retried up to 3 times with a 1.5 s settle, then Ready with a logged warning
metrics:
  completed: 2026-10-08
requirements-completed: [CORE-01]
---

# Phase 3 Plan 07: Restore seek/state verification Summary

Mock `--restore-quirks` reproduces the two live failures (dropped first seek, autoplay); the supervisor now ends restore with Play/Pause and a verify/retry loop.

## Commits

- c8a30cf: mock engine `--restore-quirks`
- 2c7529f: `Verify`/`verify_step` in supervisor, 3 quirky tests in `recover.rs`

## Verification

Quirky position and paused tests failed before the fix (RED) and pass after. `cargo test -p presto-core`, `cargo test -p presto-engine-mock` and clippy `-D warnings` are clean. Live confirmation is plan 03-08.

## Deviations from Plan

None. `quirky_second_crash_restores_paused` passed before the fix, since the mock's autoplay happened not to hide D-04 there; kept as a regression test.

## Self-Check: PASSED

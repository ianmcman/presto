---
phase: 03-core-backend-supervisor-auth
plan: 14
subsystem: core
tags: [supervisor, restore, seek]
requires: [03-13]
provides: [restore survives unanswered Seek, guarded unmute]
key-files:
  modified: [crates/presto-core/src/supervisor.rs]
decisions:
  - "Only a failed SetQueue ends a restore; other step errors fall through to verify"
  - "A restore that must end paused sends Pause first and stays muted while Playing or Loading"
metrics:
  duration: 5min
  completed: 2026-10-08
---

# Phase 3 Plan 14: Restore survives unanswered Seek Summary

Supervisor restore no longer ends on a Seek error or timeout, and `end_restore` sends Pause before unmuting and leaves the engine muted if a must-pause restore is still Playing or Loading.

## Changes

- `step_result`: only a failed SetQueue calls `end_restore`.
- `begin_restore`: no Seek at or below 2000 ms.
- `verify_step`: seek give-up sets target 0 and falls through to the state check.
- `end_restore`: Pause before SetVolume when the restore must end paused; logs `left muted` when the player is busy.

## Results

Commit 2e212fb. Both seek_hang tests pass; presto-core suite and clippy `-D warnings` clean.

## Deviations from Plan

None.

## Self-Check: PASSED

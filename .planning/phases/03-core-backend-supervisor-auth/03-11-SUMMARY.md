---
phase: 03-core-backend-supervisor-auth
plan: 11
subsystem: supervisor
tags: [restore, mute, mock-engine]
requires: [03-09, 03-10]
provides:
  - "Load restores run muted and put the snapshot volume back on every exit"
  - "Mock engine logs `mock: audible at N ms`"
affects: [crates/presto-core/src/supervisor.rs, crates/presto-engine-mock]
key-files:
  modified:
    - crates/presto-core/src/supervisor.rs
    - crates/presto-core/tests/recover.rs
    - crates/presto-engine-mock/src/main.rs
    - crates/presto-engine-mock/src/player.rs
key-decisions:
  - "Mute before SetQueue (and again after) instead of guarding against engine autoplay"
requirements-completed: [CORE-01]
duration: 15min
completed: 2026-10-08
---

# Phase 3 Plan 11: Silent load restore Summary

A load restore now sends `SetVolume 0` before and after `SetQueue`, and `end_restore` queues `SetVolume { snap.volume }` on confirm, 20 s deadline, step error, or "queue did not load".

## Tasks

1. RED (35d5092): `Player::audible()`, mock stderr `mock: audible at N ms`, three quirky recover tests assert on the restarted engine's log. They failed on audio only (position test: audible at 0 ms).
2. GREEN (see git log, `fix(03-11)`): `Restore.unmute`, `end_restore()`, mute steps, volume added to the step log line. `cargo test -p presto-core` and clippy `-D warnings` are clean.

## Deviations from Plan

None. The mock helper is a generic `audible_log` wrapper on `Engine` used by both `handle` and `tick`.

## Notes

Live confirmation against the real engine is not part of this plan.

## Self-Check: PASSED

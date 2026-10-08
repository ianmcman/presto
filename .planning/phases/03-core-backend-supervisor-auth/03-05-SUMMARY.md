---
phase: 03-core-backend-supervisor-auth
plan: 05
subsystem: core
tags: [mirror, auth, restore, supervisor]
requires: [03-04]
provides:
  - "QueueMirror, PlayerMirror, Snapshot, snapshot(), queue_key()"
  - "AuthMachine and the D-12 fail-fast gate"
  - "Restore sequence after crash, hang, manual restart and re-auth"
  - "CoreState.auth / queue / player"
affects: [03-06]
key-files:
  created:
    - crates/presto-core/src/mirror.rs
    - crates/presto-core/src/auth.rs
    - crates/presto-core/tests/auth.rs
    - crates/presto-core/tests/mirror.rs
  modified:
    - crates/presto-core/src/supervisor.rs
    - crates/presto-core/src/state.rs
    - crates/presto-core/src/lib.rs
    - crates/presto-core/tests/recover.rs
decisions:
  - "Ready is published only when the session is usable (bridge, auth event seen, no restore pending)"
  - "Restore steps run one at a time from a pump() after every actor event; step results are matched by request id"
  - "A restore step failing with auth_expired keeps the restore for the next sign-in; other errors drop it"
  - "A pending restore snapshot is kept over a fresh one on a crash (the mirror is half-restored then)"
  - "Manual restart_engine also snapshots and restores (not in the plan)"
metrics:
  tasks: 3
  completed: 2026-10-08
---

# Phase 3 Plan 05: Mirror, Auth and Restore Summary

Supervisor now keeps a read-only queue/player mirror by revision, gates traffic on auth, and restores queue, position, shuffle/repeat/volume and play state after crash, hang or re-auth.

## Commits

- 2fb9c4e feat(03-05): queue/player mirror and auth machine
- 8456533 feat(03-05): wire mirror, auth gate and restore into supervisor
- a72b179 test(03-05): restore, auth and mirror integration tests

## Verification

`cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings` and `npm test` in engine/ all green. presto-core suite run 3 times without flakes.

## Deviations from Plan

**1. [Rule 3 - Structure] Session locals bundled into `Sess`**
The restore runner must send from hooks, so conn/pending/queued/next_id moved into a `Sess` struct and `Phase` was replaced by `drift` plus `usable()`. No behavior change for the 03-04 tests.

**2. [Rule 2 - Missing] Manual restart restores too**
`End::Restart` snapshots the mirror so `restart_engine()` does not drop the queue.

**3. Rapid-change test uses start 1 for the second queue**
Computed against the mock: Next at the last item ends playback without a rev bump, and Prev after Ended restarts; rev ends at 5 and index at 1.

## Known Stubs

None. Seek is not verified after load (`ponytail` comment, RESEARCH Pitfall 6); the live check in 03-06 decides if a retry is needed.

## Self-Check: PASSED

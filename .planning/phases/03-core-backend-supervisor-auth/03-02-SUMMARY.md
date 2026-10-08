---
phase: 03-core-backend-supervisor-auth
plan: 02
subsystem: core
tags: [rust, nix, supervisor, backoff]
requires: []
provides: [presto-core paths, Backoff]
affects: [03-04]
key-files:
  created: [crates/presto-core/src/paths.rs, crates/presto-core/src/backoff.rs, crates/presto-core/tests/stale.rs]
  modified: [Cargo.toml, Cargo.lock]
requirements-completed: [CORE-01, AUTH-03]
completed: 2026-10-07
---

# Phase 3 Plan 02: presto-core paths and backoff Summary

`presto-core` crate with a 0700 profile dir helper, pidfile-based stale engine sweep (start-ticks plus exe guard, killpg), log rotation, and the D-02 backoff machine.

## Deviations from Plan

None in behavior. Both tasks landed in one commit (backoff.rs was needed for the crate to compile). Log pruning keeps the new file and the 9 newest others, so exactly 10 remain.

## Commits

- see `git log --grep 03-02`

## Self-Check: PASSED

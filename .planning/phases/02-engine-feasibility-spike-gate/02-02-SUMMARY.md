---
phase: 02-engine-feasibility-spike-gate
plan: 02
subsystem: spike-driver
tags: [rust, ipc, spike]
requires: [presto-ipc, presto-engine-mock]
provides: [presto-spike CLI: check, signin, measure]
affects: [02-04]
key-files:
  created:
    - crates/presto-spike/Cargo.toml
    - crates/presto-spike/src/main.rs
    - crates/presto-spike/src/session.rs
    - crates/presto-spike/src/rss.rs
    - crates/presto-spike/src/checks.rs
    - crates/presto-spike/tests/mock.rs
decisions:
  - "wait_for takes a `what` label for timeout messages (allowed by plan)"
  - "Playing/Paused states already seen during a command's response wait count (states are recorded, not just streamed)"
metrics:
  tasks: 2
  completed: 2026-10-07
---

# Phase 2 Plan 02: presto-spike driver Summary

`presto-spike` binds the presto-ipc socket, spawns an engine, handshakes, heartbeats, and runs the hello/musickit/session/api/playback/events checklist plus `signin` and `measure`. `check all` passes against presto-engine-mock.

## CLI

```
presto-spike [OPTIONS] <check|signin|measure>
      --engine-bin, --engine-dir [engine], --engine-arg (repeatable), --socket,
      --profile, --log-dir [.planning/phases/02-engine-feasibility-spike-gate/logs],
      --label [run], --allow-mock
check <NAME>   hello|musickit|session|api|playback|events|all
      --song, --search-term ["Bohemian Rhapsody"], --min-play-secs [60],
      --seek-secs [120], --auth-wait-secs [90]
signin  --wait-secs [600]
measure --song, --search-term, --play-secs [300], --interval-secs [5], --settle-secs [20]
```

## Deviations from Plan

None in behavior. `Session::wait_for` has an extra `what: &str` parameter. Playback/events waits first consult recorded state (`states`, `last_progress`) because events can arrive while a command response is awaited.

## Verification

`cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` green. 6 integration tests and 2 rss unit tests in presto-spike. Not run against a real engine (plan 02-04).

Commits: 6141d25, 4447175.

## Self-Check: PASSED

---
phase: 03-core-backend-supervisor-auth
plan: 04
subsystem: core
tags: [supervisor, tokio, heartbeat, backoff, drift]
requires: [03-01, 03-02]
provides:
  - "Core::start / CoreHandle (command, request, show_sign_in, mock, restart_engine, shutdown, state watch)"
  - "CoreConfig, Timings, Launch::electron, check_bridge, REQUIRED_BRIDGE_CAPS"
  - "CoreState, EngineStatus, BridgeInfo"
  - "Extension points Actor::on_event and Actor::on_bridge_ready for 03-05"
affects: [03-05, 03-06]
tech-stack:
  patterns: ["single actor task, no locks", "process group kill with Drop guard"]
key-files:
  created:
    - crates/presto-core/src/config.rs
    - crates/presto-core/src/state.rs
    - crates/presto-core/src/supervisor.rs
    - crates/presto-core/tests/common/mod.rs
    - crates/presto-core/tests/recover.rs
    - crates/presto-core/tests/drift.rs
  modified:
    - crates/presto-core/src/lib.rs
decisions:
  - "Messages sent during the handshake wait in the channel; Shutdown during connect waits up to Timings.connect (ponytail comment in code)"
  - "Queued pre-ready commands are failed with unavailable if the engine goes to Drift before Ready"
  - "Group is SIGKILLed after the grace wait only if it still exists (killpg probe), avoiding a stray kill on a reaped pgid"
metrics:
  tasks: 2
  completed: 2026-10-08
---

# Phase 3 Plan 04: Supervisor Summary

Single-task supervisor actor that spawns the engine in its own process group, gates Ready on a valid bridge_ready, restarts on crash or hang with D-02 backoff, and reports drift without restarting.

## Commits

- e5930d4 feat(03-04): supervisor actor with config and state
- a2a3cb1 test(03-04): supervisor recover and drift tests against mock

## Verification

`cargo test -p presto-core` green (15 unit, 3 drift, 5 recover, 3 stale); `cargo clippy -p presto-core --all-targets -- -D warnings` clean.

## Deviations from Plan

None. Both tasks were committed after one combined implementation pass; the test commit follows the source commit.

## Known Stubs

`Actor::on_event` and `Actor::on_bridge_ready` are intentional no-ops; 03-05 fills them.

## Self-Check: PASSED

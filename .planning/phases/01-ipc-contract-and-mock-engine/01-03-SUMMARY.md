---
phase: 01-ipc-contract-and-mock-engine
plan: 03
subsystem: ipc
tags: [rust, tokio, mock-engine, unix-socket]
requires: [01-01]
provides:
  - presto-engine-mock binary (handshake, cmd/req/ping dispatch, playback clock, canned catalog)
  - tests/common harness reusable by plan 01-04
affects: [01-04, 05]
key-files:
  created:
    - crates/presto-engine-mock/src/catalog.rs
    - crates/presto-engine-mock/src/player.rs
    - crates/presto-engine-mock/tests/common/mod.rs
    - crates/presto-engine-mock/tests/protocol.rs
  modified:
    - crates/presto-engine-mock/src/main.rs
decisions:
  - "Peer hangup during a send exits 0 (matched by message string, since LinesCodecError is not nameable without a tokio-util dep)"
metrics:
  duration: 15min
  completed: 2026-10-07
---

# Phase 1 Plan 3: Mock engine Summary

Fault-free `presto-engine-mock` binary: connects to presto's socket, handshakes, answers every cmd/req/ping, runs an Instant-based playback clock with queue rev/seq, and serves a canned Apple-shaped catalog.

## Tasks

1. Catalog and player with 10 unit tests, time injected, no sleeps (c83d30b)
2. main loop, harness, 5 protocol integration tests over a real socket (95e1860)

## Deviations from Plan

**[Rule 1 - Bug] Send to a closed peer exited 1.** The mock sent `auth signed_in` after presto had already dropped the connection, so the EOF test saw exit 1. Broken pipe and connection reset on send now exit 0.

## Known Stubs

`Frame::Mock` replies Internal "fault injection not implemented"; plan 01-04 replaces it.

## Self-Check: PASSED

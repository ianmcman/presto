---
phase: 03-core-backend-supervisor-auth
plan: 01
subsystem: ipc
tags: [ipc, protocol, mock-engine]
requires: []
provides:
  - presto-ipc proto 1.1 (BridgeReady, ShowWindow, SetQueue.play, caps::WINDOW)
  - mock engine flags --auth, --bridge-missing, --bridge-caps
affects: [presto-core, engine]
key-files:
  modified:
    - crates/presto-ipc/src/command.rs
    - crates/presto-ipc/src/event.rs
    - crates/presto-ipc/src/frame.rs
    - docs/PROTOCOL.md
    - crates/presto-engine-mock/src/main.rs
    - crates/presto-engine-mock/src/fault.rs
    - crates/presto-engine-mock/src/player.rs
decisions:
  - "signed_out start is modeled as Faults.signed_out; cleared by mock none, which emits auth signed_in"
metrics:
  completed: 2026-10-07
---

# Phase 3 Plan 01: Protocol 1.1 and mock engine Summary

Additive IPC 1.1 (`bridge_ready`, `show_window`, `set_queue.play` defaulting true, `window` capability) with PROTOCOL.md and schema snapshot updated, and a mock engine that implements all of it plus `--auth`, `--bridge-missing`, `--bridge-caps`.

## Deviations from Plan

- [Rule 3] `version_check` test in roundtrip.rs asserted "1.0" in the mismatch message; updated to "1.1".
- `cargo fmt` reformatted two presto-ipc test files (schema.rs, common/mod.rs); included in the mock commit. Formatting only.

## Commits

- fcc50d9 feat(03-01): protocol 1.1 bridge_ready, show_window, set_queue play
- c3d2a04 feat(03-01): mock engine speaks proto 1.1

## Known Stubs

None.

## Self-Check: PASSED

`cargo test -p presto-ipc -p presto-engine-mock -p presto-spike` green.

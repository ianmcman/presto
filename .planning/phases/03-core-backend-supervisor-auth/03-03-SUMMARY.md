---
phase: 03-core-backend-supervisor-auth
plan: 03
subsystem: engine
tags: [electron, ipc, bridge]
requires: []
provides:
  - engine proto 1.1 (hello, bridge_ready, show_window)
  - bridge.js override path
affects: [03-06]
key-files:
  created: [engine/bridge-path.js, engine/window-policy.js, engine/test/bridge-path.test.js, engine/test/window-policy.test.js]
  modified: [engine/main.js, engine/bridge.js, engine/test/bridge.test.js]
decisions:
  - Rust owns window showing via show_window; engine only hides on signed_in
metrics:
  duration: 5min
  completed: 2026-10-07
---

# Phase 3 Plan 03: Engine proto 1.1 Summary

Engine speaks proto 1.1: user bridge override, bridge_ready handshake, Rust-driven window, close-to-hide, 3 s hard exit after socket loss, paused set_queue.

## Commits
- 40e9a34: pure modules and bridge.js changes
- d93cb5e: main.js wiring

## Deviations from Plan
None. `npm test` passes (24 tests); `node --check main.js` passes. Live behavior is checked in plan 03-06.

## Self-Check: PASSED

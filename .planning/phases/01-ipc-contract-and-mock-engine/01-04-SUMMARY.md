---
phase: 01-ipc-contract-and-mock-engine
plan: 04
subsystem: ipc
tags: [mock-engine, fault-injection, tokio]
requires: [01-03]
provides:
  - "hang, crash, auth_expired, slow faults via --fault flags and live mock frames"
affects: [phase-3-supervisor]
key-files:
  created:
    - crates/presto-engine-mock/src/fault.rs
    - crates/presto-engine-mock/tests/faults.rs
  modified:
    - crates/presto-engine-mock/src/main.rs
decisions:
  - "Outbound gating (hang, slow) lives in the main loop; mock acks are forced past both"
  - "Startup hang also drops the initial auth event so the stream is fully silent"
metrics:
  completed: 2026-10-07
---

# Phase 1 Plan 04: Mock fault injection Summary

The mock now supports hang, crash (exit 101, optional delay), auth_expired and slow, each triggerable by a repeatable `--fault` flag or a live `mock` frame, with 9 integration tests.

## Tasks

1. Fault state and loop wiring: 5a2b3c3
2. Integration tests: 199f595

## Deviations from Plan

None. Startup `hang` suppresses the initial `auth signed_in` event (required for the "no frame at all" assertion).

## Self-Check: PASSED

Workspace tests and clippy (-D warnings) pass.

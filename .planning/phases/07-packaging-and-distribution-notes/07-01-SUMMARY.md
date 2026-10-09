---
phase: 07-packaging-and-distribution-notes
plan: 01
subsystem: ipc, engine
tags: [cdm, widevine, protocol]
requires: []
provides: ["Event::Cdm (proto 1.2)", "engine CDM state reporting"]
key-files:
  created: [engine/cdm.js, engine/test/cdm.test.js]
  modified: [crates/presto-ipc/src/event.rs, crates/presto-ipc/src/frame.rs, crates/presto-ipc/src/lib.rs, crates/presto-ipc/tests/roundtrip.rs, crates/presto-ipc/tests/snapshots/schema__snapshot.snap, crates/presto-spike/src/session.rs, docs/PROTOCOL.md, engine/main.js]
decisions:
  - "cdm event is additive at proto 1.2; failure leaves engine alive with no window"
metrics:
  completed: 2026-10-08
---

# Phase 7 Plan 01: CDM event Summary

Additive `cdm` event (checking/ready/failed) at proto 1.2; the engine wraps `components.whenReady()` in a 120 s timeout and stays alive after failure.

## Commits
- aace7a4: wire type, docs, snapshot
- 18263b1: engine reporting

## Deviations from Plan

**[Rule 3 - Blocking]** `roundtrip.rs` `hello_shape` and `version_check` hard-coded 1.1; updated to 1.2.

## Known Stubs
None.

## Self-Check: PASSED

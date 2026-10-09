---
phase: 07-packaging-and-distribution-notes
plan: 02
subsystem: core-supervisor, ui
tags: [cdm, widevine, drift, supervisor]
requires: [07-01]
provides: [CoreState.cdm, CdmInfo, drift deferral during CDM download, mock --cdm-delay-ms/--cdm-fail]
affects: [crates/presto-core, crates/presto-engine-mock, crates/presto]
key-files:
  created: [crates/presto-core/tests/cdm.rs]
  modified: [crates/presto-core/src/state.rs, crates/presto-core/src/lib.rs, crates/presto-core/src/supervisor.rs, crates/presto-engine-mock/src/main.rs, crates/presto/src/ui/status.rs]
key-decisions:
  - "Cdm checking/failed disarms the drift timer; cdm ready re-arms it with a fresh window unless bridge already seen or drift already fired"
  - "CDM failure shows a panel with no button; user restarts Presto"
requirements-completed: [PKG-01]
duration: 15min
completed: 2026-10-09
---

# Phase 7 Plan 02: CDM state in core and UI Summary

Supervisor suspends the 15 s drift timer while the engine reports `cdm checking` or `failed`, re-arms it on `ready`, and the Starting view shows "Preparing playback components…" or a named failure panel.

## Commits
- 61d4e8a: CoreState.cdm, supervisor Cdm arm, mock flags, 3 tests in tests/cdm.rs
- 67f6c36: Starting view text and failure panel

## Deviations from Plan
None. The workspace test run prints a `bogus` fault error from an existing negative mock test; it is expected.

## Known Stubs
None.

## Self-Check: PASSED

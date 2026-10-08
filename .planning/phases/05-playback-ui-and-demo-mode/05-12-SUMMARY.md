---
phase: 05-playback-ui-and-demo-mode
plan: 12
status: complete
gap_closure: true
requirements: [PLAY-01]
---

# 05-12 summary: cold-start fetch retry

On becoming usable, only the focused view was refreshed, so Home's second view stayed failed (Offline) until Try again. `retry_offline` in `crates/presto-core/src/data/mod.rs` now retries every other Offline-failed view on that transition. Real errors are unchanged.

Test: `tests/data_startup.rs` (failed before, passes after). Workspace suite green. Three cold starts on the real engine showed Home with no error (commit efbc7a1).

## Self-Check: PASSED

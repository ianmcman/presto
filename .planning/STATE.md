---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
status: unknown
stopped_at: Completed 07-01-PLAN.md
last_updated: "2026-10-09T02:18:58.578Z"
progress:
  total_phases: 7
  completed_phases: 6
  total_plans: 56
  completed_plans: 53
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-10-07)

**Core value:** Full-catalog Apple Music playback and library browsing from a fast native Linux UI, without an Apple Developer account.
**Current focus:** Phase 07 — packaging-and-distribution-notes

## Current Position

Phase: 07 (packaging-and-distribution-notes) — EXECUTING
Plan: 3 of 5

## Performance Metrics

None yet.

## Accumulated Context

### Decisions

- Phase 2 is a go/no-go gate: stop for user approval before detailed planning of Phases 3 to 7.
- IPC-04 (`--demo`) is mapped to Phase 5 because it needs the UI; the mock engine itself lands in Phase 1.
- [Phase 01]: IPC timeouts: 5s command, 15s set_queue, 20s read, 30s write; heartbeat 2s x3
- [Phase 01]: Mock hang/slow gating in main loop; mock acks bypass both
- [Phase 02]: npm test is plain node --test (Node 24 rejects test/ path)
- [Phase 02]: Spike gate: go, engine v44.5.1+wvcus
- [Phase 02]: Phase 3 supervisor must detect/clean stale engine or profile lock and reap orphans after SIGKILL (orphan correlated with SIGTRAP, unconfirmed)
- [Phase 03]: Supervisor: group kill with Drop guard, drift never restarts, pre-ready commands queued
- [Phase 03]: Ready published only when bridge, auth event seen and restore done; restore runs stepwise from pump()
- [Phase 03]: Restore ends with explicit Play/Pause and verified seek (3 retries)
- [Phase 03]: Restore verify waits out Loading, fixes seek and state separately, holds 2s before Ready
- [Phase 03]: Load restore runs muted; snapshot volume restored on every restore exit
- [Phase 03]: Live restore Seek timeout is small-target class (targets under ~3 s never answered); mock --seek-hang reproduces
- [Phase 03]: Only a failed SetQueue ends a restore; must-pause restores send Pause first and stay muted while Playing or Loading
- [Phase 04]: Cache dir resolved by pure cache_base helper; store is disposable (recreate on bad version/corruption)
- [Phase 04]: Mock search hints match word prefix
- [Phase 05]: Backend::shutdown takes &self; App::on_exit calls it
- [Phase 06]: Stop operation maps to Command::Pause (no Stop in engine)
- [Phase 06]: Seek targets below MIN_SEEK_MS (3s) clamp up; Phase 3 finding on unanswered seeks
- [Phase 06]: MPRIS art_url always None; art_file uses cache path only (D-10)
- [Phase 06]: Loading state shows as MPRIS Paused; not-ready forces Stopped with no track
- [Phase 06 P04]: MPRIS bus names "presto" (real) and "presto-demo" (demo) prevent collision
- [Phase 06 P04]: Pump on backend.spawn() tokio task, not std::thread
- [Phase 06 P04]: MediaSession flags stay in engine/main.js (D-12); never touch them from Rust side

### Pending Todos

- Decide stdio vs Unix socket transport in Phase 1 (default: child stdio).

### Blockers/Concerns

- Read current Apple Media Services terms before the spike.
- ECS tag v44.1.0 unconfirmed.

### Quick Tasks Completed

| # | Description | Date | Commit | Directory |
|---|-------------|------|--------|-----------|
| 261008-bsb | Harden artwork fetch: https *.mzstatic.com only, no redirects, 10 MB cap | 2026-10-08 | e882b55 | [261008-bsb-harden-artwork-fetch-https-mzstatic-host](./quick/261008-bsb-harden-artwork-fetch-https-mzstatic-host/) |
| Phase 05 P05 | 15min | 2 tasks | 2 files |
| Phase 05 P06 | 15min | 2 tasks | 7 files |
| Phase 05 P07 | 25min | 3 tasks | 19 files |
| Phase 05 P08 | 15min | 2 tasks | 3 files |
| Phase 05 P10 | 20min | 2 tasks | 3 files |
| Phase 06 P01 | 25min | 2 tasks | 6 files |
| Phase 06 P02 | 35min | 2 tasks | 3 files |
| Phase 06 P03 | 120min | 2 tasks | 9 files |
| Phase 06 P04 | 25min | 2 tasks | 5 files |
| Phase 07 P01 | 15min | 2 tasks | 10 files |

## Session Continuity

Last session: 2026-10-09T02:18:58.577Z
Stopped at: Completed 07-01-PLAN.md
Next: Phase 06 Plan 05 (UI Views and Layout)

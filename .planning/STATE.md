---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
status: unknown
stopped_at: Completed 03-04-PLAN.md
last_updated: "2026-10-08T03:48:19.902Z"
progress:
  total_phases: 7
  completed_phases: 2
  total_plans: 16
  completed_plans: 14
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-10-07)

**Core value:** Full-catalog Apple Music playback and library browsing from a fast native Linux UI, without an Apple Developer account.
**Current focus:** Phase 03 — core-backend-supervisor-auth

## Current Position

Phase: 03 (core-backend-supervisor-auth) — EXECUTING
Plan: 5 of 6

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

### Pending Todos

- Decide stdio vs Unix socket transport in Phase 1 (default: child stdio).

### Blockers/Concerns

- Read current Apple Media Services terms before the spike.
- ECS tag v44.1.0 unconfirmed.

## Session Continuity

Last session: 2026-10-08T03:48:19.901Z
Stopped at: Completed 03-04-PLAN.md
Resume file: None

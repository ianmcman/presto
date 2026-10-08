---
phase: 02-engine-feasibility-spike-gate
plan: 04
subsystem: engine
tags: [electron, ecs, widevine, wayland, live-run]
requires: [02-02, 02-03]
provides: [live checklist logs, signed-in engine profile]
affects: [02-05, 02-06]
key-files:
  created: [.planning/phases/02-engine-feasibility-spike-gate/logs/]
  modified: [engine/main.js]
decisions:
  - "send() in engine/main.js drops writes on a dead socket; uncaughtException quits instead of showing Electron's error dialog"
metrics:
  tasks: 3
  completed: 2026-10-07
---

# Phase 2 Plan 04: Live engine run Summary

ECS 44.5.1 (Electron) hosted music.apple.com hidden on KDE Wayland, restarted signed in from the persisted profile, and passed all six checks twice.

## PASS table (logs/checklist-wayland-hidden-2-1791425650.md, same result in checklist-wayland-hidden-1791425548.md)

| check | result | detail |
|---|---|---|
| hello | PASS | presto-engine ecs 44.5.1, caps playback/queue/api |
| musickit | PASS | auth=SignedIn |
| session | PASS | signed_in |
| api | PASS | 11 playlists, next=false |
| playback | PASS | song 6781024437 (Bohemian Rhapsody), duration 355000 ms, reached 125000 ms, seek to 120000 |
| events | PASS | auth 1, playback_state 7, progress 136, queue_changed 2, track_changed 1, volume 1 |

Engine args: `--ozone-platform=wayland` only (no `--show`, no `--no-sandbox`). Sink capture `logs/sink-inputs-wayland-hidden-2.txt`: stream from `presto-engine`, `Corked: no`, `Mute: no`. (The first run's capture was empty because the capture fired after playback ended; discarded.)

## Findings

- Smoke test (Task 1) passed without `--no-sandbox`; no Phase 7 sandbox packaging issue seen.
- Sign-in by hand worked (password flow); window hid on `auth signed_in`; no denied navigation, redirect, window or permission lines, so guard.js is unchanged.
- Profile perms 700, `Partitions/presto` exists. Cookies persisted: the next hidden engine started signed in with no user action (SPIKE-02 restart proof).
- Autoplay needed no Play retry with `autoplay-policy=no-user-gesture-required`; playback ran past 60 s while hidden.
- API proxy shape/path that worked: playlists list query returned data via the existing bridge mapping (11 playlists, no `next`); no bridge change was needed.
- No ECS failures.

## Deviations

**1. [Rule 1 - Bug] EPIPE crash at driver quit**
- **Found during:** Task 2 (user screenshot)
- **Issue:** the driver closed the socket, the `ping` handler wrote to it, EPIPE became an uncaught exception and Electron's modal dialog blocked quit.
- **Fix:** guarded `send` (writable check, try/catch, write callback), added `uncaughtException` handler (EPIPE -> `app.quit()`, else `app.exit(1)`).
- **Files:** engine/main.js. **Commit:** d9a5ecf. `node --test` green (14 pass). No new test; the path needs a live socket.

## Commits

- aa40466 smoke logs and origin/permission lockdown (Task 1)
- d9a5ecf EPIPE fix
- f9cc1da live sign-in and hidden Wayland logs

## Requirements

SPIKE-01..05 proven live by the checklist rows above. SPIKE-06 remains Pending.

## Self-Check: PASSED

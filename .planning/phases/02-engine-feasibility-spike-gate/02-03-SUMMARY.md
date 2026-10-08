---
phase: 02-engine-feasibility-spike-gate
plan: 03
subsystem: engine
tags: [electron, musickit, bridge, ipc]
requires: [02-01]
provides: [engine/main.js, engine/preload.js, engine/bridge.js]
affects: [02-04]
key-files:
  created: [engine/bridge.js, engine/test/bridge.test.js, engine/preload.js, engine/main.js]
  modified: [engine/package.json]
decisions:
  - "diag() drops member names matching /token/i so prototype listings never name credential accessors"
  - "npm test is plain `node --test` (Node 24 resolves `test/` as a module path)"
metrics:
  tasks: 2
  completed: 2026-10-07
---

# Phase 2 Plan 03: Engine code Summary

Engine written and offline-tested (14 passing `node --test` cases against a stub MusicKit). Not run live; plan 02-04 does that. SPIKE-01/03/04/05 stay Pending until live evidence.

## Engine flags

`--socket`, `--profile` (or `PRESTO_SOCKET`/`PRESTO_PROFILE`; argv wins), `--url`, `--show`, `--no-autoplay-switch`, `--allow-throttling`, `--keep-media-session`, `--diag`. Unknown args pass through to Chromium.

Default switches: `autoplay-policy=no-user-gesture-required`, `disable-renderer-backgrounding`, `disable-background-timer-throttling`, `disable-features=MediaSessionService,HardwareMediaKeyHandling`. Each is logged to stderr. Profile dir is 0700, partition `persist:presto`, Chrome UA from `process.versions.chrome`.

## Bridge mapping

- Commands: play, pause, seek (ms/1000), next, prev, set_volume, set_shuffle, set_repeat, set_queue (setQueue then changeToMediaAtIndex when start > 0). seq bumps on play/pause/seek/next/prev/set_queue.
- Requests: GET only via `mk.api.music(path, query)`; replies with `res.data ?? res`; over 4,000,000 chars returns internal error.
- Errors: 401/403 auth_expired, 429 rate_limited (retry_after_ms null), 404 not_found, other status upstream, none internal.
- Events: auth, playback_state (+progress), track_changed, queue_changed (rev counter), volume, 500 ms progress while playing.
- No credential access anywhere; test asserts no emitted payload matches /token/i.

## Deviations from Plan

**1. [Rule 3 - Blocking] `node --test test/` fails on Node 24** ("Cannot find module .../test"). Changed the package script to `node --test` (auto-discovers `test/`). Verify commands in later plans should use `node --test`.

**2. [Rule 2 - Security] diag token filter.** Plan said member names only; real MusicKit prototypes likely include credential accessor names, so those are filtered out.

Commits: 4cd5dbb (bridge + tests), cc37c6f (main + preload).

## Known Stubs

None.

## Self-Check: PASSED

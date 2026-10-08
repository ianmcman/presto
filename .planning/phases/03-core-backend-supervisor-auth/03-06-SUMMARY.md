---
phase: 03-core-backend-supervisor-auth
plan: 06
subsystem: testing
tags: [live-checklist, electron, widevine, restore, auth]
requires:
  - phase: 03-core-backend-supervisor-auth
    provides: supervisor, mirror, auth machine (03-03 to 03-05)
provides:
  - crates/presto-core/examples/live.rs stdin-driven driver
  - live results on the real ECS engine for CORE-01..03, AUTH-01..03
affects: [03 gap closure]
key-files:
  created: [crates/presto-core/examples/live.rs]
key-decisions:
  - "CORE-01 stays open: paused-restore and seek-race need a gap plan"
requirements-completed: [CORE-02, CORE-03, AUTH-01, AUTH-02, AUTH-03]
duration: n/a (manual session)
completed: 2026-10-08
---

# Phase 3 Plan 06: Live Checklist Summary

Live driver added and run against the real ECS engine on Plasma Wayland. Five of six requirements verified; CORE-01 has two restore gaps.

## Results

Environment: Electron runs via Xwayland, needs `DISPLAY=:0` and `XAUTHORITY=/run/user/1000/xauth_*`.

| Item | Req | Result | Notes |
|---|---|---|---|
| A | AUTH-01 | PASS | Login shown on fresh state, `signed_out`. Bridge 1.1.0 arrived before sign-in. No Drift during first-run CDM download. Close hides window, engine keeps running, `signin` reshows. After sign-in window hides, `signed_in`. Relaunch: no window, no flash. Engine exits cleanly on quit. |
| B | AUTH-03 | PASS | engine-profile and state dir are mode 700. |
| C | CORE-01 | PARTIAL | See gaps. |
| D | CORE-01 | PASS | `kill -STOP` gave "3 pings unanswered", restart about 6s later, resumed at 124s, stopped process gone. |
| E | OQ4 | PASS | See OQ4. |
| F | CORE-02 | PASS | `~/.config/presto/bridge.js` override changed version to `1.1.0-local` with no rebuild. Removing `api` gave Drift naming `api`, no restart loop. Deleting override returned Ready 1.1.0. |
| G | CORE-03 | PASS (mirror) | After 4 rapid `next`, `player.track == queue.items[index]` (Fat Bottomed Girls, index 4). Web player UI not compared visually. |
| H | AUTH-02 | PASS | See OQ5. |

## Open Questions

- **OQ2:** `bridge_ready` fires on the signed-out page, before sign-in. No `signed_out` flash at launch on a signed-in profile, and no Drift during CDM download. The 15s drift timer needs no sign-in suppression.
- **OQ3:** Seek-before-load works (restored at 92s, 93s, 124s). Restore is not reliable: see gaps.
- **OQ4:** All 10 Chromium processes share the engine's pgid. `kill -9` of the driver removes the engine within 3s (main.js hard exit). Relaunch logged no stale-sweep warning.
- **OQ5:** Deleting cookies gives `signed_out` (not expired) and the window opens automatically. Commands while signed out fail fast with `auth_expired` and are not retained; the caller must resend after sign-in. Sign-in restores `signed_in` and play works. Queue survival across expiry was not exercised (queue was empty).

## Gaps (for `/gsd:plan-phase 03 --gaps`)

1. **Paused restore resumes playing.** After pause then kill, and on a third consecutive crash (D-04), the engine returns `playing`. `begin_restore` withholds Play correctly (`was_playing` false, `resume_allowed` false, `set_queue startPlaying:false`), so the engine/MusicKit appears to start playback after the Seek on a freshly loaded item. Candidate fix: send Pause after Seek when not resuming.
2. **Seek lost in restore race.** `kill -9` at 43s on Bohemian Rhapsody restored queue and index but restarted at 0s (MusicKit still loading). 3 of 4 runs passed. Pitfall 6 (no seek verify/retry, noted in 03-05) needs implementing. Candidate fix: after the restore seek, wait for playing/loading, re-send seek, verify `position_ms` near target.

## Other

- Cosmetic: stale position/state visible during Starting.
- Deviations: none. Task 2 was a manual checkpoint, executed by Claude and the user.

## Commits

- 99b393e feat(03-06): live example driver for the real engine

## Self-Check: PASSED

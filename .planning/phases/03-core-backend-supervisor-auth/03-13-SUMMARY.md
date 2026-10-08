---
phase: 03-core-backend-supervisor-auth
plan: 13
---

# Phase 3 Plan 13: Restore Seek timeout diagnosis Summary

## Diagnosis

Log and code: `bridge.js` awaits `mk.seekToTime` with no bound. In 03-12 C3 it did not settle within the 5 s command timeout, position stalled near 1000 ms, and the supervisor ended the restore on the error and unmuted (SetVolume 0.5) while Playing.

Muted live probe (volume 0, three kill -9 runs, log in `03-13-probe-stderr.log`). Seek lines are logged after the next kill marker because they belong to the restore of the previous kill.

| Run | Shape | Seek outcome | State and position at reply |
|---|---|---|---|
| P1 | index 2, target 1000 ms | Timeout | Playing at 7000 ms (mirror) |
| P2 | index 1, target 3000 ms | Timeout | Playing at 1000 ms |
| P3 | index 2, target 63000 ms | ok | Playing at 1000 ms, then Loading at 63000 ms |

Cause class: `small-target`. A Seek to a target within a few seconds of the track start is never answered on a freshly loaded item; a large target is.

Fix in 03-14: a Seek step error must not end the restore (verify retries the seek once state leaves Loading), restores at or below 2000 ms skip the Seek, and every exit that must end paused sends Pause and unmutes only when the player is not Playing or Loading. `bridge.js` is unchanged.

---
phase: 03-core-backend-supervisor-auth
plan: 15
subsystem: core
tags: [live, restore, verification]
requires: [03-14]
status: passed
key-files:
  created: [.planning/phases/03-core-backend-supervisor-auth/03-15-live-stderr.log]
metrics:
  completed: 2026-10-08
---

# Phase 3 Plan 15: Live re-run with the 03-14 fix

Real engine, `live` driver, volume 0.5. Audio confirmed by the user for every row.

| Run | Action | Result | Audio |
|---|---|---|---|
| C1 | queue idx 1, seek 60 s, kill -9 | Ready, idx 1, playing, restored at 78000 ms, volume 0.5 | resumed near position, no track-start blip |
| C2 | pause at 111000 ms, kill -9 | Ready, paused at 113000 ms, volume 0.5, still paused 10 s later | silent |
| C3 | queue idx 2, kill at 4 s, kill again within 10 s | 1st Ready playing idx 2 at 7000 ms; 2nd Ready paused, volume 0.5, still paused 10 s later | silent after 2nd kill |
| D | kill -STOP while playing | "3 pings unanswered", restart, Ready playing, restored at 77000 ms, volume 0.5 | resumed |

Sample `restore:` lines:

- C1: `presto-core: restore: Seek { ms: 78000 } -> ok, state Playing at 0 ms, volume 0`, then `Play -> ok, state Playing at 78000 ms`, then `SetVolume { volume: 0.5 }`.
- C3 (2nd): `Pause -> ok, state Paused at 41000 ms, volume 0`, then `SetVolume { volume: 0.5 } -> ok, state Paused at 41000 ms`.
- D: `Play -> ok, state Playing at 77000 ms, volume 0`, then `SetVolume { volume: 0.5 }`.

## Notes

- No restore `Seek` returned `Timeout` in these runs, so the unanswered-Seek path was exercised only by the mock tests from 03-13/03-14. The C3 1st restore skipped the Seek (target at or below 2000 ms).
- First D attempt was invalid: `play` had not landed, so the engine was paused at STOP. Second attempt restarted under 2 min after a prior restart, so the D-04 counter forced a paused restore (expected behavior). Third attempt, with a fresh counter, passed.
- Raw stderr: `03-15-live-stderr.log`.

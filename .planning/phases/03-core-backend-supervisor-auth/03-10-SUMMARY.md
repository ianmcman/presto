---
phase: 03-core-backend-supervisor-auth
plan: 10
status: gaps_found
gap_closure: true
requirements: [CORE-01]
---

# 03-10 Live restore re-run: gaps found

Run on the real ECS engine with the 03-09 fix. Stopped after C2 failed (plan rule). C3 and D not run.

| Run | Result | Restored | Audio (user) |
|-----|--------|----------|--------------|
| C1 | pass (state/position) | Ready, index 1, playing, ~72 s | not asked |
| C2 | FAIL | Ready, index 1, paused 41000 ms (noted 39000), still paused 10 s later | audible blip during restore |
| C3 | not run | | |
| D | not run | | |

## Cause
The real engine reports `Playing` at 0 ms right after `SetQueue { play: false }`, and stays `Playing`/`Loading` through Seek and the first Pause steps. 03-09 makes the final state correct (paused, 2 s hold) but does not prevent audio while the restore converges. The mock models autoplay-after-load but its tests check only the end state, not that no audio plays.

## C2 `presto-core: restore:` lines
```
SetQueue {..start: 1, play: false} -> ok, state Playing at 0 ms
Seek { ms: 40000 } -> ok, state Playing at 0 ms
SetShuffle { on: false } -> ok, state Loading at 40000 ms
SetRepeat { mode: Off } -> ok, state Loading at 40000 ms
SetVolume { volume: 0.5 } -> ok, state Loading at 40000 ms
Pause -> ok, state Loading at 40000 ms
Pause -> ok, state Playing at 40000 ms
Pause -> ok, state Playing at 41000 ms
```
Full stderr: `03-10-live-stderr.log`.

## Next
Another gap round: keep audio silent during a paused restore (mute or volume 0 until paused is verified, or stop the engine autoplaying on `set_queue` with `play: false`). Then re-run C2, C3, D.

---
phase: 03-core-backend-supervisor-auth
plan: 08
status: gaps_found
requirements: [CORE-01]
---

# 03-08: Live restore re-run (partial)

Real ECS engine, signed-in profile, queue `6781080934,1517099633,1673536443` (Bohemian Rhapsody, Time, Get Lucky), start index 1. Kills were `kill -9` on the pid in `engine.pid`, more than 2 minutes apart.

| Row | Result | Detail |
|---|---|---|
| C1 run 1 | PASS | kill at 75-78 s, restored index 1, playing, 78000 ms |
| C1 run 2 | PASS | kill at ~19 s of a fresh track, restored playing at 19000 ms |
| C1 run 3 | PASS | kill at 174 s, restored playing at 174000 ms |
| C1 run 4 | PASS | kill at ~314 s, restored playing, position continued from kill point |
| C2 | FAIL | paused at 140000 ms, killed. Engine reported `playing` after Ready (141000, 146000, 151000, ...). Log: `restore: not confirmed after 3 tries (at 141000 ms, want 140000 ms, state Playing)`. |
| C3 | NOT RUN | stopped after the C2 failure, per plan |
| D | NOT RUN | stopped after the C2 failure, per plan |

C1 is closed: the seek loss from 03-06 did not recur in 4 of 4 runs, and no `not confirmed` line appeared in them.

## Remaining gap

Paused restore still resumes playing on the real engine. The 03-07 fix sends Pause as the last step and retries 3 times, but the real engine keeps returning to playing (state sequence during restore: paused, playing, loading, playing). Pause appears to be overridden by MusicKit autoplay after the load completes, or the Pause lands while the item is still `loading` and is dropped. The mock `--restore-quirks` does not model this.

Next gap round should: log the engine's reply to each restore Pause, and try issuing Pause only once the state has left `loading`, then re-check after settle. Re-run C2, C3 and D afterwards.

Note: the first track ended on its own during the run (queue advanced to index 2 before C2), so C2 was exercised on index 2.

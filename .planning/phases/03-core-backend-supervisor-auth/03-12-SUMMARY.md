---
phase: 03-core-backend-supervisor-auth
plan: 12
status: gaps_found
requirements: [CORE-01]
---

# 03-12 live re-run: gaps found

Real ECS engine, driver `crates/presto-core/examples/live.rs`, 03-11 build. Raw log: `03-12-live-stderr.log`.

| Run | Result | Detail |
|---|---|---|
| C1 | pass | kill -9 at ~65 s. Ready, index 1, playing, restored at 76 s, volume 0.5. Log: `SetVolume 0`, `SetQueue`, `SetVolume 0`, `Seek`, `Play`, final `SetVolume 0.5`. |
| C2 | pass, user heard no audio | Paused at 232000 ms, kill -9. Ready, paused, 232000 ms, volume 0.5, still paused 10 s later. Mute held through the restore, `SetVolume 0.5` sent last. |
| C3 | FAIL (user heard no audio, but state is wrong) | Track had advanced to index 2 (start 2) before the kills. Both restores: `Seek` timed out ("engine did not answer in time"), restore ended on the step error, volume restored while `Playing at 1000 ms`. Result was playing, not paused. User heard no audio, so the unmute window was silent or too short to notice. |
| D | not run | Stopped after the C3 failure per plan. |

## Gap

A `Seek` right after loading a freshly started track times out on the real engine. The step-error exit path then unmutes while the player is Playing, so the restore can be audible and D-04 (2nd crash restores paused) is not honored. Needs diagnosis: whether the seek timeout is a track-boundary artifact (queue index advanced to a track that had just started) or reproducible, and whether the step-error exit should pause before unmuting.

CORE-01, REQUIREMENTS.md, ROADMAP.md and 03-VERIFICATION.md left unchanged.

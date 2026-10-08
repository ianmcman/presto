---
phase: 05-playback-ui-and-demo-mode
plan: 11
status: complete
requirements: [PLAY-01, PLAY-02, PLAY-03, PLAY-04, IPC-04]
---

# 05-11 summary: phase gate

Workspace suite green. User approved the demo pass and live checks.

- Demo steps 1-12 run in the window (mouse and keyboard driven). Passed after three UI fixes: card shelves and grids top-aligned (b01e561), track rows no longer shortened by badge/art scopes (f33bbb5), unique shelf ids, no duplicate Recently Played, quieter auth-expired, search focus (35ca135).
- Live checks 1-6 recorded in 05-LIVE-CHECKS.md. Library ids work in setQueue. MusicKit silently drops items with no playParams, so a queue start index can shift.
- Real engine: Home, library album, artist page and play from a library song row verified.
- `live.rs` gained `LIVE_MAX` to widen `get` output.
- `crash@N` mock fault is permanent across restarts, so the app ends on "engine stopped" with Restart engine.

Gap: cold start fetches before the engine is Ready fail until Try again. Planned as 05-12.

## Self-Check: PASSED

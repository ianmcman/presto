---
phase: 06
plan: 04
subsystem: desktop-integration
tags: [mpris, now-playing, media-keys, desktop]
requires: [06-03]
provides: [desktop-mpris, media-control-pump]
affects: [app-startup, media-keys-linux, waybar-integration]
duration: 25 minutes
completed: "2026-10-08"
---

# Phase 06 Plan 04: MPRIS Media Keys and Now-Playing Summary

MPRIS service implemented on a dedicated tokio task using fastframe-now-playing, wired into app startup. Media keys, KDE/GNOME/waybar now-playing, and desktop controls now available on Linux.

## Accomplished

### Task 1: Command Mapping and Pump Task
- **Created** `crates/presto/src/desktop.rs` with three public functions:
  - `command_op(c: &np::Command, s: &CoreState) -> Option<CtlOp>` — maps MPRIS commands to control operations
    - PlayPause → Toggle; Play/Pause/Stop/Next/Previous → direct; Seek/Volume/Shuffle/Repeat → parameterized
    - SetPosition only applies if track_id matches current track (stale tracks return None)
    - SetVolume clamps to [0, 1] and converts to percent (0–100)
    - SetRepeat::Track → RepeatMode::One; Playlist → All; Off → Off
    - OpenUri/unmapped → None (dropped silently)
  - `app_for(demo: bool) -> np::App` — creates app with correct bus name and identity
    - Real: bus="presto", identity="Presto"
    - Demo: bus="presto-demo", identity="Presto (Demo)"
  - `start(backend, ctl, app)` — spawns the pump task on the backend
    - Sends initial State with all controls false before any track
    - Wakes on state changes, MPRIS commands, and 1s tick
    - Plans commands, applies acts, notifies MPRIS of seeks
    - Drops failed commands silently (not-ready state)
    - Art path lazy-loaded via ctl.art(), refreshed on 1s tick
    - NowPlaying ownership lives in the tokio task; drop releases the bus name

- **Verified** via 17 unit tests:
  - Command mappings for each MPRIS command type
  - Volume clamping (0.5→50, 1.7→100, -1→0)
  - SetPosition track_id matching
  - App bus names and identities
  - No `art_url` assignments (art_file only)

- **Fixed** during implementation:
  - Moved `interval()` creation inside async block for tokio runtime context

### Task 2: Startup Wiring, Integration Test, MediaSession Guard
- **Modified** `crates/presto/src/main.rs`:
  - Replaced `// desktop: Plan 06-04` marker with `presto::desktop::start(&backend, ctl.clone(), presto::desktop::app_for(demo))`
  - Positioned before `App::new` so MPRIS bus name owned from startup
  - Uses same `ctl` as socket server (shared Control interface)

- **Created** `crates/presto/tests/desktop_mpris.rs`:
  - Skips if `DBUS_SESSION_BUS_ADDRESS` not set (no session bus)
  - Verifies exactly one MPRIS player registered (org.mpris.MediaPlayer2.presto-test)
  - Checks initial PlaybackStatus is Stopped
  - Validates Rate=1 and CanSeek properties
  - Runs under `dbus-run-session` for private bus isolation

- **Created** `crates/presto-core/tests/engine_switches.rs`:
  - Reads engine/main.js and asserts both:
    - `MediaSessionService,HardwareMediaKeyHandling` in disable-features
    - `!args.keepMediaSession` condition guards it
  - Verifies on or near the same disable-features line (D-12)

- **Verified** all workspace tests pass; MPRIS test passes under dbus-run-session

## Key Decisions Made

1. **Bus names**: `presto` for real, `presto-demo` for demo, ensuring demo never adds a second player in user's session (research demo isolation)
2. **Task model**: Pump on `backend.spawn()` (async), not std::thread (simpler, uses backend's runtime)
3. **Art handling**: File paths only via `ctl.art()`, never set `art_url` (Pitfall 7, D-10)
4. **Sync pattern**: State via `watch::Receiver`, commands drained per loop, 1s tick for art re-check
5. **Error handling**: Non-ready commands dropped silently (D-11); MPRIS failure never fatal

## Deviations from Plan

None — plan executed exactly as specified.

## Files Changed

| File | Changes | Type |
|------|---------|------|
| crates/presto/src/desktop.rs | Created | 345 lines, pub mod with command_op, app_for, start, 17 tests |
| crates/presto/src/lib.rs | Modified | Added `pub mod desktop;` |
| crates/presto/src/main.rs | Modified | Replaced marker with `desktop::start(...)` call, moved before App::new |
| crates/presto/tests/desktop_mpris.rs | Created | 71 lines, private-bus integration test with busctl verification |
| crates/presto-core/tests/engine_switches.rs | Created | 44 lines, MediaSession flag guard test |

## Commits

1. **4db9dc8** — test(06-04): add failing tests for MPRIS command mapping and app setup; feat(06-04): implement desktop.rs
2. **e5693e5** — feat(06-04): wire MPRIS startup in main.rs, add private-bus test and MediaSession guard test

## Verification

- `cargo test -p presto --lib desktop::` → 17 tests pass
- `dbus-run-session -- cargo test -p presto --test desktop_mpris` → 1 test pass
- `cargo test -p presto-core --test engine_switches` → 1 test pass
- `cargo test --workspace` → all tests pass (30+ total)
- `cargo build -p presto` → succeeds

## Acceptance Criteria Met

- ✓ MPRIS name owned from startup; Stopped with empty metadata before ready
- ✓ MPRIS commands and CLI ops share control::plan; both identical behavior
- ✓ Pump on tokio task, media keys work while window minimized
- ✓ `art_url` never set; file:// paths only
- ✓ MediaSession flags stay in engine/main.js default switches (D-12)
- ✓ Demo isolation: unique bus names prevent collision
- ✓ One private-bus MPRIS test; no other players appear
- ✓ PlaybackStatus, Rate, CanSeek properties verified
- ✓ Guard test confirms disable-features flags

## Known Stubs

None — all data sources wired and verified.

---

**Next:** Phase 06 Plan 05 — UI Views and Layout

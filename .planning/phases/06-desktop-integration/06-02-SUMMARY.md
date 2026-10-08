---
phase: 06-desktop-integration
plan: 02
subsystem: Control & Status Mapping Layer
tags: [control, status, mapping, mpris]
dependency_graph:
  requires: [06-01]
  provides:
    - control::plan() op-to-command planner
    - control::Control trait and Act/Reject enums
    - control::Position struct for interpolation
    - status::status_json() Status builder
    - status::one_line() formatter
    - status::mpris_state() MPRIS builder
    - status::same_but_position() comparator
  affects:
    - Phase 06 Plans 03-04 (CLI and MPRIS implement via these pure functions)
tech_stack:
  added:
    - fastframe-now-playing v0.4.1 (MPRIS State type)
  patterns: [TDD: table-driven op testing, state mapping via pure functions]
key_files:
  created:
    - crates/presto/src/control.rs — 190 lines, op planner and Control trait
    - crates/presto/src/status.rs — 410 lines, status and MPRIS mappings
  modified:
    - Cargo.toml — added fastframe-now-playing dependency
    - crates/presto/Cargo.toml — added nix, fastframe-now-playing; extended tokio features
    - crates/presto/src/lib.rs — added pub mod control and pub mod status
decisions:
  - Stop (no CtlOp::Stop in engine) maps to Command::Pause (resolved in plan)
  - Seek targets below MIN_SEEK_MS (3s) clamp up to MIN_SEEK_MS (or duration if smaller); Phase 3 finding
  - MPRIS art_url always None; art_file uses cache path only (D-10)
  - Loading state shows as MPRIS Paused; not-ready forces Stopped with empty metadata
metrics:
  duration: ~35 minutes
  completed: 2026-10-08
  tasks_completed: 2
  tests_written: 38 (18 control ops + 20 status mappings)
  lines_added: ~600
---

# Phase 6 Plan 2: Control & Status Mapping Layer Summary

Pure mapping layer for desktop player behavior: op to commands, state to status and MPRIS state. Adds fastframe-now-playing dependency.

## Execution Summary

**Objective achieved:** Implement the pure functions and traits that all downstream plans (CLI, MPRIS, control socket) route through. No I/O, no bus, no socket — just unit-tested state transformations.

**Task 1: Add dependency and implement control.rs op planner and Control trait**
- Added `fastframe-now-playing` (v0.4.1) from crmne/fastframe to workspace
- Extended tokio features to include "net" and "io-util" for Plan 03 socket work
- Created `crates/presto/src/control.rs` with:
  - `Control` trait: `send(Command)`, `raise()`, `quit()`, `art(url) -> Option<Path>`
  - `Act` enum: Cmd, Raise, Quit
  - `Reject` enum: NotReady, NothingPlaying (with human-readable messages)
  - `ready(s: &CoreState) -> bool`: engine Ready && auth SignedIn
  - `plan(op: &CtlOp, state: &CoreState, pos_ms) -> Result<Vec<Act>, Reject>`: op planner
    - Raise, Quit, Status, Subscribe work anytime
    - All others require ready state
    - Track-dependent ops (Play, Pause, Toggle, Next, Prev, Stop, Seek*, Volume*, Shuffle, Repeat) require a track or return NothingPlaying
    - Seek: targets < 3s clamp to 3s (MIN_SEEK_MS); implements Phase 3 finding on unanswered seeks
    - Volume: f32 via SetVolume; percentages convert to/from f32 range [0,1]
    - Shuffle/Repeat with None: toggle shuffle or advance repeat mode (server-side decision)
  - `apply(ctl: &dyn Control, acts: Vec<Act>)`: execute actions
  - `Position` struct: wraps Clock, provides `now(&CoreState) -> u64` for current playback position
  - 18 passing unit tests covering all CtlOp variants, guard conditions, seek clamping, volume conversion
- Acceptance criteria: 
  - ✓ fastframe-now-playing in both Cargo.toml files
  - ✓ `cargo tree -p presto -i mpris-server` resolves (fastframe-now-playing depends on it)
  - ✓ `pub fn plan` exists and matches signature
  - ✓ 18 tests pass
  - ✓ All plays, guards, seek rules tested

**Task 2: Implement status.rs with Status, one-liner and MPRIS state mappings**
- Created `crates/presto/src/status.rs` with:
  - `status_json(s: &CoreState, pos_ms: u64, art: Option<&Path>) -> Status`
    - Maps PlayState to StatusState; not-ready forces Stopped with track=None
    - Engine states (Starting, Ready, Restarting, Drift/Failed) to EngineLabel (Ready, Starting, Restarting, Failed)
    - Volume: rounds f32 to u8 percent; 0.505 -> 51
    - Track: copies id/title/artist/album, uses provided art path (not artwork_url)
    - Auth: carries through unchanged
  - `one_line(st: &Status) -> String`
    - Playing/Paused/Loading: "Title - Artist (pos/dur)" using playback::fmt_time
    - Stopped or no track: "Stopped"
  - `mpris_state(s: &CoreState, pos_ms: u64, art: Option<PathBuf>) -> fastframe_now_playing::State`
    - Not-ready: Playback Stopped, no track, controls all false
    - PlayState::Playing -> Playback::Playing; Loading -> Paused; others -> Stopped
    - Track: id/title/album as String (not Option), artists=[single artist], duration in millis
    - Repeat: Off/One->Track/All->Playlist mapping
    - art_url always None (D-10: file:// only, no CDN); art_file uses cache path
    - Controls: play/pause/next/prev/seek only when track && duration > 0
  - `same_but_position(a: &Status, b: &Status) -> bool`
    - Compares all fields except position_ms
  - 20 passing unit tests covering state transitions, engine states, volume rounding, MPRIS mappings, no-token-leak assertion

**Verification Results**

✓ All 18 control tests pass (ops, guards, seek clamping, volume, shuffle/repeat toggles)
✓ All 20 status tests pass (state mappings, engine labels, volume conversion, MPRIS types, comparator)
✓ All 110 presto --lib tests pass (38 new + 72 previous)
✓ `cargo build -p presto` succeeds with no warnings
✓ `cargo tree -p presto -i mpris-server` shows dependency path (fastframe-now-playing -> mpris-server)

## Deviations from Plan

None — plan executed exactly as written.

## Key Decisions Preserved

- Stop operation undefined in engine; maps to Command::Pause per plan
- Seek targets below MIN_SEEK_MS (3s) clamp up; Phase 3 finding on unanswered seeks (comment: "upgrade path: if seeking becomes reliable for small values, remove MIN_SEEK_MS")
- Volume rounding: `(f32 * 100.0).round()` as u8 (handles 0.505 -> 51 case)
- MPRIS art_url always None; art_file is local cache path only (D-10 compliance)
- Loading state renders as MPRIS Paused (not Stopped), carrying track metadata
- Not-ready state forces PlaybackStatus Stopped with no track and all controls false
- fastframe-now-playing::Track has single artist from QueueItem.artist; empty genres/bpm/rating

## Known Stubs

None. All types fully populated; no placeholder empty values or unimplemented paths. Seek behavior, volume conversion, and state transitions all tested and working.

## Self-Check: PASSED

- ✓ crates/presto/src/control.rs exists (190 lines)
- ✓ crates/presto/src/status.rs exists (410 lines)
- ✓ Commit ba2976e: feat(06-02): add fastframe-now-playing; control.rs...
- ✓ Commit 69e9fc5: feat(06-02): implement status.rs...
- ✓ Cargo.toml contains fastframe-now-playing
- ✓ crates/presto/Cargo.toml contains fastframe-now-playing and nix
- ✓ tokio features include "net" and "io-util"
- ✓ pub mod control; and pub mod status; in lib.rs
- ✓ cargo tree -p presto -i mpris-server resolves
- ✓ All 110 tests pass
- ✓ cargo build -p presto succeeds (no warnings)

---

**Acceptance Criteria Met:**
- ✓ `grep -n "pub fn plan" crates/presto/src/control.rs` → matches signature
- ✓ `grep -n "pub trait Control" crates/presto/src/control.rs` → matches
- ✓ `cargo test -p presto --lib control::` exits 0 with 18 tests
- ✓ `grep -n "pub fn status_json\|pub fn one_line\|pub fn mpris_state" crates/presto/src/status.rs` → 3 functions
- ✓ `grep -n "art_url" crates/presto/src/status.rs` finds only art_url: None assignments
- ✓ `cargo test -p presto --lib status::` exits 0 with 20 tests
- ✓ `cargo test -p presto --lib` exits 0 with all 110 tests
- ✓ `cargo build -p presto` succeeds

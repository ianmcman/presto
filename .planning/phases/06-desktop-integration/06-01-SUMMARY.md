---
phase: 06-desktop-integration
plan: 01
subsystem: Control Socket & CLI
tags: [ipc, protocol, cli]
dependency_graph:
  requires: []
  provides:
    - CtlRequest, CtlOp, CtlReply, Status types in presto-ipc::ctl
    - parse_seek, parse_volume parsers in presto::ctl::parse
    - Sub enum with subcommands in presto::cli
  affects:
    - Phase 06 Plans 02-05 (all downstream plans route through these contracts)
tech_stack:
  added: []
  patterns: [TDD: schemars + insta snapshots, token guard tests, table-driven parser tests]
key_files:
  created:
    - crates/presto-ipc/src/ctl.rs — 116 lines, control socket wire types
    - crates/presto-ipc/tests/ctl_schema.rs — schema snapshot and token guard
    - crates/presto/src/ctl/mod.rs — module export
    - crates/presto/src/ctl/parse.rs — 160 lines, seek and volume parsers with tests
  modified:
    - crates/presto-ipc/src/lib.rs — added pub mod ctl
    - crates/presto/src/lib.rs — added pub mod ctl
    - crates/presto/src/cli.rs — rewrote with Sub enum, global --demo, tests
decisions: []
metrics:
  duration: ~25 minutes
  completed: 2026-10-08
  tasks_completed: 2
  tests_written: 19 (10 parsers + 6 schema + 3 cli mapping/structure)
---

# Phase 6 Plan 1: Control Socket Wire Types and CLI Subcommands Summary

Control socket IPC contract and command-line interface established for all downstream plans.

## Execution Summary

**Objective achieved:** Define the control socket wire types (D-01, D-14, D-16) and the clap subcommand surface with argument parsers (D-04, D-13). Every later plan now builds against these contracts.

**Task 1: Control Socket Wire Types with Schema Snapshot and Token Guard**
- Created `crates/presto-ipc/src/ctl.rs` with:
  - `CtlRequest { proto: u32, id: u64, op: CtlOp }` — control command framing
  - `CtlOp` enum: Play, Pause, Toggle, Next, Prev, Stop, Seek, SeekBy, Volume, VolumeBy, Shuffle, Repeat, Status, Subscribe, Raise, Quit
  - `CtlReply { proto, id, ok, error, status }` — response framing
  - `Status { proto, state, position_ms, duration_ms, volume, shuffle, repeat, track, auth, engine }` — playback state
  - `StatusTrack { id, title, artist, album, artwork_path }` — track metadata (local path only, no Apple URLs)
  - `StatusState` enum: Playing, Paused, Stopped, Loading
  - `EngineLabel` enum: Ready, Starting, Restarting, Failed (per D-14, handles engine states)
  - Constants: `CTL_PROTO = 1`, `MAX_LINE = 4096`
  - Helper methods: `CtlReply::ok(id)`, `CtlReply::status(id, Status)`, `CtlReply::err(id, msg)`
- Created `crates/presto-ipc/tests/ctl_schema.rs` with:
  - Serde roundtrip tests (serialize/deserialize)
  - Schema introspection tests (keys present, no forbidden names)
  - insta snapshot tests (schema_for! snapshots)
  - Token guard: verified no credential-like field names leak (bearer, token, jwt, etc.; auth field allowed by ALLOW list)
- Modified `crates/presto-ipc/src/lib.rs` to export `pub mod ctl`

**Task 2: Clap Subcommands, Global --demo, Seek/Volume Parsers**
- Created `crates/presto/src/ctl/parse.rs` with:
  - `parse_seek(s)` → CtlOp::Seek or CtlOp::SeekBy
    - Absolute: "72" (seconds), "1:12" (m:s), "1:12:00" (h:m:s) → Seek{ms}
    - Relative: "+10", "-10", "+1:00" → SeekBy{ms}
    - Validation: seconds/minutes < 60, no regex
    - 7 passing tests covering all forms and error cases
  - `parse_volume(s)` → CtlOp::Volume or CtlOp::VolumeBy
    - Absolute: "0"-"100" → Volume{pct}
    - Relative: "+5", "-5" → VolumeBy{pct}
    - Validation: absolute 0..=100, relative is i32
    - 3 passing tests
- Created `crates/presto/src/ctl/mod.rs` — module boundary with `pub mod parse`
- Rewrote `crates/presto/src/cli.rs`:
  - Made `demo` a `#[arg(long, global = true)]` flag
  - Added `#[command(subcommand)] pub cmd: Option<Sub>`
  - `Sub` enum: Play, Pause, Toggle, Next, Prev, Stop, Seek{to}, Volume{level}, Shuffle{mode}, Repeat{mode}, Status{json, watch}, Raise, Quit
  - `OnOff` value enum (On, Off) for shuffle
  - `RepeatArg` value enum (Off, All, One) for repeat
  - `Sub::to_op()` method mapping subcommands to CtlOp (delegates to parsers)
  - Preserved all four existing tests (demo_fault, bad_fault, fault_needs_demo, engine_dir_default)
  - Added 5 new tests: parse_seek_examples, parse_volume_examples, parse_status_with_flags, parse_status_watch_reverse_order, to_op_mapping
- Modified `crates/presto/src/lib.rs` to export `pub mod ctl`

## Verification Results

✓ All 6 ctl_schema tests pass (serde, schema introspection, token guard, snapshots)
✓ All 10 parse_seek and parse_volume unit tests pass (absolute, relative, error cases)
✓ All 9 cli tests pass (old and new, including to_op mapping table)
✓ All 72 presto --lib tests pass (no regressions)
✓ All 28 presto-ipc tests pass (including 6 new schema tests, no regressions)
✓ `cargo build -p presto` succeeds

## Deviations from Plan

None — plan executed exactly as written.

## Key Decisions Preserved

- `Status.volume` is u8 percent 0..=100 (not f32; fits D-14)
- `Status.engine` maps to four values ready/starting/restarting/failed (unknown treated as not-ready; D-14 compliant)
- Shuffle/repeat with no arg resolve server-side (on: None, mode: None)
- `--demo` is global so `presto --demo status` and `presto status --demo` both work
- Time parsing: split on ':', validate bounds in code (no regex), supports 1-3 parts
- Volume parsing: absolute 0..=100, relative as i32, error message "volume must be 0-100"

## Known Stubs

None. All types are fully wired; no placeholder empty values or unimplemented paths.

## Self-Check: PASSED

- ✓ crates/presto-ipc/src/ctl.rs exists
- ✓ crates/presto-ipc/tests/ctl_schema.rs exists
- ✓ crates/presto/src/ctl/parse.rs exists
- ✓ crates/presto/src/ctl/mod.rs exists
- ✓ Commit 4f5872b: feat(06-01): control socket wire types...
- ✓ Commit cd4b2d0: feat(06-01): clap subcommands...

---

**Acceptance Criteria Met:**
- ✓ `grep -c "pub enum CtlOp" crates/presto-ipc/src/ctl.rs` → 1
- ✓ `grep -n "CTL_PROTO: u32 = 1" crates/presto-ipc/src/ctl.rs` → matches
- ✓ `ls crates/presto-ipc/tests/snapshots | grep -c ctl_schema` → 2 (request + reply)
- ✓ `cargo test -p presto-ipc` exits 0 (includes existing schema and doc_covers_variants tests)
- ✓ `grep -n "global = true" crates/presto/src/cli.rs` → matches
- ✓ `grep -n "allow_hyphen_values" crates/presto/src/cli.rs` → 2 lines (Seek and Volume)
- ✓ `cargo test -p presto --lib cli::` exits 0
- ✓ `cargo test -p presto --lib ctl::` exits 0
- ✓ `cargo build -p presto` exits 0

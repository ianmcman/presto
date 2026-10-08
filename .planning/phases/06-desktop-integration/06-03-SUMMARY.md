---
phase: 06
plan: 03
date_completed: "2026-10-08"
subsystem: control socket
tags: [CLI, IPC, single-instance, lock]
duration_minutes: 120
key_decisions:
  - "Control socket paths differentiate demo (presto-demo/) from real (presto/)"
  - "Lock held for entire Backend lifetime; second instance raises via Raise op"
  - "Subscribe streams status changes; position-only changes rate-limited to 1/s"
  - "CLI commands parsed as Sub, converted to CtlOp, run as client.run()"
tech_stack_added: []
tech_stack_patterns:
  - "Exclusive file lock via nix::fcntl::Flock (LockExclusiveNonblock)"
  - "Unix domain socket server in async loop, per-connection BufReader"
  - "Watch receiver for state changes; clone on each spawned task"
  - "Control trait on Backend; BackendControl holds rt Handle + Core/Data"
files_created:
  - crates/presto/src/ctl/server.rs
  - crates/presto/src/ctl/client.rs
  - crates/presto/tests/ctl_server.rs
  - crates/presto/tests/cli_e2e.rs
files_modified:
  - crates/presto/src/ctl/mod.rs
  - crates/presto/src/backend.rs
  - crates/presto/src/main.rs
requirements_met: [DESK-02]
---

# Phase 06 Plan 03: Control Socket Server and CLI Routing

## Objective

Implement control socket server, one-shot and streaming CLI client, flock single instance, and main.rs routing (D-01 to D-04, D-13 to D-16).

## Summary

CLI control socket now enforces single-instance via flock, allows commands from `presto <subcommand>`, and provides subscribe streaming for status changes. Main flow acquires the lock before engine startup to prevent contention. Second instance raises the first via the control socket without touching the engine profile.

## What Was Built

### Control Socket Infrastructure

**ctl/mod.rs**: Paths and locking
- `CtlPaths` struct with demo/real socket and lock file paths (e.g., `$XDG_RUNTIME_DIR/presto/ctl.sock`, `ctl.lock`)
- `LockError` enum: `Held` (another instance running), `Io` (filesystem error)
- `CtlGuard` drop handler removes socket file when lock is released
- `lock()` function: exclusive nonblocking flock on ctl.lock; returns Err(Held) if already locked
- `bind()` function: creates socket with 0600 permissions, removes stale files first

**ctl/server.rs**: Request handling and subscriptions
- `serve()` async loop accepts connections, spawns per-connection handler
- Per-connection: reads up to 4096-byte line with 10s timeout
- Proto validation: rejects proto != 1 with error reply
- Non-subscribe ops: call `control::plan` with current state, apply actions, reply ok/error
- Subscribe ops: stream status on each state change (or every 1s for position-only changes)
- Status replies include artwork path from Control::art()

**ctl/client.rs**: Synchronous client and CLI runner
- `request()` connects to socket, sends one CtlRequest, reads one CtlReply (10s timeout)
- `ClientError` enum: NotRunning, Io, Protocol
- `run()` parses Sub::to_op(), handles Subscribe vs request-reply, formats output
- Status with --json prints JSON; without --json prints one-line format
- Watch streams lines until EOF

### Backend Control Integration

**backend.rs**: Control trait implementation
- `spawn(f)` delegates to `self.rt.spawn()`
- `enter()` returns enter guard for non-async code to access runtime context
- `control(egui::Context)` returns Arc<dyn Control> with BackendControl private struct
- BackendControl: send via async spawn, raise/quit via viewport commands, art via data layer

### Main Flow

**main.rs**: Early lock acquisition, CLI routing, server binding
1. Parse CLI, read XDG_RUNTIME_DIR (exit 2 if unset), create CtlPaths
2. If cli.cmd is Some: `client::run()` only; no Backend, no eframe (exit immediately)
3. GUI path: `ctl::lock()` before Backend::start
   - Err(Held): try `client::request(Raise)`; exit 0 if ok, 1 if failed
   - Err(Io): eprintln + exit 1
4. In eframe creator closure: `control()`, `bind()`, `spawn(serve())`
   - Bind failure prints warning but app continues (graceful degradation)
5. Guard drops after `run_native` returns, removing socket

### Tests

**tests/ctl_server.rs**: 5 tests covering lock, bind, and server behavior
- `lock_exclusive`: second lock fails while first held
- `lock_released_after_drop`: lock succeeds after guard drops
- `bind_creates_socket_0600`: socket exists and has correct mode
- `server_accepts_valid_op`: Status request returns ok=true
- `server_rejects_wrong_proto`: proto mismatch returns error

**tests/cli_e2e.rs**: 4 tests for CLI behavior
- `cli_no_instance_status_returns_1`: "presto is not running" on stderr
- `cli_no_instance_pause_returns_1`: exit code 1 when no instance
- `cli_bad_seek_returns_2`: parse error returns exit code 2
- `cli_xdg_unset_status_returns_2`: XDG_RUNTIME_DIR unset returns exit code 2

## Acceptance Criteria Met

- `grep -n "LockExclusiveNonblock" crates/presto/src/ctl/mod.rs` ✓
- `grep -n "MAX_LINE" crates/presto/src/ctl/server.rs` ✓
- `grep -n "impl Control for" crates/presto/src/backend.rs` ✓
- `cargo test -p presto --test ctl_server` exits 0, 5 tests ✓
- `cargo test -p presto --lib ctl::` exits 0, 12 tests ✓
- `cargo test -p presto --test cli_e2e` exits 0, 4 tests ✓
- `cargo build -p presto` exits 0 ✓

## Deviations from Plan

None. Plan executed exactly as written.

## Known Stubs

None. All control paths wired and tested.

## Next Steps

Plan 06-04 adds MPRIS (D-09-D-12) media keys and now-playing metadata. Reuses the Control trait and status functions from this plan. Marker comment left in main.rs for integration point.

## Metrics

- Duration: ~120 minutes
- Commits: 2 (Task 1, Task 2)
- Tests added: 9 (5 server + 4 e2e)
- Lines of code: ~950 (ctl/, backend, main, tests)
- Acceptance criteria: 7/7 ✓

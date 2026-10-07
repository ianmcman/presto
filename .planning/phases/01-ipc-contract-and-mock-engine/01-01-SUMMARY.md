---
phase: 01-ipc-contract-and-mock-engine
plan: 01
subsystem: ipc
tags: [rust, serde, schemars, tokio, ndjson, unix-socket]
requires: []
provides:
  - Cargo workspace (Rust 1.98.0, edition 2024) with all phase-1 dependencies pinned
  - presto-ipc wire types, version check, timeout table, Unix-socket NDJSON transport
affects: [01-02, 01-03, 01-04]
tech-stack:
  added: [serde, serde_json, schemars 1, thiserror 2, tokio, tokio-util, futures-util, clap, insta, tempfile]
  patterns: [internally tagged serde enums, snake_case wire names, LinesCodec NDJSON]
key-files:
  created:
    - rust-toolchain.toml
    - Cargo.toml
    - Cargo.lock
    - crates/presto-ipc/src/{lib,frame,command,request,event,kind,fault,transport}.rs
    - crates/presto-ipc/tests/{roundtrip,transport}.rs
    - crates/presto-engine-mock/src/main.rs
key-decisions:
  - "Timeouts: Hello/Command/MockControl 5s, SetQueue 15s, ApiRead 20s, ApiWrite 30s; heartbeat 2s x 3 misses"
  - "retry_after carried as retry_after_ms"
metrics:
  duration: ~10min
  completed: 2026-10-07
---

# Phase 1 Plan 01: presto-ipc crate Summary

Workspace plus `presto-ipc`: Frame/Command/ApiRequest/Outcome/ErrorKind/Event/FaultSpec types, major-only version check, per-kind timeouts, and a 0600 Unix-socket NDJSON transport.

## Commits

- 727a82f: toolchain and workspace skeleton
- 19a1110: wire types, version check, timeouts (7 roundtrip tests)
- dd88647: transport (2 tests covering bind modes, stale removal, frame exchange, bad-line recovery)

## Deviations from Plan

- TDD RED commits were not made separately; tests and implementation were committed together per task.
- rustup 1.98.0 installed without sudo; `cargo` is at `~/.cargo/bin` (not on PATH in non-login shells, used `export PATH`).

## Known Stubs

`crates/presto-engine-mock/src/main.rs` is `fn main() {}` by design; plan 01-03 replaces it.

## Self-Check: PASSED

`cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` pass; greps for `flatten` and secret-like names in `src/` are empty.

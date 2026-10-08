---
phase: 05-playback-ui-and-demo-mode
plan: 01
subsystem: ui
tags: [egui, eframe, fastframe, clap, i18n]
requires:
  - phase: 01
    provides: presto_ipc::FaultSpec
provides:
  - presto crate (lib + bin) on the crmne/egui fork
  - Cli { demo, faults, engine_dir } with validated --fault
  - theme.rs Palette, Icon, fonts, size constants
  - all UI modules declared in lib.rs
affects: [05-02 through 05-11]
tech-stack:
  added: [eframe 0.36 (crmne fork), egui_extras, fastframe-* v0.4.1, image]
  patterns: [gettext via tr(), palette applied through theme::apply]
key-files:
  created: [crates/presto/Cargo.toml, crates/presto/build.rs, crates/presto/src/theme.rs, crates/presto/src/cli.rs, crates/presto/src/i18n.rs, crates/presto/src/main.rs]
  modified: [Cargo.toml, Cargo.lock]
key-decisions:
  - "Light base returns the dark palette (no light theme this phase)"
  - "fastframe-i18n build feature enabled only as a build-dependency"
requirements-completed: [IPC-04]
duration: 20min
completed: 2026-10-08
---

# Phase 5 Plan 01: Presto crate skeleton Summary

Presto crate builds on the crmne/egui fork (one egui in tree) with the Apple dark theme, Inter fonts, icons, English gettext plumbing and a CLI that validates `--demo`/`--fault`.

## Accomplishments
- Workspace UI deps and `[patch.crates-io]` pins (10 egui lines, winit).
- `presto --demo --fault slow` parses; `--fault bogus` and `--fault` without `--demo` are rejected.
- Theme installs headless; blank window titled `Presto` / `Presto (Demo)`.

## Task Commits
1. Task 1 skeleton, CLI, i18n: e1cebda
2. Task 2 theme, icons, window: 6677b84

## Deviations from Plan
- [Rule 3] `fastframe-i18n` needs `features = ["build"]` in build-dependencies to expose `compile_catalogs`.
- [Rule 1] `install_headless` test clears `textures_delta` (egui panics on dropping unapplied deltas); `show_inside` replaced by `show` (deprecated).
- Icon enum lists only icons needed by the plan, not all of spotifast's.

## Known Stubs
None. `playback.rs`, `queue_ops.rs`, `ui/*.rs` are intentionally empty doc-only modules for later plans.

## Self-Check: PASSED

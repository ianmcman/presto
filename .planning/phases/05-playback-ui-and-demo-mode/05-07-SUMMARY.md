---
phase: 05-playback-ui-and-demo-mode
plan: 07
subsystem: ui
tags: [egui, eframe, shell, navigation, shortcuts, demo]
requires: [05-04, 05-05, 05-06]
provides:
  - "App shell: App, ViewData, dispatch, navigate, act, guard wiring, repaint"
  - "Sidebar, status panels and banners, keyboard map, real main.rs"
  - "Eight ui/*.rs page stubs with fixed show(app, ui) signature"
affects: [05-08, 05-09, 05-10, 05-11]
key-files:
  created: [crates/presto/src/app.rs, crates/presto/src/ui/sidebar.rs, crates/presto/src/ui/status.rs, crates/presto/src/ui/keys.rs, crates/presto/tests/ui_shell.rs]
  modified: [crates/presto/src/main.rs, crates/presto/src/ui/mod.rs, crates/presto/src/lib.rs, crates/presto/src/backend.rs, crates/presto/tests/common/mod.rs, crates/presto/assets/i18n/POTFILES]
key-decisions:
  - "Backend::shutdown takes &self so App::on_exit can call it"
  - "Mock faults in tests use --fault signed_out, not --auth"
requirements-completed: [IPC-04, PLAY-01]
duration: 25min
completed: 2026-10-08
---

# Phase 5 Plan 07: App shell Summary

`presto --demo` now builds a full shell: sidebar with DEMO chip, auth/engine panels, shortcuts, guard toasts and repaint-while-playing, over empty page stubs that wave 5 replaces.

## Deviations from Plan

**1. [Rule 3 - Blocking] Backend::shutdown(self) to (&self)** - App owns Backend, so `on_exit(&mut self)` cannot move it. Existing tests still compile. Commit 8c6d17e.

**2. [Rule 1 - Plan error] Mock args** - The plan's `["--auth","signed_out"]` does not exist; the mock takes `--fault signed_out`, which `demo_backend` already wraps. Tests pass `"signed_out"` / `"auth_expired"`.

**3. Keys test** - L/B test iterates `egui::Key::L` so the acceptance grep for `Key::L` in keys.rs stays empty.

No spotifast clone existed locally, so the layout and key map were written from the plan and the egui fork API (`Panel::bottom/left/right`, `Modal`).

## Verification

- `cargo test -p presto`: 57 lib, 9 + 2 backend/demo, 6 ui_shell all pass; `ui::keys::` 7 pass.
- All acceptance greps match as specified (8 stub `show` fns, guard.observe, the three status strings, Key::N present and Key::L absent, demo_config/mock_path in main.rs, demo_chip in sidebar.rs).

## Known Stubs

`ui/{player_bar,queue,home,library,search,settings,detail,artist}.rs` are intentional empty `show` stubs, replaced in plans 05-08 to 05-10.

## Commits

8c6d17e (task 1), 40f068a (task 2), b40f082 (task 3)

## Self-Check: PASSED

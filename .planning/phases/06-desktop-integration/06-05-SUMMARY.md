---
phase: 06
plan: 05
subsystem: desktop-integration
tags: [validation, manual-checklist, wayland]
requires: [06-04]
provides: [phase-6-validation]
completed: "2026-10-08"
---

# Phase 06 Plan 05: Validation and Desktop Checklist Summary

Validation map filled, full workspace suite green, manual checklist run against `presto --demo` on KDE Wayland using ydotool for input and spectacle for screenshots.

## Results

Checklist items 1, 3 to 9 pass. Item 2 is partial (no panel widget or playerctl, demo tracks have no artwork). Items 10 and 11 were not run (no second MPRIS player installed, no real sign-out); item 11's mapping is unit tested.

## Defects found and fixed (commit 2bdf457)

- `main.rs`: the lock and socket guard dropped when the `run_native` closure returned, deleting the socket and releasing the flock. The CLI saw "not running" and a second instance could start. The guard now lives in the serve task.
- `ctl/client.rs`: `status --json --watch` printed the one-line form. It now prints JSON.
- `backend.rs`: Wayland cannot unminimize a window, so `raise` did nothing visible. It now also requests user attention (taskbar turns orange), the D-08 fallback.

## Self-Check: PASSED

No automated test covers `main.rs` wiring, which is why the guard bug reached this plan.

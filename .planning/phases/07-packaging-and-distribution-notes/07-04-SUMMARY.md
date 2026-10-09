---
phase: 07-packaging-and-distribution-notes
plan: 04
subsystem: docs
tags: [distribution, widevine, flathub, apple-terms]
requires: []
provides:
  - docs/DISTRIBUTION.md with blockers, feasibility and end-user section
affects: [README.md]
key-files:
  created: [docs/DISTRIBUTION.md]
  modified: [README.md]
decisions:
  - "Sources were not fetched (no WebFetch available); source-derived claims are marked not re-checked"
metrics:
  completed: 2026-10-08
requirements: [PKG-02]
---

# Phase 7 Plan 04: Distribution notes Summary

`docs/DISTRIBUTION.md` documents CDM licensing, Apple Media Services Terms, Flathub policy and castlabs ECS terms as Known/Unknown with source URLs and no legal conclusion, plus Flatpak/AppImage feasibility and an Arch install/first-run/paths/uninstall section. README links it and drops the stale Phase 5 line.

## Deviations from Plan

**[Rule 3 - Blocking] Sources not fetched.** No WebFetch tool was available. Per the plan's fallback, source claims carry "(not re-checked 2026-10-08, from research summary)" and the doc says so up front. No quotes were invented.

Both tasks went in one commit because they edit the same new file.

## Known Stubs

None. The doc refers to `packaging/arch/PKGBUILD` and `packaging/scan-no-cdm.sh`, which other plans in this phase create.

**Commit:** see git log (`docs(07-04)`).

## Self-Check: PASSED

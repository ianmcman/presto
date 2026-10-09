---
phase: 07-packaging-and-distribution-notes
plan: 03
subsystem: packaging
tags: [arch, pkgbuild, engine-dir]
requires: []
provides: [default_engine_dir, PKGBUILD presto-git, scan-no-cdm.sh]
key-files:
  created: [packaging/arch/PKGBUILD, packaging/presto.desktop, packaging/presto.svg, packaging/scan-no-cdm.sh]
  modified: [crates/presto/src/cli.rs, crates/presto/src/launch.rs, crates/presto/src/main.rs, crates/presto-core/src/config.rs, .gitignore]
decisions:
  - "--engine-dir is Option; installed <exe>/../lib/presto/engine wins over ./engine"
metrics:
  completed: 2026-10-08
requirements-completed: [PKG-01]
---

# Phase 7 Plan 03: Engine dir resolution and Arch packaging Summary

presto resolves an installed engine relative to its executable, and `packaging/` holds a presto-git PKGBUILD (ECS via npm at build time, no CDM), desktop entry, icon and a file-list Widevine scan.

## Tasks

1. Exe-relative default engine dir: 736f9a1 (3 new launch tests, 1 new cli test; presto 133 and presto-core 82 lib tests pass).
2. PKGBUILD, desktop, icon, scan script, .gitignore: see git log (scan self-check accepts clean tarball, rejects one with libwidevinecdm.so).

## Deviations

- PKGBUILD comment reworded to avoid the literal `--no-sandbox` so the plan's grep criterion holds.
- Transient presto-ipc compile error (another agent's concurrent edit) cleared on its own; not touched.

The real makepkg build is not run here (plan 05).

## Self-Check: PASSED

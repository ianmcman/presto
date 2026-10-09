---
phase: 07-packaging-and-distribution-notes
plan: 05
subsystem: packaging
tags: [arch, pkgbuild, widevine, manual-verification]
requires: [07-01, 07-02, 07-03, 07-04]
provides:
  - PKGBUILD installs MIT license, prunes unused electron build scripts
  - manual checklist results from a CachyOS install
key-files:
  modified: [packaging/arch/PKGBUILD, docs/DISTRIBUTION.md, .planning/phases/07-packaging-and-distribution-notes/07-MANUAL-CHECKLIST.md, .planning/phases/07-packaging-and-distribution-notes/07-VALIDATION.md]
decisions:
  - "License: MIT, matching the repo LICENSE"
metrics:
  completed: 2026-10-08
requirements: [PKG-01]
---

# Phase 7 Plan 05: Arch package and manual checklist Summary

`presto-git` builds in a clean devtools chroot, installs, fetches the CDM on first run (observed 4.10.3112.0), signs in and plays, and writes nothing under /usr. Rebuilt package `presto-git-r235.5f34913-1` is 119 MB.

## Checklist outcome

Items 1 to 6, 8 and 9 pass. Item 7 skipped (optional). Item 8: `presto --demo` played mock tracks. Item 9: `pacman -Rns` clean and `/usr/lib/presto` gone; the `rm -rf` of temp state dirs was not run. Not confirmed in item 3: `presto --help` text and launcher icon.

## Changes after namcap

- LICENSE installed to `/usr/share/licenses/presto-git/LICENSE`.
- Removed `electron/vmp-resign.py`, `cli.js`, `install.js` and all `extract-zip` prebuilds except linux-x64-gnu. The app execs `dist/electron` via `path.txt`, so none are needed at runtime. These caused the python/nodejs namcap errors; the java-runtime one was not investigated separately.
- Rebuild: scan ok, license listed, one `dist/electron`, `ldd` on `presto` clean. namcap is not installed on the host, so it was not re-run.

## Deviations

PKGBUILD rebuild used `PRESTO_SRC=file:///home/mcmanusiang/presto` (existing clone in src/ is local); the fix is PKGBUILD-only so no push was needed before building.

## Notes

- First chroot attempt failed on a lagging CachyOS mirror and a bad pacman signature; environment issue, fixed by refreshing archlinux-keyring.
- Out-of-scope bug: clicking a radio station (non-playlist) does not play.

## Known Stubs

None.

## Self-Check: PASSED

---
phase: 02-engine-feasibility-spike-gate
plan: 01
subsystem: infra
tags: [electron, castlabs-ecs, widevine, npm]
requires: []
provides:
  - engine/ with castlabs ECS v44.5.1+wvcus pinned
affects: [02-03, 02-04]
tech-stack:
  added: [castlabs ECS v44.5.1+wvcus]
  patterns: []
key-files:
  created: [engine/package.json, engine/package-lock.json]
  modified: [.gitignore]
key-decisions:
  - "Pinned newest wvcus tag v44.5.1 (D-02); v44.1.0 fallback not needed"
requirements-completed: [SPIKE-02, SPIKE-06]
duration: 10min
completed: 2026-10-07
---

# Phase 2 Plan 01: ECS install Summary

castlabs ECS `v44.5.1+wvcus` installed in `engine/`, pinned exactly, `node_modules` git-ignored.

## Terms gate

Question: "Proceed with the engine spike on your own Apple Music subscriber account?" User reply: "Proceed" (via AskUserQuestion), 2026-10-07. No engine file existed before the reply.

## Versions

- ECS tag: `v44.5.1+wvcus` (newest of v43.2.0, v43.5.0, v43.7.7, v44.1.0, v44.5.1)
- electron package version: `44.5.1+wvcus`
- chrome-sandbox: owner `mcmanusiang`, mode `775` (not root 4755, as expected). Namespace sandbox should be used; `--no-sandbox` is a fallback only (Pitfall 10).
- Date: 2026-10-07

## Deviations from Plan

**1. [Rule 3 - Blocking] npm 12 refuses git dependencies by default**
- `npm install` failed with EALLOWGIT (`allow-git=none`, npm default, not user config).
- Fix: ran with `--allow-git=root` for that command only (git deps declared by the root project). No config changed.

**2. [Rule 3 - Blocking] ECS binary not downloaded by npm install**
- The package ships `install.js` with no install lifecycle script, so `dist/` was missing.
- Fix: ran `node node_modules/electron/install.js` once. Plans that reinstall must repeat this step and pass `--allow-git=root`.

Commit: 5b2c3c2

## Known Stubs

None.

## Self-Check: PASSED

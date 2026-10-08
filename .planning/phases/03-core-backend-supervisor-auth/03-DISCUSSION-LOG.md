# Phase 3: Core Backend, Supervisor, Auth - Discussion Log

> **Audit trail only.** Decisions are in 03-CONTEXT.md.

**Date:** 2026-10-08
**Areas discussed:** Recovery behavior, Sign-in window UX, Re-auth and expired session, Bridge loading and drift

All questions: the recommended option was chosen.

## Recovery behavior
- Resume after restart: resume if it was playing (alt: always paused, always playing)
- Backoff: 1s doubling cap 30s, give up after 5 fast failures (alt: retry forever)
- Stale engine: kill orphan silently via pidfile (alt: ask, refuse)
- Poison queue: restore paused after 2nd consecutive crash (alt: drop queue, none)

## Sign-in window UX
- Appears immediately with Presto prompt (alt: after click)
- Close: hide, Sign in button to reopen (alt: reopen, quit)
- Later launches: never shown unless signed_out/re-auth (alt: menu item)
- Signed-out UI: blocked by sign-in panel (alt: usable with errors)

## Re-auth and expired session
- Banner with button (alt: auto-open window)
- After re-auth: keep queue, resume per prior state (alt: leave paused)
- Requests while signed out: none, cache only (alt: send and fail)
- Detection: events only (alt: probe)

## Bridge loading and drift
- Path: config override then install dir (alt: install only)
- Handshake: version, capabilities, MusicKit build (alt: minimal)
- Drift: blocking error panel (alt: degrade silently)
- Ready gate: queue commands, 15s timeout (alt: fail fast)

## Claude's Discretion
Crate layout, queue revision reconciliation, profile 0700 mechanics, pidfile format.

## Deferred Ideas
None.

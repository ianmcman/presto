---
phase: 03-core-backend-supervisor-auth
verified: 2026-10-08T00:00:00Z
status: gaps_found
score: 4/5 success criteria verified
gaps:
  - truth: "After killing or hanging the engine, playback resumes at the same queue and position after a backoff restart"
    status: partial
    reason: "Live run on the real engine: kill -9 at 43s restored queue and index but restarted at 0s (1 of 4 runs). A paused session restores as playing."
    artifacts:
      - path: "crates/presto-core/src/supervisor.rs"
        issue: "begin_restore (line ~335) sends Seek once with no verify or retry; MusicKit may still be loading. Nothing stops the engine from auto-playing after Seek when was_playing is false."
    missing:
      - "After the restore seek, wait for playing/loading, re-send seek, verify position_ms near target (Pitfall 6)"
      - "Send Pause after Seek when not resuming (paused restore, and third consecutive crash per D-04)"
      - "Mock or integration test covering both cases"
---

# Phase 3 Verification

**Goal:** The app runs a supervised engine reliably and the user can sign in once and stay signed in.
**Status:** gaps_found. CORE-01 is not satisfied.

## Success criteria

| # | Criterion | Status | Evidence |
|---|---|---|---|
| 1 | Kill/hang resumes same queue and position | PARTIAL | Hang (SIGSTOP) detected by 3 missed pings, restart about 6s, resumed at 124s (live D). Crash restore lost the seek in 1 of 4 runs. Paused state restores as playing (also on third crash). Restore logic exists in `supervisor.rs`/`mirror.rs` but has no seek verification. |
| 2 | Bridge editable on disk, handshake shows version and capabilities | VERIFIED | Live F: `~/.config/presto/bridge.js` override gave `1.1.0-local` with no rebuild. Missing capability gives Drift, no restart loop. |
| 3 | Login shown on first launch, hides after sign-in, stays hidden | VERIFIED | Live A, per 03-06 results. |
| 4 | Expired session offers re-auth, cached data stays | VERIFIED (partial scope) | Live H: cookies deleted gives `signed_out`, window opens, commands fail fast with `auth_expired`. Queue survival across expiry not exercised; cached-data display is UI work in later phases. `tests/auth.rs` passes. |
| 5 | Mirror matches MusicKit queue, profile mode 0700 | VERIFIED | Live G (`player.track == queue.items[index]` after 4 rapid next) and B (engine-profile and state dir mode 700). |

## Requirements

| ID | Status | Evidence |
|---|---|---|
| CORE-01 | BLOCKED (partial) | Crash/hang detection and backoff restart work. "restores queue and position" fails in the two live cases above. |
| CORE-02 | SATISFIED | Live F, plans 03-03/03-04. |
| CORE-03 | SATISFIED | Live G, `tests/mirror.rs`. |
| AUTH-01 | SATISFIED | Live A. |
| AUTH-02 | SATISFIED | Live H, `tests/auth.rs`. Cached-data visibility is a UI concern not yet built. |
| AUTH-03 | SATISFIED | Live B, `paths.rs` tests. |

All six IDs appear in plan frontmatter (03-06 lists all) and in REQUIREMENTS.md. No orphans.

## Automated checks

`cargo test -p presto-core`: all suites pass (24 unit, plus drift, recover, auth, mirror, stale). The tests do not cover the two failing restore cases, so green results do not contradict the live gaps.

## Anti-patterns

No blockers found beyond the missing seek verification. Cosmetic: stale position/state shown during Starting.

## Bookkeeping

ROADMAP.md (Phase 3 `[x]`, completed 2026-10-08) and REQUIREMENTS.md (CORE-01 `[x]`, Complete) are premature. They should be reverted until the gap plan lands. Not edited by the verifier.

## Gaps

Single root cause area: restore sequencing in `Supervisor::begin_restore`. Fix with `/gsd:plan-phase 03 --gaps`:
1. Seek retry with position verification after load.
2. Pause after Seek when `was_playing` is false or `resume_allowed` is false.
3. Re-run live items C and D (3-4 crash iterations, plus paused and third-crash cases).

_Verifier: Claude (gsd-verifier)_

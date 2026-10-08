---
phase: 02-engine-feasibility-spike-gate
verified: 2026-10-07T00:00:00Z
status: passed
score: 5/5 roadmap criteria verified
---

# Phase 2: Engine Feasibility Spike Gate Verification Report

**Goal:** Prove a hidden Widevine-capable engine plays Apple Music catalog tracks, proxies library calls and streams events to Rust, with measurements and a user go/no-go.
**Status:** passed (verified from committed logs and code; no live run)

## Observable Truths

| # | Truth | Status | Evidence |
|---|---|---|---|
| 1 | Sign-in persists across engine restarts | VERIFIED | `checklist-wayland-hidden-*.md` and `checklist-after-sigkill-1791426562.md` show `session PASS signed_in` on a fresh engine |
| 2 | Rust command plays past 60 s and across a seek | VERIFIED | `checklist-wayland-hidden-2-1791425650.md`: reached 125000 ms, seek 120000; `listen2`: 155000 ms, seek 150000. Audible playback is the user's report (sink-inputs log shows uncorked `presto-engine` stream) |
| 3 | `/v1/me/library/playlists` returns JSON to Rust | VERIFIED | api row: 11 playlists, next=false |
| 4 | Playback events stream to Rust | VERIFIED | events row: playback_state 7, progress 136, track_changed 1, queue_changed 2 |
| 5 | Report with RSS, hidden window (Wayland/X11), codec, queue API, MPRIS, comparison, recommendation, user decision | VERIFIED | `SPIKE-REPORT.md` has every section; `logs/matrix.md` has 8 rows; `rss-summary.txt`, `codec.txt`, `diag-ready.json`, mpris-*.txt exist; Approval section records "Decision: go" |

**Score:** 5/5

## Artifacts

| Artifact | Status | Details |
|---|---|---|
| `engine/{main,bridge,guard,preload}.js` + tests | VERIFIED | bridge 185 lines, main 186 lines (both over the 120 minimum); `node_modules` is gitignored and untracked |
| `crates/presto-spike/src/{session,checks,rss,main}.rs`, `tests/mock.rs` | VERIFIED | present, substantive (222/411/76/157 lines) |
| `crates/presto-ipc`, `crates/presto-engine-mock` | VERIFIED | present |
| `logs/matrix.md`, `rss-*.csv`, `codec.txt`, `diag-ready.json` | VERIFIED | present |
| `SPIKE-REPORT.md` | VERIFIED | GO, ECS v44.5.1+wvcus, date, web player build recorded |

## Requirements Coverage

| ID | Source plans | Status | Evidence |
|---|---|---|---|
| SPIKE-01 | 02-03, 02-04 | SATISFIED | musickit check PASS, diag-ready.json |
| SPIKE-02 | 02-01, 02-04, 02-05 | SATISFIED | session PASS on fresh and post-SIGKILL engines |
| SPIKE-03 | 02-02, 02-03, 02-04, 02-05 | SATISFIED | playback row, seek past 60 s |
| SPIKE-04 | 02-02, 02-03, 02-04 | SATISFIED | api row |
| SPIKE-05 | 02-02, 02-03, 02-04, 02-05 | SATISFIED | events row |
| SPIKE-06 | 02-01, 02-02, 02-05, 02-06 | SATISFIED | SPIKE-REPORT.md, matrix, RSS |

All six IDs appear in plan frontmatter and in REQUIREMENTS.md (marked complete). No orphaned requirements.

## Secrets Check

No match in `logs/` for token, cookie, bearer, authorization, password, JWT-shaped strings. The only hit on "session_id" is `application.process.session_id` in PulseAudio sink dumps (benign).

## Anti-Patterns

None blocking.

## Caveats (documented in report, not gaps)

- Codec inferred as AAC from the playlist filename; no `CODECS=` string captured.
- X11 rows ran under XWayland only.
- Multi-item queues not exercised live.
- Orphan engine after SIGKILL and `--diag` SIGTRAP are carried to Phase 3.
- Report notes `idle_after_hello` and `signed_in_idle` RSS are single samples.
- Apple ToS unassessed (Phase 7).
- Audible playback and the user approval cannot be re-checked from files; they rest on the recorded checklists and the Approval section.

## Human Verification Required

None outstanding; the user go decision is already recorded in SPIKE-REPORT.md.

_Verifier: Claude (gsd-verifier)_

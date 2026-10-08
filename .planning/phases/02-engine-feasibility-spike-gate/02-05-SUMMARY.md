---
phase: 02-engine-feasibility-spike-gate
plan: 05
subsystem: engine
tags: [electron, ecs, widevine, wayland, x11, mpris, rss, codec]
requires: [02-04]
provides: [hidden-window matrix, RSS/PSS numbers, codec evidence, MusicKit API surface, SIGKILL result, listening verdict]
affects: [02-06]
key-files:
  created: [.planning/phases/02-engine-feasibility-spike-gate/logs/]
  modified: [engine/main.js]
decisions:
  - "Default MediaSessionService stays disabled; it is the only source of a duplicate MPRIS name"
metrics:
  tasks: 3
  completed: 2026-10-07
---

# Phase 2 Plan 05: SPIKE-06 measurements Summary

All 8 hidden-window runs on Wayland and XWayland played past 90 s with the window hidden. The user heard continuous audio and a working seek.

## Matrix (logs/matrix.md)

X11 rows run under XWayland inside the KDE Wayland session; no native X11 session was tested. Each row: `check all`, 6/6 PASS, sink probed 25 s in.

| label | platform | window | flags | playback | corked | mpris |
|---|---|---|---|---|---|---|
| wl-hidden | wayland | hidden from launch | defaults | PASS (95000 ms) | no | none |
| wl-hidden-noautoplay | wayland | hidden from launch | --no-autoplay-switch | PASS (95000 ms) | no | none |
| wl-hidden-throttle | wayland | hidden from launch | --allow-throttling | PASS (95000 ms) | no | none |
| wl-shown-then-hidden | wayland | shown, hidden on signed_in | --show | PASS (95000 ms) | no | none |
| wl-hidden-mediasession | wayland | hidden from launch | --keep-media-session | PASS (95000 ms) | no | org.mpris.MediaPlayer2.chromium.instance830422 |
| x11-hidden | x11 (XWayland) | hidden from launch | defaults | PASS (95000 ms) | no | none |
| x11-hidden-noautoplay | x11 (XWayland) | hidden from launch | --no-autoplay-switch | PASS (95000 ms) | no | none |
| x11-shown-then-hidden | x11 (XWayland) | shown, hidden on signed_in | --show | PASS (95000 ms) | no | none |

The autoplay and throttling switches did not change the outcome (the driver starts playback through MusicKit after sign-in), so they stay as defaults. A duplicate `org.mpris.MediaPlayer2.chromium.*` name appears only with `--keep-media-session`.

## RSS / PSS (logs/rss-summary.txt, 300 s playing, whole process tree, MiB)

| phase | RSS min | RSS max | PSS min | PSS max |
|---|---|---|---|---|
| idle_after_hello | 421.7 | 421.7 | 173.7 | 173.7 |
| signed_in_idle | 1489.1 | 1489.1 | 935.8 | 935.8 |
| playing | 1593.5 | 1775.8 | 880.2 | 1068.5 |

## Codec (logs/codec.txt)

The song playlist is a media playlist with no master, so no `CODECS=` line exists. Seen: `.rphq.aac.wa.m3u8` (application/x-mpegurl), fMP4, `EXT-X-KEY METHOD=ISO-23001-7` (CENC). Inference: AAC, encrypted. The exact CODECS string is unconfirmed.

## MusicKit surface (logs/diag-ready.json)

MusicKit `playback/AppleMusic-2640.11.0-external`, web build `index~c10ba4a68d.js`, 25 mk members, 40 queue members, PlaybackStates 0-10 captured, `has_media_session` true, Chrome/152 UA.

## SIGKILL

The new engine on the same profile reported `| session | PASS |` signed_in. Sign-in survives SIGKILL. The original claim that all children exited with the main process was wrong; see the orphan finding.

## Listening verdict

User approved after the rerun: continuous audio, seek worked, no window.

## Finding: orphaned engine and SIGTRAP

After the SIGKILL test an orphaned engine (pid 835551, ppid 1) kept holding the profile and socket. The first listen run (`listen`) then crashed with SIGTRAP at playback start (playback/events FAIL "Connection reset by peer"; coredumpctl: electron SIGTRAP at 21:36, 21:37, 21:41). The orchestrator killed the orphan and `listen2` passed 6/6 (reached 155000 ms, seek 150000, 156 progress, 7 playback_state). The correlation is unconfirmed as the cause. The Phase 3 supervisor must detect and clean a stale engine or profile lock, and reap children after the main process dies. matrix.md was corrected. Both listen runs are kept as evidence.

## Deviations from Plan

- [Rule 3] `--diag` run hit a SIGTRAP; handled by rerun (see codec log).
- [Rule 3] `check playback` ran before the bridge was ready in one attempt; used `check all` for the codec run.
- [Rule 1] Matrix SIGKILL note said all children exited; corrected after the orphan was found.
- Task 3 needed a rerun (`listen2`) after the orphan was killed.
- Earlier `sigkill-victim` checklists are aborted attempts (kill pattern mismatch); only the last one and `after-sigkill` count.

## Commits

824c813 (matrix, SIGKILL), 8ea2553 (RSS, codec, API surface), d0acea1 (listen logs, orphan note).

## Known Stubs

None.

## Self-Check: PASSED

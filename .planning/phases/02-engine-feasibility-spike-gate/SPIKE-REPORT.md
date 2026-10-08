# Phase 2 Spike Report: hidden Chromium engine for Apple Music

## Verdict

GO, with castlabs ECS `v44.5.1+wvcus`. All five success criteria are met on the test machine (KDE Plasma Wayland, own subscriber account). Caveats: X11 was tested only under XWayland, the stream codec is inferred and not confirmed, and the engine left an orphan process after SIGKILL that Phase 3 must handle. Chrome via CDP (D-15) was not needed because ECS had no failures.

## Environment

- Date: 2026-10-07
- Kernel: 7.2.8-1-cachyos
- Desktop: KDE Plasma Wayland. X11 rows ran under XWayland (DISPLAY=:0); no native X11 session was tested. ECS runs were Wayland-only for the Wayland rows.
- ECS tag: `v44.5.1+wvcus` (Electron 44.5.1; web player UA reports Chrome/152). See `02-01-SUMMARY.md`.
- Web player build: `index~c10ba4a68d.js`, MusicKit `playback/AppleMusic-2640.11.0-external` (`logs/diag-ready.json`)
- Account: the user's own subscriber account (D-09). Terms gate answered "Proceed" on 2026-10-07 before any engine file existed.
- Sandbox: `--no-sandbox` not needed (`logs/checklist-smoke-1791425043.md`, `logs/checklist-smoke-hardened-1791425159.md`). chrome-sandbox is mode 775, not setuid root; the namespace sandbox worked.
- Widevine CDM 4.10.3112.0 was downloaded on first launch via the component updater (`logs/engine-smoke-cdm-1791425079.log`).

## Success Criteria

| # | Criterion | Result | Evidence |
|---|---|---|---|
| 1 | Sign in via Apple's login page, restart, still signed in | Met. Hand sign-in worked; next hidden engine started signed in. Also survived SIGKILL. | `logs/engine-signin-1791425168.log`, `logs/checklist-wayland-hidden-1791425548.md`, `logs/checklist-after-sigkill-1791426562.md` |
| 2 | Rust command plays a full catalog track past 60 s and across a seek | Met. Bohemian Rhapsody (355000 ms), reached 125000 ms, seek to 120000. Later runs reached 95000 to 155000 ms. User heard continuous audio and a working seek. | `logs/checklist-wayland-hidden-2-1791425650.md`, `logs/checklist-listen2-1791427458.md`, `logs/sink-inputs-wayland-hidden-2.txt` |
| 3 | Rust calls `/v1/me/library/playlists` via the page, no token visible | Met. 11 playlists returned. No file in `logs/` contains "token". | `logs/checklist-wayland-hidden-2-1791425650.md` (api row) |
| 4 | Play, pause and progress events reach Rust during playback | Met. auth 1, playback_state 7, progress 136, queue_changed 2, track_changed 1, volume 1. | `logs/checklist-wayland-hidden-2-1791425650.md`, `logs/events-wayland-hidden-2-1791425580.ndjson` |
| 5 | Written report with RSS, hidden window, codec, queue API, MPRIS, comparison, recommendation | This report | this file |

## Memory (RSS)

Method: sum over the engine process tree from `/proc/<pid>/smaps_rollup`, every 5 s, 300 s of playback. PSS avoids double-counting shared pages. No pass/fail threshold (D-14). Source: `logs/rss-summary.txt`, `logs/rss-measure-wayland-1791426580.csv`.

| phase | RSS min | RSS max | RSS last | PSS min | PSS max | PSS last |
|---|---|---|---|---|---|---|
| idle_after_hello (MiB) | 421.7 | 421.7 | 421.7 | 173.7 | 173.7 | 173.7 |
| signed_in_idle (MiB) | 1489.1 | 1489.1 | 1489.1 | 935.8 | 935.8 | 935.8 |
| playing (MiB) | 1593.5 | 1775.8 | 1605.3 | 880.2 | 1068.5 | 892.0 |

idle_after_hello and signed_in_idle are single samples; only playing has a time series. What idle_after_hello covers (engine before the web player loads) is an assumption from the label.

## Hidden Window

Source: `logs/matrix.md`. Each row ran `check all` (6/6 PASS) and was probed 25 s in.

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

- Playback continues with the window hidden on Wayland and on X11 under XWayland. The sink input is `presto-engine`, not corked, in every run.
- The autoplay and throttling switches did not change outcomes because the driver starts playback through MusicKit after sign-in. They stay as defaults (no user-gesture autoplay, no renderer backgrounding or timer throttling). Whether they matter for other start paths is not measured.
- Sign-in with a hidden window: the engine window is shown only while the session is signed_out and hides on `auth signed_in`. This ran with `--show` on first sign-in (`logs/engine-signin-1791425168.log`).
- SIGKILL: after killing the main process 20 s into playback, a new engine on the same profile was signed in (`logs/checklist-after-sigkill-1791426562.md`). Children did not all exit; see Risks.

## Stream Codec

Source: `logs/codec.txt`. The track playlist is `...rphq.aac.wa.m3u8` (`application/x-mpegurl`), an HLS media playlist with fMP4 (`EXT-X-MAP`) and `EXT-X-KEY METHOD=ISO-23001-7` (CENC). It is a media playlist with no master, so no `CODECS=` line exists. AAC is inferred from the filename; the exact CODECS string is unconfirmed. No `Network` type Media lines were captured.

## Queue API Surface

Source: `logs/diag-ready.json`.

- `mk_members` (25): play, pause, seekToTime, skipToNextItem, skipToPreviousItem, playMediaItem, setStationQueue, setQueue, playNext, playLater, playAt, updateQueue, savePlaybackState, restorePlaybackState, shuffleMode, repeatMode, autoplayEnabled, volume, plus SharePlay members. `changeToMediaAtIndex` is not in the listed own members (the bridge calls it after `setQueue` when the start index is above 0; it was not exercised live).
- `queue_members` (40): items, position, currentItem, length, append, splice, clear, clearAfterCurrent, remove, userAddedItems, autoplayItems, nextPlayableItem, previousPlayableItem, and others.
- PlaybackStates: 0 none, 1 loading, 2 playing, 3 paused, 4 stopped, 5 ended, 6 seeking, 8 waiting, 9 stalled, 10 completed (7 is not in the captured enum; the `playback_state 7` in the checklist is an event count).
- Set_queue shape: the checklist playback check started a track by song id; the bridge `set_queue` mapping takes `{songs, startPlaying}`. The song-id playback path passed. Multi-item queues were not exercised live.
- CORE-03 implication: MusicKit owns the queue and exposes position, items and mutation methods, so Rust can mirror by `queue_changed` revision. The bridge emits a revision counter.

## API Proxy

- Path form that worked: GET `mk.api.music(path, query)` with the library playlists path; the bridge replies `res.data ?? res`. Result: 11 playlists, no `next`. `api_music_arity` is 3.
- Error mapping (implemented and unit tested, not triggered live): 401/403 auth_expired, 429 rate_limited, 404 not_found, other upstream, none internal.
- The bridge never reads token properties, `diag()` drops member names matching /token/i, and a test asserts no emitted payload matches /token/i. A grep of `logs/` for "token" is empty.

## MPRIS

Source: `logs/mpris-baseline.txt` (empty), `logs/mpris-wl-hidden.txt` and other default rows (empty), `logs/mpris-wl-hidden-mediasession.txt`.

- Baseline with no engine: no MPRIS names.
- Default engine: no MPRIS name. Default switches include `disable-features=MediaSessionService,HardwareMediaKeyHandling`.
- `--keep-media-session`: `org.mpris.MediaPlayer2.chromium.instance830422` appears. The default disable switch therefore removes the duplicate, in the Wayland and XWayland rows measured.
- DESK-01 input: Presto registers its own MPRIS name; the engine must keep MediaSessionService disabled. `has_media_session` is true in the page, so the page still uses the API; only Chromium's bus export is off.

## Candidate Comparison

| Candidate | Widevine source | Hidden window | Measured? | Effort | Distribution notes | Verdict |
|---|---|---|---|---|---|---|
| castlabs ECS | CDM fetched by component updater on first launch (4.10.3112.0 seen) | Works on Wayland and XWayland | Yes | Low; engine exists | Flatpak can ship ECS; AUR/AppImage fetch the ECS binary and CDM at runtime. Linux needs no VMP signing | Recommended |
| System Chrome via CDP | Chrome's own CDM | Desk: no truly hidden headful window on Wayland | No (D-15 not triggered) | Medium | Best licensing story; user browser version drift | Fallback only |
| CEF via cef-rs | None bundled; supply `libwidevinecdm.so`, load in zygote | Desk | No | High; ~300 MB dist, CDM/version coupling | Own the CDM sourcing | Deferred |
| WebKitGTK/wry | None | n/a | No | n/a | n/a | Rejected |

Desk rows come from CLAUDE.md, not from this spike.

## Risks and Phase 3 Inputs

- Stale engine or profile lock: after the SIGKILL test an orphaned engine (pid 835551, ppid 1) kept holding the profile and socket for about 12 minutes. The next run (`logs/checklist-listen-1791427310.md`) crashed with SIGTRAP at playback start (playback/events FAIL "Connection reset by peer"; coredumpctl shows electron SIGTRAP at 21:36, 21:37, 21:41). After the orphan was killed, `listen2` passed 6/6. The correlation is unconfirmed. Phase 3 supervisor must detect and clean a stale engine or profile lock and reap children after the main process dies.
- `--diag` SIGTRAP: with `--diag` (CDP Network attached) the main process died with SIGTRAP shortly after playback began (`logs/codec.txt`). Runs without `--diag` are stable. Phase 3 should not rely on `--diag` for production, and the cause is not investigated.
- `check playback` run alone failed once because the bridge was not ready; it needs the bridge ready first. Phase 3 handshake must gate on bridge-ready.
- Engine origin allowlist: `engine/guard.js` denies navigation, windows and permissions outside the allowed origins. No denials were logged during sign-in or playback. Phase 3 keeps this and should log denials.
- npm 12 refuses git dependencies by default; install needs `--allow-git=root`, then `node node_modules/electron/install.js` once because the ECS binary is not fetched by `npm install` (`02-01-SUMMARY.md`). Packaging and CI must repeat both.
- CDM downloaded at first run: first launch needs network and time before playback works. Phase 7 packaging note: never ship the CDM; document the first-run download.
- Web player drift (D-04): the bridge depends on the `MusicKit` global and `mk.api.music`. No resilience was built in the spike.
- Autoplay: no retry was needed with `autoplay-policy=no-user-gesture-required`.
- ToS (Phase 7 PKG-02): Apple Music ToS stance on automating the web player is not assessed. The user proceeded on their own account only.
- EPIPE hardening (`send` guard, `uncaughtException` quit) in `engine/main.js` should carry into Phase 3.
- Memory is about 1.5 GiB RSS (about 0.9 to 1.1 GiB PSS) signed in. The judgment is the user's (D-14).

## Approval

Pending user approval.

# Hidden window and MPRIS matrix
Date: 2026-10-07. X11 rows run under XWayland (DISPLAY=:0) inside the KDE Wayland session; no native X11 session was tested.
Each row: `check all --song 6781024437 --min-play-secs 30 --seek-secs 90`, all six checks PASS. Sink and MPRIS probes taken 25 s in. MPRIS baseline (no engine) is empty (mpris-baseline.txt).

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

Notes:
- The sink entry is `application.name = presto-engine`, `Corked: no` in every run.
- The engine registers no MPRIS name by default; MediaSessionService must be left enabled (`--keep-media-session`) to get a duplicate `org.mpris.MediaPlayer2.chromium.*` name, so the default disable switch removes the duplicate.
- Playback also passes with `--no-autoplay-switch` and `--allow-throttling` when the window is hidden. The driver starts playback through MusicKit after the profile is signed in, so these switches did not change the outcome in this setup; they stay as defaults.

## SIGKILL
Main engine process killed with SIGKILL 20 s into playback (all child processes exited with it). A new engine on the same profile (`after-sigkill`, checklist-after-sigkill-*.md) reported `| session | PASS |` signed_in. Sign-in survives SIGKILL.
Earlier `sigkill-victim` checklists in this directory are from aborted attempts where the kill pattern did not match; only the last `sigkill-victim` run and `after-sigkill` count.

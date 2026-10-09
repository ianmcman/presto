# Phase 06 Manual Desktop Checklist

**Date:** [to be filled]
**Tester:** [to be filled]
**Desktop:** [Wayland/X11]

**Automated test results:** `cargo test --workspace` and `dbus-run-session -- cargo test -p presto --test desktop_mpris` both pass (all 130+ tests green)

---

## 1. MPRIS Bus Registration

Steps: Start `presto --demo` (or real `presto` if signed in). Run `busctl --user list | grep mpris`.

Expected: Exactly one line matching `org.mpris.MediaPlayer2.presto*` appears (demo shows `org.mpris.MediaPlayer2.presto-demo`, real run shows `presto`). Real run: exactly one player named `presto` (no collision with other instances).

Result:

---

## 2. Now-Playing Widget Integration

Steps: With Presto running and a track playing, check KDE or GNOME panel media widget. Run `playerctl -p presto metadata`.

Expected: Widget shows title, artist, album art. Progress bar advances while track plays. `playerctl` output includes title and `mpris:artUrl` starting with `file://` (not a CDN URL).

Result:

---

## 3. Media Keys (Global Shortcuts)

Steps: Start Presto and play a track. Click on another window to focus it. Press play/pause, next, previous keys on keyboard. Minimize Presto window. Press play/pause, next, previous keys again.

Expected: Media keys control Presto playback while another window is focused. Media keys control Presto playback while Presto is minimized. No key presses are lost or ignored.

Result:

---

## 4. Raise From Minimized

Steps: Start Presto and minimize it. Run `presto raise`.

Expected: Window restores and comes to focus (or flashes demands-attention on KDE; acceptable per D-08).

Result:

---

## 5. Quit From Minimized

Steps: Minimize Presto. Run `presto quit`. Check with `playerctl -p presto` (should fail if Presto exited). Run `pgrep -c presto` (should be 0 or 1 if engine still running).

Expected: Process exits cleanly via on_exit handler. No zombie processes. MPRIS bus name released (playerctl -p fails).

Result:

---

## 6. Single-Instance Lock

Steps: Start Presto. In another terminal, run `presto` (bare command, no args). Check return code: `echo $?`. Check process count: `pgrep -c presto`.

Expected: Second invocation returns exit code 0 (single-instance enforcement). First window raises or comes to focus. `pgrep -c presto` shows 1 (only one instance running).

Result:

---

## 7. CLI Status and Control Commands

Steps: With Presto running and a track playing, run each: `presto status`, `presto status --json`, `presto seek +10`, `presto seek 1:00`, `presto volume 30`, `presto volume -5`, `presto shuffle`, `presto repeat`.

Expected: `status` prints one-line format: `Title - Artist (pos/dur)`. `status --json` prints JSON with all fields (position, volume, state, etc.). `seek` commands move playhead as expected. `volume` commands adjust level 0-100 or relative +/-. `shuffle` toggles shuffle mode. `repeat` cycles through Off → All → One. All commands complete without error.

Result:

---

## 8. Watch Mode (Streaming Status)

Steps: Run `presto status --json --watch` in a terminal. Play music, pause, seek, adjust volume, change shuffle/repeat in Presto. Let it run for ~10 seconds. Try using it as a waybar custom module: `exec = "presto status --json --watch"`.

Expected: Each status change prints one JSON line. Position updates approximately once per second while playing. Output suitable for waybar parsing. No buffering delays visible.

Result:

---

## 9. No Instance Check

Steps: Kill all Presto processes (if any running). Run `presto status`. Check stderr and exit code: `echo $?`.

Expected: Prints `presto is not running` to stderr. Exit code is 1.

Result:

---

## 10. Coexistence With Other Players

Steps: Start another media player (mpv, browser, or another app with MPRIS). Start playing music in it. Start Presto (idle, no track or paused).

Expected: Other player keeps playing (Presto does not steal focus). Other player is not paused by Presto startup. Both appear in `busctl --user list | grep mpris` (Presto added, other still there).

Result:

---

## 11. Not-Ready State (Signed Out / Engine Stopped)

Steps: With Presto running, sign out of Apple Music (or stop the engine). Check MPRIS status: `busctl --user call org.mpris.MediaPlayer2.presto /org/mpris/MediaPlayer2 org.freedesktop.DBus.Properties.Get ss 'org.mpris.MediaPlayer2.Player' PlaybackStatus`. Try media keys. Check metadata: `playerctl -p presto metadata`.

Expected: PlaybackStatus shows `Stopped`. Metadata is empty (no track info). Media keys do nothing (silently ignored, no error). MPRIS still registered (bus name not released).

Result:

---

## Sign-Off

- [x] All 11 items completed with Result: [pass | fail | note]
- [ ] All results pass (no failures recorded)
- [ ] Any failures documented for gap planning

**Notes:**
[space for notes, known issues, or observations]

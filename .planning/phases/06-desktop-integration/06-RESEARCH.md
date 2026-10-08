# Phase 6: Desktop Integration - Research

**Researched:** 2026-10-08
**Domain:** MPRIS server, Unix-socket control channel, clap subcommands in an eframe app
**Confidence:** MEDIUM-HIGH (read the fastframe crate source and Presto code directly; nothing compiled or run yet)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** The CLI talks to the running app over its own control socket at `$XDG_RUNTIME_DIR/presto/ctl.sock`, NDJSON, same style as the engine IPC. Not MPRIS/D-Bus.
- **D-02:** With no running instance, every CLI command prints "presto is not running" and exits 1. No auto-launch.
- **D-03:** Plain `presto` with an instance already running raises/focuses the existing window via the control socket and the second process exits. One instance per profile (the engine profile lock forbids two).
- **D-04:** Clap subcommands; launching the GUI is the default. Existing `--demo`, `--fault`, `--engine-dir` stay on the GUI path. No separate `prestoctl` binary.
- **D-05:** Closing the window quits the app (current Phase 5 `on_exit` shutdown). Media keys and MPRIS work while the process lives, including minimized.
- **D-06:** Hide-on-close and a tray icon are deferred to a later phase (TRAY-01). Do not add them here.
- **D-07:** Media keys are MPRIS only. No global-shortcuts portal. Phase 5 in-window keyboard map is unchanged.
- **D-08:** Presto is a plain MPRIS player: accurate PlaybackStatus, no stealing focus or pausing other players.
- **D-09:** Full player: Metadata (trackid, title, artist, album, length, artUrl), PlaybackStatus, Position, Rate fixed 1.0, Volume, Shuffle, LoopStatus, CanSeek/Seek/SetPosition, Play/Pause/PlayPause/Stop/Next/Previous, root Raise/Quit, Identity "Presto".
- **D-10:** `mpris:artUrl` is a `file://` path into the Phase 4 artwork cache. The shell never fetches Apple's CDN.
- **D-11:** The bus name is owned from startup. When signed out, loading or the engine is restarting: PlaybackStatus Stopped, empty metadata, CanPlay false, control methods are no-ops. `busctl --user list | grep mpris` shows exactly one player.
- **D-12:** The engine's own MediaSession stays disabled (Phase 2 default switches); do not re-enable.
- **D-13:** Subcommands: `play`, `pause`, `toggle`, `next`, `prev`, `stop`, `seek <time|+/-secs>`, `volume <0-100|+/-n>`, `shuffle [on|off]`, `repeat [off|all|one]`, `status`, `raise`, `quit`. No queue editing, search or play-by-id.
- **D-14:** `status --json` prints stable keys: `state` (playing/paused/stopped/loading), `position_ms`, `duration_ms`, `volume`, `shuffle`, `repeat`, `track {id,title,artist,album,artwork_path}`, `auth`, `engine` (ready/starting/restarting). Carries a `proto` version.
- **D-15:** Plain `status` prints one line, e.g. `Playing: Title - Artist (1:12/3:40)`, or `Stopped`.
- **D-16:** `status --json --watch` streams NDJSON, one line per change, via a subscribe op on the control socket.

### Claude's Discretion
- MPRIS crate choice: `fastframe-now-playing` vs `mpris-server` + `zbus`. Evaluate fit with the eframe event loop and Rust-owned queue mirror.
- Control socket frame schema, exit codes beyond 0/1, time/volume argument parsing details, stale socket cleanup.

### Deferred Ideas (OUT OF SCOPE)
- Tray icon with hide-on-close (TRAY-01).
- Global shortcuts portal for media keys.
- Queue/search/play-by-id CLI commands.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DESK-01 | MPRIS exposes now-playing and controls from engine events; media keys work; engine MediaSession disabled | `fastframe-now-playing` v0.4.1 (wraps mpris-server 0.10); state mapping table; engine switch already verified in SPIKE-REPORT |
| DESK-02 | CLI controls playback and prints `status --json` | Own `ctl.sock` server on the Backend runtime, sync std-only client, clap subcommands, flock single-instance |
</phase_requirements>

## Summary

Use `fastframe-now-playing` (already pinned by tag v0.4.1 in the workspace family, resolves per SPOTIFAST-SEAMS). Source read: it runs mpris-server on its own thread with a private current-thread runtime, so it does not touch eframe's loop or Presto's tokio runtime. API is three calls: `NowPlaying::start(App, wake)`, `commands()` (poll), `update(State)` (cheap, self-throttled, position at most 1/s). It already builds a bus name `org.mpris.MediaPlayer2.<bus_name>` (no `.instance<pid>` suffix), handles Raise/Quit/Seek/SetPosition/Volume/Shuffle/Loop, file:// URL escaping for `art_file`, and logs a warning and runs without MPRIS if the session bus is missing. Going to raw `mpris-server` + `zbus` would re-write that for no gain.

Drive the integration from a tokio task on the Backend runtime, not from egui's `logic()`. Reason: on Wayland a minimized or occluded eframe window may get no frames, so a `logic()`-driven pump would stall media keys exactly when D-05 requires them. The task waits on `core.state()` changes, a `Notify` set by the `wake` closure, and a 1 s interval while playing. Commands go straight to `Backend::command`-style sends (same `core.command` path); only Raise and Quit need the egui `Context` (clone passed in; `send_viewport_cmd` is callable from any thread).

For D-01 do not use `fastframe-instance`: it is request/reply only (no streaming for `--watch`) and creates its own `instance.sock`. Single-instance = `flock` on `$XDG_RUNTIME_DIR/presto/ctl.lock` (nix 0.31 `Flock`, already a workspace dep with the `fs` feature) plus the NDJSON `ctl.sock` served by tokio `UnixListener`. The lock holder may unlink and rebind `ctl.sock` freely, which solves stale-socket cleanup.

**Primary recommendation:** `fastframe-now-playing` + a tokio "desktop" task in `presto` that owns it; hand-written ~150-line NDJSON control server with flock single-instance; CLI client is plain `std::os::unix::net::UnixStream` (no tokio).

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| fastframe-now-playing | git tag v0.4.1 (`crmne/fastframe`) | MPRIS service + command queue | Spotifast's own; resolves (SPOTIFAST-SEAMS); already in `~/.cargo/git/checkouts`; pulls mpris-server 0.10 + zbus 5 (zbus already in Cargo.lock via accesskit) |
| clap | 4 (workspace) | subcommands | existing `Cli` |
| tokio | 1 (workspace) | control server (`net`, `io-util`, `sync`, `time`) | add `net`, `io-util` to presto crate features |
| nix | 0.31 (workspace) | `Flock` for single instance | already used in presto-core |
| serde / serde_json | 1 | ctl frames | existing |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| fastframe-now-playing | mpris-server + zbus direct | More code, same result. Only if the fastframe crate blocks something (see Open Questions: seek capability) |
| flock + ctl.sock | fastframe-instance | Second socket, no streaming |
| std client | tokio client | Runtime startup cost for a one-shot CLI; no benefit |

**Installation** (workspace `Cargo.toml` adds the line, presto crate uses `workspace = true`):
```toml
fastframe-now-playing = { git = "https://github.com/crmne/fastframe", tag = "v0.4.1" }
```
Verify with `cargo tree -p presto -i mpris-server` after adding. Add `[target.'cfg(target_os = "linux")']` gating is not needed yet (Linux-only phase); the crate itself is cfg-gated.

## Architecture Patterns

### Recommended structure
```
crates/presto/src/
├── cli.rs          # Cli + optional Sub enum (clap)
├── ctl/
│   ├── mod.rs      # paths, single-instance lock, shared frame types (or put types in presto-ipc)
│   ├── server.rs   # tokio UnixListener, ops, subscribe
│   └── client.rs   # sync std client: send op, print, exit code
├── desktop.rs      # NowPlaying owner task: state->MPRIS, MPRIS->commands
└── status.rs       # pure fns: CoreState -> StatusJson, -> one-line text, -> mpris State
```
Keep mapping logic in pure functions (`status.rs`) so it is unit-testable without a bus or a socket.

### Startup order (main.rs)
1. `Cli::parse()`. If subcommand is not GUI: run client, exit with code. No Backend, no eframe.
2. GUI path: take `ctl.lock` (flock, non-blocking). If held: connect `ctl.sock`, send `raise`, exit 0 (D-03). Do this before `Backend::start`, which would otherwise hit the engine profile lock.
3. `Backend::start`; bind `ctl.sock` (unlink first, we hold the lock; chmod 0600; dir already via `ensure_private_dir`); start `NowPlaying` (before or inside eframe creator; bus name is owned from here, satisfying D-11).
4. In the eframe creator, hand the `egui::Context` to the desktop task for Raise/Quit.
5. `on_exit`: `Backend::shutdown`, drop the desktop task (dropping `NowPlaying` releases the bus name), remove `ctl.sock`; lock drops with the process.

### Demo isolation
`launch::demo_config` already uses `engine-demo.sock` and a wiped `state/demo`. Mirror that: demo uses `ctl-demo.sock`, `ctl-demo.lock`, and MPRIS `bus_name` `presto-demo` (identity "Presto (Demo)"), so a demo run beside a real run does not violate "exactly one player" for the real one, and tests never collide with a user's session. CLI needs to target it: make `--demo` a clap `global = true` flag (works as `presto --demo status`), or honor `PRESTO_CTL_SOCKET`. Recommend `--demo` global; tests use it.

### State mapping (pure fn, table-test it)

| Source | Condition | MPRIS (`fastframe_now_playing::State`) | `status` |
|--------|-----------|------|------|
| engine != Ready, or auth not SignedIn | any | `Playback::Stopped`, `track: None`, `controls.play/pause/next/previous/seek = false`; commands ignored | `state: stopped`; `engine` and `auth` carry the reason |
| `PlayState::Playing` | | `Playing` | `playing` |
| `PlayState::Paused` | | `Paused` | `paused` |
| `PlayState::Loading` | track present | `Paused` (crate doc: paused "or still loading") | `loading` |
| `Stopped` / `Ended` | | `Stopped`, metadata kept off | `stopped` |
| `Drift` / `Failed` engine | | treat as not ready | `engine: "failed"` (extends D-14's three values; flag in plan) |

Track: `id` = `QueueItem.id`, `artists = vec![artist]`, `duration = duration_ms`, `art_file` from the artwork cache. Volume `Some(f64)`, shuffle `Some`, repeat `Some` (`One -> Repeat::Track`, `All -> Repeat::Playlist`).

### Command mapping (MPRIS and ctl share one fn)

| Request | Engine `Command` |
|---------|------------------|
| Play / Pause / PlayPause | `Play` / `Pause` / by current `PlayState` (Playing -> Pause, else Play) |
| Stop | `Pause` (engine has no Stop; see Open Questions) |
| Next / Previous | `Next` / `Prev` |
| SeekBy(ms) | `Seek{ms: clamp(pos+ms, 0, duration)}`; apply `MIN_SEEK_MS` rule from `playback.rs` (targets under 3 s never answered, STATE.md Phase 3) |
| SetPosition{track_id, position} | ignore if `track_id != current track id`, else `Seek` |
| SetVolume(f64) | `SetVolume{volume: f32}` |
| SetShuffle / SetRepeat | `SetShuffle` / `SetRepeat` |
| Raise / Quit | viewport commands (below) |

`pos` for relative seeks: use the same interpolated clock the UI uses (`self.clock`, `clock.observe(&player, now)` in `App::logic`), or extrapolate in the task from `position_ms` + elapsed since last change while Playing. Extract the clock into a reusable piece if it lives inside `App`.

### Artwork for `art_file`
`DataHandle::art().get(&expand(url, 600))` returns `ArtState::{Ready(path), Pending, Failed}` and needs a tokio context (task already has one). Pass `Ready(path)` as `art_file`; on `Pending` pass `None` and re-evaluate on the 1 s tick; a changed `art_file` counts as a track change in the crate's throttle so metadata is republished. Never set `art_url` (that would let a shell fetch the CDN, violates D-10).

### Raise and Quit
```rust
// from the desktop task; Context is Clone + Send + Sync
ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
ctx.request_repaint();
// Quit
ctx.send_viewport_cmd(egui::ViewportCommand::Close); // runs eframe on_exit
```

### Control socket protocol (NDJSON, mirror engine IPC)
Put the frame types where the token-guard and insta schema-snapshot tests already run (`presto-ipc`, new `ctl` module) so IPC-02 style checks cover `status`. Suggested:
```json
{"proto":1,"id":7,"op":"play"}
{"proto":1,"id":8,"op":"seek","ms":72000}            // absolute; client resolves "+10" using a prior status
{"proto":1,"id":9,"op":"seek_by","ms":10000}
{"proto":1,"id":10,"op":"volume","pct":50}           // or "volume_by"
{"proto":1,"id":11,"op":"status"}
{"proto":1,"id":12,"op":"subscribe"}
// replies
{"proto":1,"id":7,"ok":true}
{"proto":1,"id":11,"ok":true,"status":{...D-14...}}
{"proto":1,"id":7,"ok":false,"error":"engine not ready"}
```
Relative seek/volume resolved server-side (`seek_by`, `volume_by`) so the client needs no extra round trip. `subscribe` replies with the current status then one status line per change; the server uses a `watch::Receiver<CoreState>` and drops a line when the mapped status is unchanged (position changes: emit at most 1/s, otherwise a playing track floods waybar). Unknown `proto` -> error reply. Cap request line length (e.g. 4 KiB) and give each connection a read timeout so a stray client cannot hold the server.

Exit codes: 0 ok, 1 not running (D-02), 2 usage (clap default), 3 command rejected (engine not ready, nothing to control). `status` when running but nothing playing exits 0.

### CLI parse shapes
- `seek`: `72`, `1:12`, `+10`, `-10`, `1:12:00` (absolute seconds or m:ss/h:mm:ss; leading sign means relative). Pure `parse_time(&str) -> Seek::{Abs(ms), Rel(ms)}`.
- `volume`: `0-100` absolute, `+n`/`-n` relative, clamp. `shuffle` no arg = toggle? D-13 says `[on|off]`; no arg = toggle (document it). `repeat` no arg = cycle off->all->one.
- Negative args like `seek -10` need clap `allow_negative_numbers = true` or `allow_hyphen_values = true` on that arg (see Pitfalls).

### Anti-Patterns to Avoid
- Driving MPRIS from `App::logic`: stalls when the window is minimized or occluded on Wayland.
- Per-frame full `State` rebuild with an `art().get()` call: fine (cheap, crate throttles), but do it in the task, not each frame.
- Re-enabling the engine MediaSession or adding `HardwareMediaKeyHandling` (D-12).
- Exposing any token or Apple URL with credentials in `status` (existing rule; `artwork_path` is a local path only).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| MPRIS interfaces, property change signals, Seeked | zbus interface impls | `fastframe-now-playing` | Throttling, volume-hold for Plasma, track-id path escaping, file URL escaping already done |
| Single-instance exclusion | pidfile + kill(0) | `nix::fcntl::Flock` | Released by kernel on crash; pidfiles go stale |
| Arg parsing / usage errors | manual argv | clap derive | existing |
| Time parsing | regex crate | ~20-line split on `:` | simple; unit-test it |
| NDJSON framing | tokio-util codec | `BufReader::read_line` with cap | already the style; avoid a new dep in presto |

## Runtime State Inventory
Not a rename/refactor phase. Omitted. (New runtime artifacts: `$XDG_RUNTIME_DIR/presto/ctl.sock`, `ctl.lock`, `ctl-demo.*`; all in tmpfs runtime dir, removed on exit or reboot.)

## Common Pitfalls

### Pitfall 1: Window events stop while minimized (Wayland)
**What goes wrong:** media keys/CLI appear dead when the window is hidden.
**Why:** compositors withhold frame callbacks from minimized/occluded surfaces; winit/eframe stops redrawing; `ctx.request_repaint()` from the wake closure may not run `logic()`.
**Avoid:** pump on the tokio task (above). Only Raise/Quit depend on the viewport; verify manually that `Minimized(false)`/`Close` take effect from a minimized window. If `Close` does not run while minimized, fall back to `Backend::shutdown()` + `std::process::exit(0)` in the Quit path.
**Warning sign:** `playerctl`/CLI `pause` works only after clicking the window.

### Pitfall 2: Focus on Wayland
`ViewportCommand::Focus` is advisory under Wayland (no xdg-activation token): KDE shows demands-attention instead of raising. Acceptable (D-08 forbids focus stealing anyway). Test on KDE and note in the verification doc, do not try to work around it.

### Pitfall 3: Two MPRIS players
Engine MediaSession reappears if switches change (SPIKE-REPORT: `--keep-media-session` adds `org.mpris.MediaPlayer2.chromium.instance<pid>`). Add a test/script asserting the engine default switch list still contains `MediaSessionService,HardwareMediaKeyHandling`, plus the busctl count check. Also: demo and real both running = two names; that is why demo gets its own bus name and is not part of the criterion.

### Pitfall 4: Bus name never released / start failure
No session bus (SSH, TTY): crate logs a warning and continues. Do not make MPRIS failure fatal. Name already taken (stale process): `mpris-server` errors; the crate swallows and logs. Surface nothing in UI; log only.

### Pitfall 5: Small-target seeks never answered
Phase 3 finding: live engine never answers `Seek` to targets under ~3 s. Reuse `MIN_SEEK_MS` handling from `playback.rs` for CLI `seek` and MPRIS `SeekBy`/`SetPosition` instead of letting a command time out (5 s).

### Pitfall 6: CLI negative numbers
`presto seek -10` is parsed by clap as an unknown flag. Use `#[arg(allow_hyphen_values = true)]` on the positional (and for `volume -5`).

### Pitfall 7: Position flood
`status --watch` must not emit on every `Progress` event. Rate-limit to 1 line/s while playing; always emit immediately on any non-position change.

### Pitfall 8: `Volume` feedback loops
MPRIS client sets volume -> app sends `SetVolume` -> engine echoes `Volume` event -> app publishes. The crate handles the "held level" case; just do not publish volume from a path that also re-sends a command.

### Pitfall 9: Socket hygiene
Create/bind under the 0700 dir; set 0600; unlink only after holding the flock; reject lines over the cap; never log full frames at info. `XDG_RUNTIME_DIR` unset: GUI already exits 2 in demo; do the same for ctl in real mode and CLI.

## Code Examples

### NowPlaying setup (from fastframe-now-playing lib.rs docs, v0.4.1)
```rust
let mut app = fastframe_now_playing::App::new("presto", "Presto");
app.desktop_entry = "presto".into();       // matches with_app_id("presto") in main.rs
app.uri_schemes = vec![];                  // no OpenUri support
let wake = Arc::new(tokio::sync::Notify::new());
let w = wake.clone();
let mut np = NowPlaying::start(app, move || w.notify_one());
// task loop
loop {
    tokio::select! {
        _ = state_rx.changed() => {}
        _ = wake.notified() => {}
        _ = tick.tick() => {}
    }
    for c in np.commands() { dispatch(c, &state_rx.borrow()); }
    np.update(to_mpris_state(&state_rx.borrow(), art));
}
```
`NowPlaying` has a `Cell` in its throttle (Send, not Sync); keep it owned by the one task. Confirm it compiles inside `rt.spawn` (needs `Send`); if not, run it on a dedicated std thread with a `blocking_recv`-style loop.

### Single instance
```rust
use nix::fcntl::{Flock, FlockArg};
let f = OpenOptions::new().create(true).write(true).open(dir.join("ctl.lock"))?;
match Flock::lock(f, FlockArg::LockExclusiveNonblock) {
    Ok(guard) => guard,                       // keep for process life
    Err((_, nix::errno::Errno::EWOULDBLOCK)) => { /* running: send raise, exit 0 */ }
    Err((_, e)) => return Err(e.into()),
}
```

### Client one-shot
```rust
let mut s = UnixStream::connect(sock).map_err(|e| match e.kind() {
    NotFound | ConnectionRefused => NotRunning, _ => Other(e) })?;
s.set_read_timeout(Some(Duration::from_secs(10)))?;   // > 5 s command timeout
writeln!(s, "{}", serde_json::to_string(&req)?)?;
```

## State of the Art

| Old | Current | Impact |
|-----|---------|--------|
| souvlaki for Linux MPRIS | `mpris-server` 0.10 (tokio feature) via fastframe | fastframe uses it; souvlaki only for mac/win |
| Chromium MediaSession as MPRIS | disabled in engine; Presto owns the name | already done in Phase 2 |

## Open Questions

1. **Stop semantics.** Engine has no `Stop` command (`Command` enum: Play, Pause, Seek, Next, Prev, SetVolume, SetShuffle, SetRepeat, SetQueue, ShowWindow). Recommend `Stop` = `Pause` and keep metadata, since `SetQueue` with empty ids is untested and Plan::Clear in `edit_queue` already uses Pause for "clear". Planner: state this in the plan, no new engine command.
2. **`can_seek(false)` at build in the crate.** `mpris.rs` builds the player with `.can_seek(false)` then `publish()` sets `set_can_seek(controls.seek && track.is_some())` on first/changed state. First `update()` must be sent right after `start` (with `Controls` all false) so CanSeek is correct. Verify with `busctl --user get-property`. If the crate misbehaves, the fallback is mpris-server direct.
3. **Rate property.** Crate source shows no Rate set; mpris-server default is 1.0 with min/max defaults. Verify via busctl (D-09 needs Rate 1.0).
4. **`engine: failed`.** D-14 lists ready/starting/restarting only; `Drift`/`Failed` exist. Recommend emitting `failed` (and `drift`) in JSON; consumers treat unknown as not-ready. Planner/user may prefer mapping both to `restarting`.
5. **Raise under Wayland from background** (Pitfall 2) and **Close while minimized** (Pitfall 1): need one manual check on the user's KDE session.
6. **Where the frame types live** (presto-ipc vs presto crate). Recommend presto-ipc so schema snapshot and token-guard tests cover them; confirm the guard test enumerates modules automatically.
7. **`--demo` targeting from the CLI** (global flag vs env). Recommend global `--demo`.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test (built-in), insta for snapshots, tempfile; no nextest |
| Config file | none; workspace `Cargo.toml`; integration tests in `crates/presto/tests/` (`common/`, `demo_boot.rs`, `backend_mock.rs`) |
| Quick run command | `cargo test -p presto --lib` |
| Full suite command | `cargo test --workspace` |
| Session bus tests | `dbus-run-session -- cargo test -p presto --test desktop_mpris` (dbus-run-session and busctl present on this host) |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DESK-01 | State -> MPRIS `State` mapping (signed out / loading / playing / paused, controls false when not ready, art Pending vs Ready) | unit | `cargo test -p presto --lib status::` | Wave 0 |
| DESK-01 | MPRIS `Command` -> engine `Command` mapping (PlayPause by state, SeekBy clamp + MIN_SEEK_MS, SetPosition stale id ignored, Stop=Pause, no-ops when not ready) | unit | `cargo test -p presto --lib desktop::` | Wave 0 |
| DESK-01 | Under `--demo` mock: exactly one `org.mpris.MediaPlayer2.presto-demo*` name; PlaybackStatus follows mock play/pause; `PlayPause` method call toggles | integration (private bus) | `dbus-run-session -- cargo test -p presto --test desktop_mpris` (uses zbus or shelling `busctl --user`) | Wave 0 |
| DESK-01 | Engine default switches keep MediaSession disabled | unit/grep | `cargo test -p presto-core engine_switches` or `npm test` in `engine/` | partially (check engine/test) |
| DESK-02 | clap parse: subcommands, `seek -10`, `volume +5`, `--demo` global, existing `--fault requires --demo` still holds | unit | `cargo test -p presto --lib cli::` | extend existing `cli.rs` tests |
| DESK-02 | `parse_time` / volume arg parsing | unit | `cargo test -p presto --lib ctl::` | Wave 0 |
| DESK-02 | Status JSON key set stable (insta snapshot) and carries no token-like fields | unit | `cargo test -p presto-ipc` | Wave 0 |
| DESK-02 | ctl server: spawn server on temp socket with a fake state watch; ops ack, bad proto/oversized line rejected, subscribe emits on change and rate-limits position | integration (tokio, no bus) | `cargo test -p presto --test ctl_server` | Wave 0 |
| DESK-02 | Single instance: second lock attempt fails; stale `ctl.sock` with no lock holder is replaced | unit | `cargo test -p presto --lib ctl::lock` | Wave 0 |
| DESK-02 | End to end: `presto --demo` + `presto --demo status --json` prints expected keys; no instance -> stderr "presto is not running", exit 1 | integration (spawn binary, headless unavailable for eframe: see note) | `cargo test -p presto --test cli_e2e` | Wave 0 |
| DESK-01/02 | Media keys on real KDE/GNOME/waybar; Raise from minimized; Close from minimized | manual | checklist in phase VERIFICATION | manual-only (needs real desktop, Wayland compositor) |

Note: `demo_boot.rs` shows how existing tests boot the app without a display; reuse its harness (check how it avoids opening a window) for `cli_e2e`. If the GUI cannot start headless, test the ctl server + desktop task through `Backend` with the mock engine and skip eframe.

### Sampling Rate
- **Per task commit:** `cargo test -p presto --lib`
- **Per wave merge:** `cargo test --workspace`, then the dbus-run-session MPRIS test
- **Phase gate:** full suite green, plus the manual desktop checklist and `busctl --user list | grep mpris` showing one line

### Wave 0 Gaps
- [ ] `crates/presto/src/status.rs` with table tests (pure mapping)
- [ ] `crates/presto/tests/ctl_server.rs` (temp-dir socket, fake `watch<CoreState>`)
- [ ] `crates/presto/tests/desktop_mpris.rs` (private session bus; skip with message if `dbus-run-session` absent)
- [ ] insta snapshot for status JSON in presto-ipc
- [ ] No new framework install needed

## Sources

### Primary (HIGH: read directly)
- `~/.cargo/git/checkouts/fastframe-*/*/crates/fastframe-now-playing/src/{lib.rs,mpris.rs}` and Cargo.toml (v0.4.1): API, bus naming, threading, throttle, file URL escaping
- `~/.cargo/git/checkouts/fastframe-*/*/crates/fastframe-instance/{README.md,src/lib.rs}`: request/reply only, own `instance.sock`
- Presto source: `backend.rs`, `main.rs`, `launch.rs`, `cli.rs`, `app.rs` (logic/on_exit), `mirror.rs`, `state.rs`, `presto-ipc` command/event enums, `artwork.rs`, `paths.rs`
- `.planning/phases/02-engine-feasibility-spike-gate/SPIKE-REPORT.md` section MPRIS; `docs/SPOTIFAST-SEAMS.md`

### Secondary (MEDIUM)
- nix 0.31.3 `Flock` exists in registry source (signature recalled from memory for the example; verify at compile)
- Wayland frame-callback and Focus behavior: general knowledge of winit/compositors, not tested here (LOW-MEDIUM; flagged for manual check)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH, crate source read
- Architecture: MEDIUM, `Send`-ness of `NowPlaying` in a tokio task and Wayland minimized behavior unverified
- Pitfalls: MEDIUM

**Research date:** 2026-10-08
**Valid until:** 2026-11-07 (fastframe tag is pinned, so stable)

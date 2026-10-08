# Phase 3: Core Backend, Supervisor, Auth - Research

**Researched:** 2026-10-07
**Domain:** Rust process supervision (tokio), IPC extension, Electron window/bridge lifecycle, queue mirroring
**Confidence:** MEDIUM-HIGH (all repo facts read from code; Chromium/Electron behaviors from training knowledge are marked LOW and need a live check)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Recovery**
- **D-01:** After a crash or hang restart, restore queue and position. Resume playing only if state was playing at failure; otherwise stay paused at position.
- **D-02:** Backoff 1s, 2s, 4s, doubling, capped at 30s. A failure within 60s of start is a fast failure; after 5 fast failures give up, show "engine failed" with a manual Restart. The counter resets after 2 minutes of stability.
- **D-03:** Stale engine handling: write a pidfile in the state dir. At startup, if the recorded pid is alive and is the engine binary, SIGTERM then SIGKILL it silently and log it. Reap the engine's process group when the main process dies.
- **D-04:** After the 2nd consecutive crash on the same queue, restore the queue paused rather than resuming.
- **D-05:** Hang detection uses the Phase 1 heartbeat (2s x3).

**Sign-in window**
- **D-06:** On `signed_out` the engine window shows immediately, and Presto shows a prompt ("Sign in to Apple Music in the window") with a Bring-to-front button.
- **D-07:** Closing the sign-in window hides it (engine keeps running). Presto shows a Sign in button to reopen it.
- **D-08:** After sign-in the window stays hidden on later launches. It shows only for `signed_out` or re-auth. A debug flag may force-show it.
- **D-09:** While signed out, a sign-in panel blocks the full UI. Cached data shows if present.

**Re-auth**
- **D-10:** On AuthExpired show a banner with a Re-authenticate button. The window opens on click, never automatically.
- **D-11:** After successful re-auth keep queue and position and resume per the D-01 rule.
- **D-12:** While signed out or expired Rust sends no API requests and serves cached data only. Pending requests fail fast with the signed-out state.
- **D-13:** Session state comes from engine auth events and AuthExpired response errors only. No periodic probe.

**Bridge loading and drift**
- **D-14:** bridge.js loads from `$XDG_CONFIG_HOME/presto/bridge.js` if present, else from the install dir copy. Editing and restarting the engine changes behavior with no rebuild.
- **D-15:** The handshake carries bridge version string, capability list and MusicKit build.
- **D-16:** If MusicKit is missing or a required capability is absent, show a blocking error panel ("Apple's web player changed; update bridge.js") with details and log path. This is not counted as a crash and does not enter the restart loop.
- **D-17:** Rust queues commands until `bridge_ready`. After 15s without it, raise the drift error (D-16).

### Claude's Discretion
- Crate layout for the supervisor and queue mirror.
- Queue revision reconciliation details (Phase 1 D-07 snapshots with revision are the base).
- Profile dir creation and 0700 enforcement mechanics (location fixed by Phase 2 D-12).
- Pidfile location and format, log tailing format.

### Deferred Ideas (OUT OF SCOPE)
None. Data layer, caches, ported UI views and MPRIS are other phases.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CORE-01 | Supervisor detects crash/hang, restarts with backoff, restores queue and position | Supervisor actor, pure `Backoff` state machine, restore sequence, pidfile sweep, process-group kill |
| CORE-02 | Bridge loads from standalone file at runtime, handshake reports version and capabilities | `bridge_ready` event (proto 1.1), `main.js` path resolution, drift timer |
| CORE-03 | MusicKit owns queue; Rust holds read-only mirror reconciled by revision | `QueueMirror` with per-session generation, rev discard rules, rapid-change test |
| AUTH-01 | Sign in once via Apple's flow in engine window, then hidden | Window show/hide rules in `main.js`, close-to-hide, new `show_window` command |
| AUTH-02 | Detect expired/signed-out from events, offer re-auth, keep cached data | `AuthMachine`, banner/panel state exposed on `CoreState`, fail-fast gate |
| AUTH-03 | Profile dir mode 0700 | `ensure_private_dir` helper with ownership/symlink check, stat test |
</phase_requirements>

## Summary

Almost everything needed exists as a seed. `presto-spike/src/session.rs` already does listen, spawn, hello, heartbeat (2 s x 3, declared on the 4th tick, about 6 s), per-kind timeouts and clean shutdown. `engine/main.js` already has origin guard, EPIPE hardening, bridge-ready gate (replies `unavailable` before ready), and show on `signed_out` / hide on `signed_in`. `presto-ipc` has the typed vocabulary and `QueueChanged{rev,items,index}` snapshots. Phase 3 is: lift the session into a supervised actor in a new lib crate, add four protocol additions, change three behaviors in `main.js`, and build a Rust-side mirror and auth state machine, all testable against `presto-engine-mock` without Widevine.

The protocol cannot carry D-15 today. The engine `hello` is sent on socket connect, before the page loads, so bridge version, capabilities and MusicKit build must arrive in a later frame. Add an `Event::BridgeReady{version, capabilities, musickit_build}` (additive, bump proto to 1.1). Also needed: a way to show/hide the engine window from Rust (D-06, D-07, D-10) and a way to restore a queue paused (the bridge hardcodes `startPlaying: true`). Both are small additive protocol changes. Unknown-variant deserialization is not tolerant in serde-tagged enums, but both ends ship together, so a minor bump plus the snapshot and `doc_covers_variants` test updates is enough.

The two real risks are process lifecycle (orphans after SIGKILL, Chromium singleton lock forwarding) and queue restore fidelity (MusicKit seek before load, non-song queue items). Both need a live-engine check and cannot be settled by the mock.

**Primary recommendation:** New lib crate `crates/presto-core` containing `Supervisor` (one tokio task owning the child, socket and a pure `Backoff` machine), `QueueMirror`, `AuthMachine`, `paths` (profile 0700, pidfile, logs). Expose a `watch::Receiver<CoreState>` plus an `mpsc` command/request handle. Test everything against the mock using a per-attempt launcher closure and overridable durations. Add proto 1.1 additions first, since mock, engine and core all depend on them.

## Standard Stack

### Core (all already in the workspace unless noted)
| Library | Version | Purpose | Why |
|---------|---------|---------|-----|
| tokio | 1.53.2 (Cargo.lock) | actor loop, `select!`, timers, `process::Command` (has `process_group`, verified in registry source) | CLAUDE.md stack |
| tokio-util LinesCodec | 0.7.19 | NDJSON via existing `presto_ipc::transport` | reuse |
| presto-ipc | path | frames, kinds, timeouts, heartbeat consts | reuse |
| serde / serde_json / thiserror | workspace | state and errors | reuse |
| nix | 0.31.3 (verified via `cargo info`, 2026-10-07) | `kill`, `killpg`, `Signal`, `geteuid`; features `signal`, `process`, `user` | avoids hand-written `unsafe` libc; libc 0.2.190 is already transitive |
| tracing + tracing-subscriber | add (latest at plan time) | supervisor logs with fields | optional; `eprintln!` matches existing code. Decide in plan, do not block on it |

### Supporting (dev)
| Library | Purpose |
|---------|---------|
| tokio `test-util` feature | `start_paused` tests for backoff and drift timer where no real process is involved |
| tempfile, insta | already present |
| `node --test` | engine tests (Node 24, no `test/` path arg; STATE.md) |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `nix` | `rustix` (1.1.5 already in lock) or raw `libc` | rustix is fine too; pick one. `nix::sys::signal::killpg` and `unistd::geteuid` are the shortest |
| separate `presto-core` crate | put it in `crates/presto` | Phase 5 owns the binary and UI. A lib crate lets tests run without egui and lets the `--demo`/CLI reuse it |
| `backoff` crates | hand-written `Backoff` | D-02 has custom fast-failure and stability semantics; 40 lines, see below |

**Installation:**
```bash
cargo add -p presto-core nix --features signal,process,user   # after creating crates/presto-core
```
Add `nix = "0.31"` to `[workspace.dependencies]`. Add `test-util` to tokio dev-dependencies of presto-core only.

## Architecture Patterns

### Recommended Project Structure
```
crates/presto-core/
├── src/lib.rs          # pub Core::spawn(CoreConfig) -> CoreHandle
├── src/config.rs       # CoreConfig: launcher, paths, Timings (overridable for tests)
├── src/paths.rs        # state dir, profile dir (0700), pidfile, log files, bridge path
├── src/backoff.rs      # pure state machine, no tokio, injected Instant
├── src/supervisor.rs   # actor: spawn, handshake, heartbeat, pump, restart, restore
├── src/mirror.rs       # QueueMirror + PlayerMirror (state, position, volume)
├── src/auth.rs         # AuthMachine
├── src/state.rs        # CoreState (what the UI watches)
└── tests/              # against presto-engine-mock
engine/main.js          # + bridge path resolution, close-to-hide, show_window cmd, bridge_ready
engine/bridge.js        # + BRIDGE_VERSION, CAPABILITIES, ready payload, start paused
```
`CoreHandle`: `state() -> watch::Receiver<CoreState>`, `send(Command)`, `request(ApiRequest) -> Outcome`, `show_sign_in()`, `restart_engine()`, `shutdown()`. Phase 5 wraps this in spotifast's Backend trait (see `docs/SPOTIFAST-SEAMS.md`); do not shape it to egui now.

### Pattern 1: One actor owns everything
One task owns child, `Conn`, pending-request map (`id -> oneshot`), ping counter, mirror and auth. Public API talks to it over `mpsc`; UI reads `watch`. No locks. This is the existing `Session::wait_for` loop generalized: `select!` over tick, `recv`, command channel, child exit (`child.wait()`), drift deadline.

Supervisor states: `Starting -> Handshaking -> WaitingBridge -> Ready -> (Backoff | Drift | Failed)`. `Drift` and `Failed` are terminal until `restart_engine()`.

### Pattern 2: Pure Backoff machine (D-02, D-04)
```rust
// Source: derived from D-02/D-04; unit-testable with injected Instants
pub struct Backoff { consec: u32, fast: u32, started: Option<Instant> }
impl Backoff {
    pub fn on_start(&mut self, now: Instant) { self.started = Some(now); }
    /// Call on every crash/hang. Returns None = give up.
    pub fn on_failure(&mut self, now: Instant) -> Option<Duration> {
        let up = now - self.started.take().unwrap_or(now);
        if up >= STABLE { self.consec = 0; self.fast = 0; }   // 2 min stable resets both
        if up < FAST { self.fast += 1; }
        if self.fast >= 5 { return None; }
        let d = Duration::from_secs(1) * 2u32.saturating_pow(self.consec);
        self.consec += 1;
        Some(d.min(Duration::from_secs(30)))
    }
}
```
Constants FAST=60 s, STABLE=120 s. Ambiguity to confirm: a failure at 60 to 120 s uptime is not fast but does not reset either counter (this code does exactly that). See Open Questions.

Same-queue crash counter (D-04): `last_queue_key: u64` (hash of item ids) and `same_queue_crashes`. On failure: if the mirror's queue key equals the key at the previous failure, increment, else set to 1. At 2 or more, restore paused regardless of `was_playing`. Reset on stable period or on a different queue.

### Pattern 3: Restore sequence (D-01, D-04, D-11)
Snapshot taken at failure from the mirror: `ids`, `index`, `position_ms`, `was_playing`. After the new engine reports `bridge_ready` and auth `signed_in`:
1. `SetQueue{ids, start: index, play: false}` (new field, see Protocol Additions).
2. Wait for `QueueChanged` (restore is done when the mirror shows the same ids).
3. `Seek{ms: position_ms}`.
4. `Play` only if `was_playing && same_queue_crashes < 2`.

Do the snapshot from the mirror, never query the dead engine. Use last `Progress` (500 ms granularity). Gate the user's own commands behind the restore (queue them) so a user click during restore is not overwritten.

### Pattern 4: Auth machine (D-06..D-13)
States `Unknown | SignedOut | SignedIn | Expired`, input only from `Event::Auth` and `ErrorKind::AuthExpired` outcomes. Effects: entering `SignedOut` sets `CoreState.prompt = SignIn` and Rust sends `show_window(true)`; `Expired` sets a banner and sends nothing; `Core::show_sign_in()` sends `show_window(true)`. While not `SignedIn`, `request()` returns `Outcome::Err{AuthExpired}` immediately without touching the socket (D-12), and `Cmd` playback commands are rejected the same way. Transport/queue state and cached data stay in `CoreState`; the UI decides what to render.

### Pattern 5: Queue mirror reconciliation (CORE-03)
Rules, in order:
- Mirror carries `(generation, rev)`. `generation` increments on every engine (re)start because the bridge's `rev` counter restarts at 0 in a new page. Reset `rev` baseline at `bridge_ready`.
- Apply `QueueChanged` only if `rev > last_rev` (Phase 1 D-07). Drop equal and lower.
- Mirror is read-only: no Rust-side optimistic edits. The only writes are `SetQueue` commands, and the mirror changes only when the resulting event arrives.
- `Progress` older than the latest `seq` is dropped (Phase 1 rule). `seq` also restarts per engine, so reset it at `bridge_ready`.
- Optional hardening: bridge coalesces `queueItemsDidChange` plus `queuePositionDidChange` within one microtask/animation frame into one snapshot (see Pitfall 5). The mirror remains correct without it because each event is a full snapshot with a higher rev.

### Pattern 6: Bridge loading (D-14, D-15, D-17)
`main.js` resolves once at startup: `process.env.XDG_CONFIG_HOME || ~/.config` + `/presto/bridge.js`, if `fs.existsSync`, else `path.join(__dirname, 'bridge.js')`. Keep reading from disk on every `inject()` (already the case). Log the chosen path. A syntax error in the user's copy makes `executeJavaScript` reject (`inject failed` is already logged) and `bridge_ready` never arrives, so the 15 s drift timer fires: correct behavior with no extra code.

`bridge.js` adds `const BRIDGE_VERSION = '...'` and `CAPABILITIES = ['playback','queue','api','window'?...]`, and passes both plus `MusicKit.version`/build into `__presto.ready()`. `main.js` forwards it as `{t:'evt', evt:{type:'bridge_ready', ...}}` (instead of only logging). Presto holds a `REQUIRED_BRIDGE_CAPS` constant; missing any, or `musickit_build == null`, gives `Drift` with details (D-16).

The drift timer starts when the engine connects (after hello) and is cancelled by `bridge_ready`. It does not restart on reload: `main.js` sets `bridgeReady=false` on navigation, and the page navigates during sign-in. Signing in, a legitimate flow, can take minutes with no bridge (the bridge only installs when `MusicKit` exists and `getInstance()` works, which the sign-in page may or may not satisfy). LOW confidence on this: verify live whether `bridge_ready` fires on the signed-out page. If it does not, the 15 s timer must be suppressed while the engine window is visible for sign-in, or the bridge must be ready before auth. Spike evidence: sign-in worked with `--show` and `authEvt()` emitted `signed_out`, which requires the bridge to have started, so the bridge does install on the signed-out page (MEDIUM, inferred from `logs/engine-signin-*.log`; confirm).

### Anti-Patterns to Avoid
- **Counting Drift as a crash:** D-16 says it is not. Keep the engine alive in `Drift`; the user edits bridge.js and presses Restart.
- **Replaying user commands after a crash:** only the restore sequence replays. Fail in-flight requests with `Unavailable`.
- **Periodic auth probe:** forbidden by D-13.
- **Rust reading cookies/tokens to check the session:** forbidden by project constraint.
- **Relying on `--diag`:** causes SIGTRAP (SPIKE-REPORT). Never pass it from the supervisor.

## Protocol Additions (proto 1.0 -> 1.1)

All additive. Update `PROTO`, `docs/PROTOCOL.md` (hand-written, `doc_covers_variants` test fails if a wire name is missing), insta snapshots under `crates/presto-ipc/tests/snapshots`, the IPC-02 token schema test (`bridge_ready.version` and `capabilities` are fine; avoid any name containing token/auth/secret segments).

| Addition | Shape | Why |
|----------|-------|-----|
| `Event::BridgeReady` | `{ version: String, capabilities: Vec<String>, musickit_build: Option<String> }` | D-15, D-17. Hello precedes the page, so it cannot carry these |
| `Command::ShowWindow` | `{ show: bool }`, engine handles in `main.js`, never forwarded to the page | D-06, D-07, D-10. Must work even when bridge is not ready (sign-in window) |
| `SetQueue.play` | `#[serde(default = "true")] play: bool` | restore paused without an audio blip; bridge uses `startPlaying: cmd.play !== false` |
| capability `window` | in `caps` | gate the above |

`ShowWindow` must be answered by `main.js` directly, before the `!bridgeReady` gate (current code replies `unavailable` to all `cmd` when not ready). Mock must implement all three (emit `bridge_ready` after hello, ack `show_window`, honor `play`).

Mock additions for tests: a startup state `signed_out` (e.g. `--fault signed_out` or `--auth signed_out`), `--bridge-missing` (never sends `bridge_ready`, for drift), `--bridge-caps` variants, and the ability to crash on attempt N only. The last is better solved in the test launcher closure (vary argv per attempt) than in the mock.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Signals / process groups | raw `unsafe libc::kill` | `nix::sys::signal::{kill, killpg}` | safe wrappers |
| Spawn in own group | `setsid` via `pre_exec` | `tokio::process::Command::process_group(0)` | stable, verified in tokio 1.53.2 source |
| Heartbeat / framing / timeouts | new ones | `presto_ipc` (`HEARTBEAT_*`, `Kind::timeout`, `transport`) | already defined and tested |
| Mock engine | a second fake | `presto-engine-mock` + faults | D-10/D-12 of Phase 1 |
| 0700 dir | `mkdir` then hope | `DirBuilder::mode(0o700).recursive(true)` then `set_permissions` (pattern already in `presto-ipc::transport::bind` and spike `private_dir`) | umask-independent |
| Unix socket path/stale socket | custom | `transport::bind` | removes stale file, 0600 |

Reuse, do not rewrite: lift `Session::start`, `wait_for` ping logic and `shutdown` from `crates/presto-spike/src/session.rs`; move `default_profile()` from spike `main.rs` into `paths.rs`.

## Runtime State Inventory

This is not a rename phase, but it adds persistent files and process state the planner must treat explicitly.

| Category | Items | Action |
|----------|-------|--------|
| Stored data | Engine profile `$XDG_STATE_HOME/presto/engine-profile` (cookies, CDM, from Phase 2, mode already 0700 from spike) | AUTH-03: verify and `chmod` on every start; refuse if not a real dir owned by euid |
| Live service config | None | None |
| OS-registered state | Orphan engine process from a prior SIGKILL (observed pid ppid=1 for ~12 min); Chromium `SingletonLock`/`SingletonSocket`/`SingletonCookie` in the profile | pidfile sweep at start; see Pitfall 1 |
| Secrets/env vars | `PRESTO_SOCKET`, `PRESTO_PROFILE` (paths only); `XDG_CONFIG_HOME` for bridge.js override | none |
| Build artifacts | `engine/node_modules` (ECS binary needs `npm install --allow-git=root` then `node node_modules/electron/install.js`) | document in plan prerequisites; user-config `bridge.js` is user-owned and never overwritten by install |

## Common Pitfalls

### Pitfall 1: Orphaned engine and Chromium singleton forwarding
**What goes wrong:** After SIGKILL of presto the engine main (and children) lived on. The next engine, on the same profile, either crashed (SIGTRAP seen 3x) or, per Chromium behavior, hands off to the live instance and exits quietly (LOW, training knowledge; consistent with an engine that "connects to nothing").
**Why:** `main.js` quits on socket `close`, but `app.quit()` may stall (cookie flush, open window), and nothing hard-exits.
**How to avoid (layered):**
1. Engine: on socket close call `app.quit()` and arm `setTimeout(() => app.exit(0), 3000).unref()`. This is the primary orphan fix because it also covers presto SIGKILL.
2. Presto: spawn with `process_group(0)`; on graceful shutdown and `Drop` of the supervisor `killpg(pgid, SIGTERM)` then after 3 s `SIGKILL`. Verify live that Chromium helpers share the pgid (`ps -o pid,pgid,cmd`); LOW.
3. Presto start: read pidfile. Alive check: `/proc/<pid>/stat` start time (field 22) equals recorded value and `readlink /proc/<pid>/exe` equals the canonicalized engine binary. If so, `killpg`/`kill` SIGTERM, poll up to 3 s, SIGKILL, log at warn. Then delete pidfile. Pidfile JSON: `{pid, start_ticks, exe}` in `$XDG_STATE_HOME/presto/engine.pid`, mode 0600, written after spawn, removed on clean exit.
4. Secondary: if profile `SingletonLock` is a symlink to `host-<pid>` and that pid is not alive, delete the three Singleton* files before spawn.
Skip `PR_SET_PDEATHSIG`: it fires on death of the spawning *thread*, which is wrong under tokio's multi-thread runtime. Layers 1 and 3 cover the same case.
**Warning signs:** engine exits with code 0 within a second of spawn and never connects; `ps` shows ppid 1 electron.

### Pitfall 2: Hang vs. slow engine, and killing it
`wait_for` in the spike declares hang only when it also pumps frames. The supervisor loop must ping on a timer independent of any pending request, and the engine answers pong in `main.js` without the page (already true). Hang action: SIGKILL group directly after a 1 s SIGTERM grace; do not wait for `Frame::Cmd` timeouts. Fail all pending requests with `Unavailable`.

### Pitfall 3: Engine startup and sign-in flash
`bridge.js start()` emits `signed_out` if `mk.isAuthorized` is false at instance creation. If MusicKit authorizes asynchronously, a transient `signed_out` makes `main.js` `win.show()` on every launch, violating D-08. LOW; spike did not record a flash. Mitigation to plan: bridge emits the initial auth event only after a short settle (e.g. wait for first `authorizationStatusDidChange` or 1 to 2 s), and Rust treats `signed_out` after a prior `signed_in` within the same session as real. Verify live.

### Pitfall 4: Close-to-hide and quit
`app.on('window-all-closed', () => app.quit())` makes D-07 impossible. Add `win.on('close', (e) => { if (!quitting) { e.preventDefault(); win.hide(); } })` with `quitting` set in `before-quit`. Remove or keep `window-all-closed` as a no-op. Also: window closed by the user while `signed_out` means no way back except Presto's Sign in button, so `show_window` is required (D-07).

### Pitfall 5: Rapid queue changes and generation resets
MusicKit fires both `queueItemsDidChange` and `queuePositionDidChange`; bridge bumps `rev` each time and emits the whole `mk.queue.items` map (can be hundreds of items; 4 MiB line cap). Two traps: (a) engine restart resets `rev` to 0, so a naive "drop lower rev" mirror ignores the whole new session (fix: reset baseline at `bridge_ready`); (b) large queues re-serialized per event; coalesce in bridge (single `queueMicrotask`/50 ms trailing emit) and keep rev monotonic.
Success criterion 5 test: mock fires N rapid `set_queue`/`next`; assert mirror ids/index equal the mock's final state and `rev` equals the last. For the real engine, a manual checklist compares `mk.queue.items` ids to the mirror after rapid skips.

### Pitfall 6: Restore fidelity
- `SetQueue.ids` are song ids (`mk.setQueue({songs})`). Queue items from library (`i.`-prefixed ids), stations, albums-as-container or video are not guaranteed to round-trip. Detect: if any mirror item id fails a simple catalog-id check, restore what is possible (current item) and surface a note. Multi-item queues were never exercised live (SPIKE-REPORT). LOW.
- `mk.seekToTime` before the item has loaded may be ignored or rejected. Plan a live check; fallback is to wait for `PlaybackState::Paused` or `Loading` to settle, then seek, then verify `Progress` within 2 s of target, and retry once.
- `changeToMediaAtIndex` was never exercised live (SPIKE-REPORT). It is used when `start > 0`. Must be tested in the live checklist.
- Shuffle/repeat/volume: restore too (cheap: `SetShuffle`, `SetRepeat`, `SetVolume` from the mirror). Not mandated by D-01 but a restart that resets shuffle is visibly wrong. Include; it is three commands.

### Pitfall 7: Re-auth path in the page
Bridge emits `auth` from `isAuthorized` and `authorizationStatusDidChange`; it never emits `expired` on its own, and the mock is the only source today. Real expiry surfaces as a 401/403 on `mk.api.music` mapped to `auth_expired` (implemented, unit-tested, never triggered live). Rust must flip to `Expired` on that outcome (D-13). How the user re-authenticates in the page when cookies expired (does music.apple.com show a Sign In control, does `isAuthorized` flip) is unverified; LOW. Plan a manual live step: invalidate the session (clear cookies in profile copy) and confirm the page shows sign-in and emits `auth signed_in` after.

### Pitfall 8: Fast-failure death spiral with an unresponsive first launch
First launch downloads the Widevine CDM (needs network and time) before playback works. If the engine crashes in this window five times, D-02 gives up. Acceptable, but the `Failed` panel must show the log tail and path. Do not treat "no connect within 30 s" (spike value) as instant failure on first-ever run; keep 30 s, count it as a fast failure.

### Pitfall 9: Unix socket path length and runtime dir
Socket path is limited to about 108 bytes; tests use `/tmp` short dirs (see mock `tests/common`). `XDG_RUNTIME_DIR` may be unset in CI; the existing `NoRuntimeDir` error is correct. Core config must accept an explicit socket path.

## Code Examples

### Private directory (AUTH-03)
```rust
// Source: pattern in crates/presto-ipc/src/transport.rs bind() and presto-spike main.rs private_dir()
pub fn ensure_private_dir(p: &Path) -> io::Result<()> {
    DirBuilder::new().recursive(true).mode(0o700).create(p)?;
    let md = std::fs::symlink_metadata(p)?;
    if !md.is_dir() { return Err(io::Error::other("profile path is not a real directory")); }
    if md.uid() != nix::unistd::geteuid().as_raw() { return Err(io::Error::other("profile dir not owned by current user")); }
    if md.mode() & 0o777 != 0o700 { std::fs::set_permissions(p, Permissions::from_mode(0o700))?; }
    Ok(())
}
// test: assert_eq!(std::fs::metadata(&p)?.permissions().mode() & 0o777, 0o700) with umask 0 and a pre-existing 0755 dir
```

### Spawn with its own process group
```rust
// tokio 1.53.2 Command::process_group verified in registry source
let mut cmd = tokio::process::Command::new(&opts.bin);
cmd.args(&opts.args).process_group(0).stdout(log.try_clone()?).stderr(log).kill_on_drop(true);
let child = cmd.spawn()?;
let pgid = Pid::from_raw(child.id().unwrap() as i32);   // pgid == pid when process_group(0)
// later: nix::sys::signal::killpg(pgid, Signal::SIGTERM)
```

### Drift timer inside the actor loop
```rust
let drift = tokio::time::sleep_until(deadline); tokio::pin!(drift);
select! { _ = &mut drift, if !bridge_ready => { state = Drift{...}; /* no backoff, engine left running */ } ... }
```

### Engine hard-exit after socket close (main.js)
```js
for (const ev of ['end','close','error']) {
  sock.on(ev, (e) => { log('socket', ev, e?.message ?? ''); quitting = true; app.quit(); setTimeout(() => app.exit(0), 3000).unref(); });
}
```

## State of the Art

| Old | Current | Impact |
|-----|---------|--------|
| Spike `Session` borrowed from a single checklist | Supervised actor with restart | Same code, wrapped |
| `bridge.js` read from `__dirname` | XDG config override then install copy | one `existsSync` |
| Window shown only by engine heuristics | Rust-driven `show_window` plus engine auto-show on `signed_out` | needed for D-07, D-10 |

## Open Questions

1. **Backoff counter semantics between 60 s and 120 s uptime.**
   - Known: fast failure `<60 s`, reset after 2 min stable.
   - Unclear: does a failure at 90 s keep the doubled delay but not count as fast?
   - Recommendation: yes, as coded above (two counters). Delay resets with the fast counter at 120 s. Confirm in discuss only if the planner objects.
2. **Does `bridge_ready` fire on the signed-out page, and does `authEvt` flash `signed_out` at launch?** Needs live check; decides whether the 15 s drift timer must be suppressed while the window is shown for sign-in. Recommendation: plan a live checklist task and implement the drift timer as "no `bridge_ready` within 15 s of engine connect while auth is not known to be signed_out", with the simpler version as fallback.
3. **MusicKit seek-before-load and `changeToMediaAtIndex` behavior.** Live check; restore sequence has a retry-once-and-verify step.
4. **Do Chromium helpers share the engine pgid; does a live orphan cause the exit-0 forwarding?** Live check under the pidfile task. Layers 1 and 3 do not depend on the answer.
5. **Re-auth UX in the real page after cookie expiry.** Manual live step.
6. **Log tail format (Claude's discretion).** Recommendation: `logs/engine-<unix_ts>.log` in the state dir (as the spike does), keep newest 10, tail last 40 lines as `Vec<String>` in `CoreState.failure`.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Rust | cargo test, tokio `#[tokio::test]`, insta snapshots (existing) |
| Engine JS | `node --test` in `engine/` (existing; `npm test`) |
| Config | none beyond Cargo workspace; new crate needs `[dev-dependencies] tokio test-util, tempfile` |
| Quick run | `cargo test -p presto-core -p presto-ipc` |
| Full suite | `cargo test --workspace && (cd engine && npm test)` |

### Phase Requirements to Test Map
| Req | Behavior | Type | Command | Exists? |
|-----|----------|------|---------|---------|
| CORE-01 | Backoff sequence 1,2,4,...,30; fast-fail give-up at 5; reset after 2 min | unit (pure) | `cargo test -p presto-core backoff` | Wave 0 |
| CORE-01 | Mock `crash@ms` mid-play: restart, queue and position restored, resumes if was playing | integration | `cargo test -p presto-core --test recover` | Wave 0 |
| CORE-01 | Mock `hang`: declared after 3 missed pings (override interval to 100 ms), kill, restart | integration | same | Wave 0 |
| CORE-01 | 2nd crash on same queue restores paused (D-04); paused-at-failure stays paused (D-01) | integration | same | Wave 0 |
| CORE-01 | Stale pidfile with live matching pid is killed; non-matching pid untouched | unit/integration (spawn `sleep`) | `cargo test -p presto-core --test stale` | Wave 0 |
| CORE-02 | Editing the bridge file changes the loaded path and version | engine unit | `cd engine && npm test` (resolve-path function extracted from main.js into `bridge-path.js`) | Wave 0 |
| CORE-02 | `bridge_ready` carries version, caps, build; roundtrip + snapshot + doc coverage | ipc | `cargo test -p presto-ipc` | updates existing |
| CORE-02 / D-16, D-17 | No `bridge_ready` in 15 s (override) gives Drift, no restart; missing cap gives Drift | integration | `cargo test -p presto-core --test drift` | Wave 0 |
| CORE-03 | Stale/equal rev dropped; generation reset accepted; rapid set_queue/next ends consistent | unit + integration | `cargo test -p presto-core mirror` | Wave 0 |
| AUTH-01 | `signed_out` sets prompt, sends `show_window(true)`; `signed_in` clears | integration (mock `--auth signed_out`) | `cargo test -p presto-core --test auth` | Wave 0 |
| AUTH-01 | close-to-hide, `show_window` handling, no auto-show on `expired` | engine unit (extract window policy to a pure function) | `cd engine && npm test` | Wave 0 |
| AUTH-02 | `auth_expired` event or response gives Expired, requests fail fast without socket write, mirror/cache retained, re-auth restores per D-11 | integration | same | Wave 0 |
| AUTH-03 | profile dir mode 0700 after create, after pre-existing 0755, rejects symlink/foreign owner | unit | `cargo test -p presto-core paths` | Wave 0 |

Live (manual, needs account and Widevine; list as one checkpoint task, not automated): kill -9 electron mid-play and confirm resume; edit `~/.config/presto/bridge.js` and restart; first-launch sign-in window then hidden; invalidate session; rapid skips queue compare; check `stat -c %a` on profile.

### Sampling Rate
- Per task commit: quick run for the touched crate.
- Per wave merge: full suite.
- Phase gate: full suite green, then the live checkpoint, before `/gsd:verify-work`.

### Wave 0 Gaps
- [ ] `crates/presto-core` crate and workspace entry, `nix` workspace dep
- [ ] Mock: `bridge_ready`, `show_window`, `SetQueue.play`, startup `signed_out`, bridge-missing option
- [ ] `presto-ipc` 1.1 additions with snapshots/docs updated
- [ ] Test launcher closure (per-attempt argv) and `Timings` override (heartbeat interval, drift, backoff base) so integration tests run in under 10 s
- [ ] `engine/bridge-path.js` and a window-policy module extracted from `main.js` so they are unit-testable without Electron

## Sources

### Primary (HIGH confidence, read this session)
- `crates/presto-ipc/src/*`, `docs/PROTOCOL.md` (frames, timeouts, heartbeat, errors, caps)
- `crates/presto-engine-mock/src/main.rs`, `player.rs`, `tests/common/mod.rs` (fault model, SetQueue behavior, test harness)
- `crates/presto-spike/src/session.rs`, `main.rs` (supervisor seed)
- `engine/main.js`, `bridge.js`, `guard.js`, `preload.js` (engine seed and its gaps)
- `.planning/phases/02-engine-feasibility-spike-gate/SPIKE-REPORT.md` (orphan, SIGTRAP, bridge-ready, queue API, unexercised paths)
- tokio 1.53.2 registry source (`Command::process_group`), `cargo info nix` (0.31.3), Cargo.lock

### Secondary / Tertiary (LOW, training knowledge, flagged inline)
- Chromium `SingletonLock` forwarding behavior and helper pgid membership
- `PR_SET_PDEATHSIG` per-thread semantics
- MusicKit seek-before-load and post-expiry sign-in UX

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH (all in workspace; nix version verified)
- Architecture: HIGH for supervisor/mirror/auth (derived from existing code and decisions); MEDIUM for restore fidelity
- Pitfalls: MEDIUM (repo-derived ones HIGH; Chromium/MusicKit behaviors LOW until a live check)

**Research date:** 2026-10-07
**Valid until:** 30 days for repo facts; re-check ECS and Apple web player behavior at execution time

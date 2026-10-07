# Phase 2: Engine Feasibility Spike (GATE) - Research

**Researched:** 2026-10-07
**Domain:** castlabs ECS (Electron+Widevine) hosting music.apple.com, driven over the Phase 1 NDJSON socket
**Confidence:** MEDIUM (ECS/Sidra facts verified from source; MusicKit internals and Wayland hidden-window behavior must be measured, not assumed)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Spike castlabs ECS first. Spike system Chrome via CDP only if ECS fails a criterion. CEF and WebKitGTK get a desk comparison in the report.
- **D-02:** ECS version: look up the newest supported tag at spike time (v44.1.0 is unconfirmed) and use it. Record the exact tag.
- **D-03:** Test hidden-window behavior (`show:false`) on both Wayland and X11. Playback must continue while hidden.
- **D-04:** Record the date and observed web player build in the report. No resilience work in the spike; that belongs to the Phase 3 bridge design.
- **D-05:** The spike engine speaks the real Phase 1 `presto-ipc` protocol over the Unix socket (hello, commands, requests, events). No ad-hoc messages.
- **D-06:** A `presto-spike` crate in the workspace is the Rust driver. It listens on the socket, spawns the engine, and runs the checklist (play, seek past 60 s, proxy call, event stream). A REPL mode is not required.
- **D-07:** Spike code is a seed for Phase 3. Keep `engine/` and the spike crate tidy enough to harden rather than rewrite.
- **D-08:** The bridge script is a standalone file (`engine/bridge.js`) read from disk and injected into music.apple.com at runtime, matching CORE-02 from the start.
- **D-09:** The spike runs on the user's own subscriber account.
- **D-10:** The plan starts with a gate task: the user reads the current Apple Media Services terms and says proceed. No sign-in happens before it.
- **D-11:** The user signs in by hand in the visible engine window, then the window hides. Credentials never touch a script, log or the repo.
- **D-12:** The engine profile lives in a dedicated directory under XDG state (for example `~/.local/state/presto/engine-profile`), mode 0700, never shared with another browser. Phase 3 AUTH-03 keeps this location.
- **D-13:** GO requires all five roadmap success criteria met. Any miss means NO-GO or a documented alternative.
- **D-14:** RSS (idle and playing) is measured and reported with no pass/fail threshold. The user judges.
- **D-15:** If ECS fails, try Chrome via CDP, then report. If both fail, the report states NO-GO with evidence and options.
- **D-16:** The report is `SPIKE-REPORT.md` under the phase directory, with raw measurement logs saved beside it.

### Claude's Discretion
- Spike crate layout and how the checklist is scripted.
- How measurements are taken (RSS sampling method, duration of playback runs).
- Which test tracks and library endpoint variants to use beyond `/v1/me/library/playlists`.
- Report structure beyond the required items in SPIKE-06.

### Deferred Ideas (OUT OF SCOPE)
- Smoke script to re-detect web player breakage: revisit in Phase 3 bridge design.
- Interactive REPL driver: not needed for the gate.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SPIKE-01 | music.apple.com exposes usable MusicKit instance | Sidra polls `window.MusicKit` then `MusicKit.getInstance()` (can throw during init; retry). Bridge does the same. |
| SPIKE-02 | Sign-in persists across engine restarts | Persistent session via `userData` dir / `persist:` partition; Sidra signs in on music.apple.com with an auth iframe; needs Chrome-like UA |
| SPIKE-03 | Command from Rust plays full track past 60 s and across seek | `mk.setQueue({song:id,startPlaying:true})`, `mk.seekToTime(sec)`; autoplay policy switch for hidden window |
| SPIKE-04 | `/v1/me/library/playlists` proxied through page returns JSON | `MusicKit.getInstance().api.music(path, query)` runs inside the page with the page's tokens |
| SPIKE-05 | Playback state events stream to Rust | MusicKit events `playbackStateDidChange`, `playbackTimeDidChange`, `mediaItemDidChange`, `queueItemsDidChange`; map to `Event` |
| SPIKE-06 | Measurements + candidate comparison + recommendation | Measurement recipes below; Sidra and CEF desk facts |
</phase_requirements>

## Summary

The approach is proven by Sidra (wimpysworld/sidra, v1.1.2), which loads `music.apple.com` directly in castlabs ECS `v44.1.0+wvcus`, injects a hook that taps `MusicKit.getInstance()` events, and drives it with `mk.play/pause/seekToTime/setQueue/skipToNextItem`. I cloned and read its source. The spike is mostly a port of that pattern into `engine/main.js` + `engine/bridge.js` that speaks `presto-ipc` instead of Electron IPC to a tray app.

ECS tags verified with `git ls-remote` on 2026-10-07: `v43.7.7+wvcus`, `v44.1.0+wvcus`, `v44.5.1+wvcus` exist. The releases page lists 44.5.1 (Oct 3, 2026) as the latest stable and mentions 42.x and 45.0.0-alpha builds. Use `v44.5.1+wvcus` (v44.1.0 as fallback, since Sidra ships on it). Linux needs no VMP/EVS signing. Sidra says macOS/Windows return "Something went wrong" after login without it; irrelevant on Linux.

The real unknowns that the spike must measure: whether `show:false` windows keep playing on Wayland (autoplay policy, occlusion/throttling), whether Apple login works in the ECS window with a Chrome UA, the stream codec, and RSS. Apple-side drift is accepted (D-04). The machine used for research is a tty with no display, so every GUI step (sign-in, Wayland/X11 comparison, playback) is a human-in-the-loop checkpoint.

**Primary recommendation:** Port Sidra's pattern (Chrome UA override, `components.whenReady()`, persistent partition, MusicKit hook) into a thin `engine/main.js` + `engine/bridge.js`; use `mk.api.music()` for the proxy; verify with a Rust `presto-spike` driver that uses `presto_ipc::transport` directly.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| castlabs electron-releases | `github:castlabs/electron-releases#v44.5.1+wvcus` (tag verified via ls-remote 2026-10-07; Sidra uses v44.1.0) | Chromium + Widevine CDM component updater | Only ECS-class shell with Widevine on Linux; Sidra/Cider precedent |
| Node (bundled in Electron) | Electron's own; system Node is v24.21.0 | engine main process; `net` for the Unix socket | `net.createConnection(path)`; no extra dependency |
| presto-ipc (workspace) | 0.1.0 | Rust wire types + `transport::{bind,framed,send,recv}` | Phase 1 contract (D-05) |
| tokio, serde_json | workspace | driver | already in workspace |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| clap | workspace | `presto-spike` args (`--engine-dir`, `--profile`, `--log-dir`) | driver CLI |
| sysinfo or /proc/<pid>/smaps_rollup | n/a | RSS sampling | Prefer a shell loop reading `/proc` PSS+RSS of the whole process tree; no crate needed |
| chrome-remote-interface / raw CDP | n/a | only if D-15 Chrome fallback runs | fallback only |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| ECS | System Chrome 154 (installed: `/usr/bin/google-chrome-stable`) + CDP | Fallback per D-15; Wayland hidden window problem |
| `mk.api.music()` | `fetch()` with tokens read from MusicKit | Violates the spirit of "bridge owns auth"; do not read tokens at all |

**Installation (engine/):**
```bash
mkdir -p engine && cd engine
npm init -y
npm install --save-dev "github:castlabs/electron-releases#v44.5.1+wvcus"
# pin exact tag in package.json; commit package-lock.json
```
No `electron-log`, `electron-conf`, or builder in the spike. Version check at plan time: `git ls-remote --tags https://github.com/castlabs/electron-releases | grep wvcus | tail`.

## Architecture Patterns

### Recommended Project Structure
```
engine/
├── package.json          # pinned ECS tag
├── main.js               # args, socket client, window, CDM wait, bridge injection
├── bridge.js             # standalone file read from disk (D-08); runs in the page
└── preload.js            # contextBridge: sends events to main, receives commands
crates/presto-spike/
├── Cargo.toml
└── src/main.rs           # bind socket, spawn engine, hello, checklist, RSS log
.planning/phases/02-.../
├── SPIKE-REPORT.md
└── logs/                 # raw rss.csv, events.ndjson, codec capture
```

### Pattern 1: ECS startup (from Sidra src/main.ts, verified)
**What:** Set a truthful Chrome UA (strip "Electron"), wait for CDM, use a persistent partition, then load the site.
**Example:**
```js
// Source: wimpysworld/sidra src/main.ts (read locally)
const { app, BrowserWindow, components, session } = require('electron');
const ver = process.versions.chrome.split('.')[0] + '.0.0.0';
app.userAgentFallback = `Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/${ver} Safari/537.36`;
app.commandLine.appendSwitch('autoplay-policy', 'no-user-gesture-required');
app.setPath('userData', profileDir);           // from --profile, before ready
app.whenReady().then(async () => {
  await components.whenReady();                 // downloads Widevine CDM on first run
  console.error(JSON.stringify(components.status()));
  const ses = session.fromPartition('persist:presto');
  ses.setUserAgent(UA);
  const win = new BrowserWindow({ show: false, webPreferences: {
    partition: 'persist:presto', preload: path.join(__dirname,'preload.js'),
    contextIsolation: true, nodeIntegration: false, sandbox: true, plugins: true }});
  win.loadURL('https://music.apple.com', { userAgent: UA });
});
```
`autoplay-policy` is not in Sidra (it shows the window); CLAUDE.md lists it as needed for hidden play. Test with and without; record which is required.

### Pattern 2: Bridge hook (from Sidra assets/musicKitHook.js)
Poll until `window.MusicKit` exists, then `try { mk = MusicKit.getInstance() } catch { return }` inside the interval (getInstance throws while initializing; clearing the poll early would end setup). Inject with `webContents.executeJavaScript(fs.readFileSync('bridge.js'))` after `did-finish-load` and again on `did-navigate`; guard with a `window.__prestoBridge` flag. Sidra also runs a 5 s monitor for MusicKit instance replacement.

Event mapping (names verified in Sidra hook and Apple developer forum posts):

| MusicKit | presto-ipc `Event` |
|---|---|
| `playbackStateDidChange` (state: 0 none, 1 loading, 2 playing, 3 paused; `MusicKit.PlaybackStates` has more, e.g. stopped/ended/stalled, read the enum at runtime rather than hard-coding) | `playback_state` |
| `playbackTimeDidChange` or 500 ms timer on `mk.currentPlaybackTime` / `mk.currentPlaybackDuration` (seconds) | `progress` (ms) |
| `mediaItemDidChange` (`mk.nowPlayingItem`, attributes `name, artistName, albumName, durationInMillis, artwork.url`) | `track_changed` |
| `queueItemsDidChange` / `queuePositionDidChange` (`mk.queue.items`, `mk.queue.position`) | `queue_changed` |
| `playbackVolumeDidChange` | `volume` |
| `authorizationStatusDidChange` / `mk.isAuthorized` | `auth` |

Command mapping: `play`->`mk.play()`, `pause`->`mk.pause()`, `seek{ms}`->`mk.seekToTime(ms/1000)` (requires a nowPlayingItem), `next/prev`->`skipToNextItem/skipToPreviousItem`, `set_volume`->`mk.volume=`, `set_queue{ids,start}`->`mk.setQueue({songs: ids, startPlaying:true})` then `changeToMediaAtIndex(start)`. `set_queue` option shapes (`song`, `songs`, `album`, `startTime`) are from Apple forum posts (MEDIUM); confirm in the spike and record the queue API surface (SPIKE-06).

### Pattern 3: API proxy
`await MusicKit.getInstance().api.music('/v1/me/library/playlists', { limit: 25 })` returns the parsed body in `.data` (MusicKit v3: `{data: {data:[...], next}}`). Confirm the exact return shape and the path form (leading slash vs `v1/...`) in the spike; MusicKit adds developer token, Music-User-Token and storefront itself, so Rust never sees them. Map failures: HTTP 401/403 -> `auth_expired`, 429 -> `rate_limited` (read `Retry-After` if exposed), 404 -> `not_found`, others -> `upstream{status}`. Page returns only `{ok, status, data}`; the bridge must never return headers or tokens. Responses over 4 MiB cannot cross the socket (MAX_LINE); use `limit` and for the spike just fail with `internal` if oversized.

Page to main: use `contextBridge` in preload (`window.__presto.emit(evt)`, `window.__presto.onCommand(cb)`). Note executeJavaScript runs in the main world; preload bridge is exposed to main world via contextBridge, so `bridge.js` can call `window.__presto`. Sidra does the same via `window.AMWrapper`.

### Pattern 4: Rust driver
Use existing `presto_ipc::transport`: `socket_path()` / `bind(path)` -> `UnixListener`, accept, `framed(stream)`, `recv`/`send` of `Frame`. Spawn engine as `tokio::process::Command` running `engine/node_modules/.bin/electron engine --socket <p> --profile <d>` with stdout/stderr redirected to a log file (PROTOCOL.md rule). The Phase 1 mock harness (`crates/presto-engine-mock/tests/common/mod.rs`: `Harness`, `start_raw`, `expect`) is the pattern to copy, not depend on. Driver runs: hello exchange, heartbeat pings every 2 s (engine main must answer `pong` independent of page work), then checklist steps with assertions and prints a PASS/FAIL table.

Hello: the engine lists capabilities `playback`, `queue`, `api`; never `mock`.

### Anti-Patterns to Avoid
- **Reading MusicKit tokens** (`mk.musicUserToken`, `developerToken`) anywhere in bridge code: breaks the no-token rule and the schema test spirit.
- **Typing credentials via script or CDP** (violates D-11).
- **Running `electron` with `--no-sandbox` by default**: needed only if the chrome-sandbox SUID helper is missing in `node_modules`; Sidra uses `sandbox:true` for renderers. If needed, record it in the report.
- **Sharing a profile with Chrome/other apps** (D-12). Also never point the Chrome fallback at `~/.config/google-chrome`.
- **Logging API response bodies at info level**.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Widevine CDM sourcing | manual CDM download/copy | `components.whenReady()` in ECS | component updater handles version/compat |
| Authenticated Apple API calls | fetch with extracted tokens | `MusicKit.getInstance().api.music()` | no tokens leave the page |
| Playback | `<audio>` + manual HLS/EME | Apple's web player itself | FairPlay/Widevine, HLS, key rotation |
| Frame types / framing | new JSON shapes | `presto-ipc` | D-05 |
| MPRIS | engine-side MPRIS | none in this phase; just detect duplicates | DESK-01 is Phase 6 |
| Login UI | custom login form | Apple's own page in visible window | D-11 |

## Runtime State Inventory
Not a rename/migration phase. Omitted. Note for new state created: engine profile dir (0700) and socket under `$XDG_RUNTIME_DIR/presto/`, both created by this phase.

## Common Pitfalls

### Pitfall 1: Apple rejects the Electron UA
**What goes wrong:** Login or playback fails with "Something went wrong" or the site blocks the client.
**Why:** Default Electron UA contains `Electron/x`. Sidra comments that Apple Music detects and blocks it.
**Avoid:** Chrome UA built from `process.versions.chrome`, platform string truthful so it matches `Sec-CH-UA-Platform`; set via `app.userAgentFallback`, `session.setUserAgent`, and `loadURL(..., {userAgent})`.

### Pitfall 2: CDM not ready, or downloads fail offline
**What goes wrong:** Protected content errors, no playback.
**Avoid:** `await components.whenReady()` before `loadURL`; log `components.status()` (title, version, status) for the report. First run needs network to castlabs/Google update servers.

### Pitfall 3: Hidden window pauses or throttles
**What goes wrong:** `show:false` window audio stops, timers throttled, or Wayland occlusion handling stalls MusicKit timers.
**Avoid:** `autoplay-policy=no-user-gesture-required`; `webPreferences.backgroundThrottling:false`; consider `--disable-renderer-backgrounding`. Test hide after play (`win.hide()`) and `show:false` from launch, on Wayland (`--ozone-platform=wayland`) and X11 (`--ozone-platform=x11` under XWayland or a real X session). Record each combination. Note: a window never `show()`n on first run cannot be used to sign in, so first run is visible, later runs hidden (D-11).

### Pitfall 4: MusicKit not ready / `getInstance()` throws
Poll with try/catch; do not clear the interval until an instance returns. SPIKE-01 passes when `getInstance()` returns and `isAuthorized` is readable after sign-in.

### Pitfall 5: Sign-in does not persist
**Avoid:** one persistent partition, `app.setPath('userData', profile)` before ready, quit cleanly (`app.quit()`, not SIGKILL) so cookies flush; for the restart test also test SIGKILL once and record. Dump cookie *presence* only (count and domain, never values) for the report.
Login happens inside an iframe on music.apple.com; Sidra needs an `authFrameFix` (CSS/passkey container). If the passkey/QR prompt misbehaves, note it as a finding, not a spike failure, as long as password+2FA completes.

### Pitfall 6: Duplicate MPRIS
Chromium's MediaSessionService registers `org.mpris.MediaPlayer2.chromium.instance<pid>` on the session bus. Sidra disables it with `--disable-features=MediaSessionService,...`. Measure by `busctl --user list | grep -i mpris` (or `playerctl -l`) before and during playback, with and without the flag. Report whether the flag removes it. Also check `navigator.mediaSession` still exists (Apple writes to it).

### Pitfall 7: Seek before item loaded
`seekToTime` needs a `nowPlayingItem`; wait for `playing` state first. Test seek forward, e.g. to 120 s, and back; confirm progress continues and no stall state.

### Pitfall 8: Preview vs full playback
Subscriber account is required; a 30 s preview or a stop at 30 s means not authorized or wrong storefront. Choose a long catalog track (>3 min) and verify `durationInMillis` matches played duration, not 30 s.

### Pitfall 9: 4 MiB line limit and event floods
`progress` at 500 ms is fine. Library responses with large pages can exceed 4 MiB; request `limit=25`.

### Pitfall 10: Electron stdout noise / sandbox helper
Chromium prints to stderr; route stdout+stderr to a log (PROTOCOL.md). The `chrome-sandbox` SUID error on Linux appears on Electron installs from npm; if present, run with `--no-sandbox` for the spike and flag it in the report as a packaging concern (Phase 7).

## Code Examples

### Bridge: ready + event tap (shape, adapt from Sidra)
```js
// engine/bridge.js — read from disk by main.js, injected into music.apple.com
(() => {
  if (window.__prestoBridge) return;
  window.__prestoBridge = true;
  const t = setInterval(() => {
    if (!window.MusicKit) return;
    let mk; try { mk = MusicKit.getInstance(); } catch { return; }
    clearInterval(t);
    const P = window.__presto;                       // contextBridge from preload
    P.ready({ bridge: 1, build: document.querySelector('script[src*="musickit"]')?.src ?? null });
    for (const ev of ['playbackStateDidChange','mediaItemDidChange','queueItemsDidChange','playbackVolumeDidChange'])
      mk.addEventListener(ev, e => P.emit(ev, { state: e?.state, idx: mk.nowPlayingItemIndex }));
    setInterval(() => mk.isPlaying && P.emit('progress',
      { pos: mk.currentPlaybackTime, dur: mk.currentPlaybackDuration }), 500);
    P.onCommand(async (c) => {
      switch (c.type) {
        case 'play': return mk.play();
        case 'pause': return mk.pause();
        case 'seek': return mk.seekToTime(c.ms / 1000);
        case 'set_queue': await mk.setQueue({ songs: c.ids, startPlaying: true }); break;
      }
    });
    P.onRequest(async (r) => {
      const res = await mk.api.music(r.path, r.query);   // verify return shape in spike
      return res.data;
    });
  }, 250);
})();
```
Do not copy token-reading; none appears above.

### Rust: accept the engine
```rust
// uses presto_ipc::transport::{socket_path, bind, framed, send, recv}
let path = transport::socket_path()?;
let listener = transport::bind(&path)?;
let mut child = tokio::process::Command::new("engine/node_modules/.bin/electron")
    .args(["engine", "--socket", path.to_str().unwrap(), "--profile", profile.to_str().unwrap()])
    .stdout(log.try_clone()?).stderr(log).spawn()?;
let (s, _) = listener.accept().await?;
let mut conn = transport::framed(s);
// recv engine Hello, send presto Hello (proto 1.0, role presto), then drive checklist
```
Check exact `Hello`/`Frame` constructors in `crates/presto-ipc/src/frame.rs` while planning.

### RSS sampling (no crate)
```bash
# every 5 s for N minutes; sum RSS over electron process tree
while sleep 5; do
  ps -o rss= --ppid "$PID" -p "$PID" ; pgrep -P "$PID" | xargs -r ps -o rss= -p
done | awk '{s+=$1} END{print s/1024 " MiB"}'
```
Better: walk descendants (`pstree -p`) and sum `Rss` and `Pss` from `/proc/<pid>/smaps_rollup`; report both, since shared pages double-count in RSS. Record at: engine idle after hello (before sign-in page load), signed-in idle hidden, playing 5 min hidden. Log `date,phase,rss_mib,pss_mib` to `logs/rss.csv`.

### Stream codec
Options, in order: (a) CDP via `win.webContents.debugger` `Network.enable` and log the `.m3u8` master playlist URLs and `CODECS=` attributes (no tokens; strip query strings); (b) `chrome://media-internals` equivalent via `Media` CDP domain; (c) `mk.nowPlayingItem.attributes.audioTraits` / `MediaSource` `isTypeSupported` probe. Expect AAC (mp4a.40.x, ~256 kbps) over HLS with Widevine; Sidra's docs note lossless only on macOS/Windows with production VMP, and "software decryption on Linux". Treat as unconfirmed until captured (MEDIUM/LOW).

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| ECS v16-era `components` API | `components.whenReady()` / `components.status()` | still current in Sidra on v44 | use it |
| Persistent Widevine licenses | removed in Chromium/Widevine from Electron 43+ | ECS releases note | no offline licenses; irrelevant here |
| ECS builds per week | one build per month per series (since Aug 2025), supported v43-v45 | ECS wiki | pin exact tag; expect to bump monthly |
| ECS linux-arm64 | introduced with v44.1.0 (releases page) | Sep 2026 | CLAUDE.md says no aarch64 CDM; unverified for this release, low priority |

**Deprecated/outdated:** CLAUDE.md's "v44.1.0 unconfirmed" is resolved: tag exists; 44.5.1 is newer.

## Open Questions

1. **Does Apple's login complete in ECS on Linux with this UA?** Sidra says yes on Linux. Measure.
2. **Hidden `show:false` playback on Wayland**: unknown; the whole point of D-03. Fallback: `win.minimize()`, offscreen positioning, or `paintWhenInitiallyHidden`. Record which works.
3. **MusicKit `api.music` signature/return shape and whether it respects absolute `/v1/...` paths.** Verify in a DevTools console during the spike, record in report.
4. **Terms/ToS**: gate task D-10, user-only. Planner must make it a blocking checkpoint with no code before it that touches Apple.
5. **Display availability**: the research machine session is `tty` (no WAYLAND_DISPLAY/DISPLAY). The executor must run GUI steps in the user's desktop session; sign-in and playback checks are `checkpoint:human-verify`.
6. **Chrome fallback (D-15)**: only if needed. System Chrome is 154.0.8037.57. Would use `--user-data-dir` under XDG state and `--remote-debugging-pipe`. Plan it as a conditional task, not default.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test (libtest, tokio) for protocol-level driver logic; the live spike is a runnable binary + human checkpoints |
| Config file | workspace `Cargo.toml` (members `crates/*` already picks up `presto-spike`) |
| Quick run command | `cargo test -p presto-spike` |
| Full suite command | `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SPIKE-01 | MusicKit instance usable | live, manual | `cargo run -p presto-spike -- check musickit` | Wave 0 |
| SPIKE-02 | sign-in persists across restart | manual (credentials) + driver reports `auth signed_in` after relaunch | `cargo run -p presto-spike -- check session` | Wave 0 |
| SPIKE-03 | play >60 s and across seek | live, driver asserts progress >= 60 s and post-seek continuity | `cargo run -p presto-spike -- check playback` | Wave 0 |
| SPIKE-04 | library proxy returns JSON | live, driver asserts `res ok` with `data` array | `cargo run -p presto-spike -- check api` | Wave 0 |
| SPIKE-05 | events stream | live, driver counts `playback_state`/`progress` frames | `cargo run -p presto-spike -- check events` | Wave 0 |
| SPIKE-06 | report complete | file check | `grep -c` for required headings in SPIKE-REPORT.md | Wave 0 |
| (offline) | driver handshake against the mock engine | integration | `cargo test -p presto-spike` using `presto-engine-mock` binary | Wave 0 |

Automatable offline: the driver's checklist logic can be tested against `presto-engine-mock` (no Apple, no GUI), which proves the driver before the live run. A Node-side unit test for event/command mapping can run with plain `node --test` against a stubbed `MusicKit` object (`engine/test/bridge.test.js`); no Electron needed.

### Sampling Rate
- **Per task commit:** `cargo test -p presto-spike` (and `node --test engine/test` once present)
- **Per wave merge:** full suite command
- **Phase gate:** full suite green plus live checklist PASS table committed in `logs/`

### Wave 0 Gaps
- [ ] `crates/presto-spike/` crate and offline mock-engine test
- [ ] `engine/package.json`, `engine/main.js`, `engine/bridge.js`, `engine/preload.js`
- [ ] `engine/test/bridge.test.js` with stub MusicKit
- [ ] `npm install` of ECS (network, ~hundreds of MB download)
- [ ] `.gitignore` entries: `engine/node_modules`, `logs` content with any account data (logs hold no tokens by design; review before commit)

## Sources

### Primary (HIGH confidence)
- wimpysworld/sidra cloned and read locally (package.json, src/main.ts, assets/musicKitHook.js, README, docs): ECS pin, UA override, `components.whenReady`, hook pattern, MPRIS disable flags, VMP note
- `git ls-remote --tags` on castlabs/electron-releases: tag existence
- Local repo: docs/PROTOCOL.md, crates/presto-ipc/src (transport API), engine mock harness

### Secondary (MEDIUM confidence)
- https://github.com/castlabs/electron-releases/releases (via fetch summary): latest 44.5.1, supported series; wiki: v43-v45 supported, monthly builds
- Apple developer forums threads (setQueue `startTime`/`startPlaying`, `seekToTime`, `playbackStateDidChange` values) via search summary

### Tertiary (LOW confidence)
- Stream codec expectation (AAC over HLS) and `api.music` return shape: unverified, measure in spike
- Wayland hidden-window behavior: no source found, measure in spike

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH for ECS tag and pattern (source-verified), MEDIUM for 44.5.1 behavior (Sidra is on 44.1.0)
- Architecture: MEDIUM, MusicKit method shapes need console verification
- Pitfalls: MEDIUM, from Sidra source and forum posts

**Research date:** 2026-10-07
**Valid until:** 2026-11-07 (ECS ships monthly; Apple web player can change anytime)

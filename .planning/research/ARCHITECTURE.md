# Architecture Patterns

**Domain:** Native Rust/egui music client with a hidden Chromium playback and data engine
**Researched:** 2026-10-07
**Confidence:** MEDIUM. Spotifast structure is HIGH (read from source, v0.12.0). MusicKit queue API details, Chromium audio/MPRIS flags and engine choice details are training-data knowledge and need the spike to confirm (marked LOW below).

## What spotifast actually does (reference study)

Read from the cloned repo (`crmne/spotifast`, edition 2024, single crate, ~166k lines including 9k `demo.rs`, 23k `app.rs`, 6k `backend.rs`).

**There is no playback trait and no API trait.** The seam is a concrete message pair:

| Seam | Where | Shape |
|------|-------|-------|
| UI to backend | `Backend::send(Command)` in `src/backend.rs` | `mpsc::UnboundedSender<Command>` into a tokio runtime on its own thread (`spotifast-backend`, 2 workers) |
| Backend to UI | `Backend::poll() -> Vec<Event>` | `std::sync::mpsc::Receiver<Event>` drained with `try_iter` each frame |
| Wake-up | `fastframe_shell::Waker` | Every event calls `waker.wake()`, which `request_repaint`s whichever window exists. App is idle otherwise |
| Playback commands | `PlayerCommand` (Toggle, Next, Previous, Seek, Volume, Shuffle, Repeat, Load, AddToQueue, ClearQueue, Transfer) in `src/player.rs` | Sent inside `Command` |
| Playback state | `LocalState` (playback, track, position_ms + `position_at` Instant, volume, shuffle, repeat, connected, loading, `track_sequence`, `seek_sequence`, error) | Pushed as `Event::Local(Box<LocalState>)` |
| Data requests | `ApiRequest` enum (about 40 variants, each carrying a `generation: u64`) | Answered as `Event::Api(Box<ApiResponse>)`; generation lets the UI drop stale answers |
| Auth | `AuthStatus` and `LocalPlayback` enums | `Event::Auth`, `Event::Playback` |
| Demo | `Backend::set_offline(true)` plus `src/demo.rs` pre-populating `App` state | `--demo`, `--demo-page`, `--demo-show`, `--demo-data` behind a `demo` cargo feature. Not a mock backend: commands are swallowed, state is injected |
| UI discipline | `src/ui/*` draws and emits `Action`s; `app.rs` applies them after the frame | AGENTS.md rule: never mutate app state inside a borrowed view |
| Optimistic UI | AGENTS.md: "The interface is optimistic, always"; a lagging backend answer must never undo the user's action | `docs/_reference/queue.md` is the queue contract, enforced by tests in `app.rs` |

Other modules worth knowing:

| Concern | Spotifast | Reusable in Presto? |
|---------|-----------|---------------------|
| Shell, tray, single instance, i18n, theme, fonts, emoji, log, scroll, update | `fastframe-*` crates (git tag v0.4.1, shared with ZapFast, RekordFlash, TonePush) | YES, as-is. Pin the same tag |
| MPRIS | `fastframe-now-playing`: app calls `NowPlaying::start(App, wake)`, `commands()` yields `Command` (Play, Pause, PlayPause, Next, Previous, SeekBy, SetPosition{track_id,position}, SetVolume, SetShuffle, SetRepeat, OpenUri, Raise), `update(State{playback, track{id,title,artists,art_file,..}})`. Runs on its own thread; app stays the only decider. `app.rs` maps `MediaCommand` to `Action` | YES, as-is. Feed it from engine events. Needs `art_file` as a local path, so artwork must be on disk first |
| Artwork cache | `images.rs` `ArtLoader`: disk cache keyed by URL hash, atomic `.part` then rename, bytes dropped once a texture exists | YES, reuse pattern. Apple artwork URLs are templates (`{w}x{h}`), so key on resolved URL |
| Playlist and liked-songs cache | `model.rs` `PlaylistCache` keyed by account id + playlist id + `snapshot_id`; `liked.rs` `Cache` | Pattern yes. Apple has no `snapshot_id`; use `lastModifiedDate` or item count (LOW, verify in spike) |
| Keyring | `credentials.rs` (1.2k lines): `keyring-core`, one dedicated thread, generation checks vs sign-out, revocation markers. Stores Spotify grants and proxy password | NO for Apple (Rust holds no tokens). Keep only if a proxy password is needed. Drop the module otherwise |
| Settings | `settings.rs`, atomic JSON, backward compatible | YES |
| Single instance and CLI control | `fastframe-instance`, clap in `entrypoint.rs` | YES |
| Packaging | `PACKAGING.md`, nFPM, AUR tooling, portable archive, `check-runtime-libs.c`, Ubuntu 24.04 runners | Pattern yes. Presto adds an engine binary and a Chromium/CEF payload |

**Spotify-coupled (drop or replace):** `api/` (client, gateway with shared vs personal app routing), `auth.rs`, `credentials.rs` Spotify slots, `player.rs` (librespot Spirc/Connect), `session_reads.rs`, `zeroconf.rs` (Connect receivers), `devices.rs` UI, `radio.rs`, most of `backend.rs` `Worker`, `lyrics.rs` (Spotify lyrics source), `playlist_cover.rs`, PKCE/browser sign-in UI (`login.rs`).

**Backend-agnostic (keep):** `ui/*` layout and widgets, `theme.rs`, `i18n.rs`, `bidi.rs`, `images.rs`, `settings.rs`, `paths.rs`, `util.rs`, `history.rs`, `autoscroll`, `window.rs`, `single_instance.rs`, `updates.rs`, tray, MPRIS wiring, the Backend/Waker/Event pump, the optimistic-UI and generation-counter conventions.

**Becomes dead weight (audio-path features):** `sink.rs`, `eq.rs`, `limiter.rs`, `resample.rs`, `vis.rs`, `milkdrop.rs`, `winamp.rs` and `ui/winamp/*`. They sit on PCM that librespot hands the app. In Presto the audio is decoded and played inside Chromium, so Rust never sees samples. Do not port them. This is the largest silent scope cut; state it in the roadmap.

**Porting reality:** the UI is entangled with Spotify types (`api::models::*`, URIs like `spotify:track:..`, `PlayableItem`). `app.rs` at 23k lines is the real coupling surface, not `backend.rs`. Plan a model-translation layer: Presto defines its own `Track/Album/Artist/Playlist/Station` models in `presto-core` shaped like spotifast's so UI code ports with search-and-replace instead of a rewrite.

## Recommended Architecture

```
                         +------------------------------------------+
  presto (bin)           | presto-ui   egui views, Action enum      |
  main thread            |   draws state, emits Action              |
                         +----------------+-------------------------+
                                          | Action -> Command (after frame)
                                          | Event  <- poll() each frame, Waker repaints
                         +----------------v-------------------------+
  presto-core            | Backend handle (Command in, Event out)   |
  tokio runtime thread   |  - AppState reducers (optimistic overlay)|
                         |  - Library/Search/Detail fetchers        |
                         |  - Cache (art files, library snapshots)  |
                         |  - Supervisor (spawn, ping, restart)     |
                         |  - MPRIS bridge (fastframe-now-playing)  |
                         +----------------+-------------------------+
                                          | presto-ipc: NDJSON frames
                          +---------------+----------------+
                          | Transport trait                |
                 stdio pipes (real)              in-process channel (demo)
                          |                                |
        +-----------------v---------------+   +------------v-----------+
        | presto-engine (bin, child proc) |   | mock engine (same code |
        |  Chromium host + bridge.js      |   |  as `--mock`)          |
        |  music.apple.com, Widevine      |   +------------------------+
        |  profile dir: sign-in persists  |
        +----------------+----------------+
                         | audio: Chromium -> PulseAudio client API (pipewire-pulse) -> PipeWire
```

Dependency direction: `presto-ui -> presto-core -> presto-ipc`; `presto-engine -> presto-ipc`; nothing depends on `presto-engine` except the supervisor spawning it by path. `presto-ui` never imports `presto-ipc` types: the UI sees only core models, `Command` and `Event`.

### Workspace layout

| Crate | Kind | Contents | Depends on |
|-------|------|----------|-----------|
| `presto-ipc` | lib | Protocol types (serde), framing codec, version constant, `Transport` trait, request-id/timeout table. No egui, no tokio runtime ownership (tokio only for the table's timers, or keep sync) | serde, serde_json |
| `presto-core` | lib | Domain models, `Backend` handle (`send(Command)`, `poll() -> Vec<Event>`), engine client (maps IPC to Events), supervisor, caches, queue mirror, MPRIS adapter, settings, paths | presto-ipc, tokio, fastframe-now-playing, directories |
| `presto-ui` | lib | All views, theme, i18n, widgets, `App` state, `Action` application | presto-core, egui/eframe 0.36, fastframe-* |
| `presto` | bin | `main`: clap, fastframe-shell, single instance, wires ui to core. `--demo` flag | presto-ui |
| `presto-engine` | bin | Chromium host, bridge loader, IPC server side, `--mock` mode (same protocol, canned catalog) | presto-ipc (plus CEF or chosen engine crate) |
| `bridge/` (not a crate) | JS | `bridge.js` and `README` of MusicKit surface used. Loaded from disk at runtime (`$XDG_DATA_HOME/presto/bridge.js` overriding the packaged copy) | none |

Why this split: `presto-ipc` has to be tiny and stable because it is the contract the replaceable engine implements. `presto-engine` must not pull egui, and `presto-ui` must not pull Chromium or CEF bindings (build times, linking). Four crates is the minimum that enforces those two rules. A fifth crate for the mock is not worth it: `--mock` lives in `presto-engine` behind the same IPC code, and demo mode links the mock as a library module (`presto-engine` lib target plus bin target) over an in-process `Transport`.

### Component boundaries

| Component | Responsibility | Talks to |
|-----------|----------------|----------|
| presto-ui | Draw, emit `Action`s, hold view state and the optimistic overlay | Backend handle only |
| Backend (core) | Command dispatch, event fan-in, reducers, cache access | UI, EngineClient, Cache, MPRIS |
| EngineClient | Own the transport, request table, heartbeat; translate `Command` to IPC and IPC events to `Event` | Supervisor, Transport |
| Supervisor | Spawn `presto-engine`, detect exit/hang, backoff restart, re-handshake, replay desired state | EngineClient |
| Cache | Artwork files, library snapshots, search/recent TTL entries | Backend, ArtLoader |
| MPRIS adapter | Translate engine playback events to `NowPlaying::update`, media commands to `Command` | Backend, fastframe-now-playing |
| presto-engine | Host Chromium, show/hide window, inject bridge, relay | IPC peer; Chromium; no UI knowledge |
| bridge.js | Wrap MusicKit instance: playback control, queue, events, `api` proxy, auth state | presto-engine via a page-to-host channel (CEF message router / CDP binding / preload IPC, engine-dependent) |

## IPC protocol

**Transport:** child's stdin/stdout, newline-delimited JSON (one object per line, UTF-8, no embedded newlines; serde_json emits compact JSON so this holds). stderr is the engine log, captured by the supervisor into the fastframe-log file. Rationale: dies with the parent (set `PR_SET_PDEATHSIG` on Linux in the engine), needs no socket path or permission handling, trivial to fake in tests and for the mock. Large responses (a 100-item library page is tens of KB) are fine on a pipe; cap frames at 8 MB and fail the request over that. Move to a Unix socket only if the engine must be started independently of the UI (not planned).

**Envelope** (all frames):

```json
{"v":1,"k":"cmd|req|res|evt","id":42,"op":"play","p":{...}}
```

| Kind | Direction | Has id | Semantics |
|------|-----------|--------|-----------|
| `cmd` | UI to engine | no | Fire and forget playback intent: `play`, `pause`, `toggle`, `next`, `previous`, `seek{ms}`, `set_volume`, `set_shuffle`, `set_repeat`, `set_queue{source, start_index}`, `queue_add{next:bool}`, `queue_remove{index}`, `show_window{reason}`, `hide_window`, `shutdown`. Results arrive as `evt` state changes |
| `req` | UI to engine | yes (u64, UI-allocated, monotonic) | Anything needing an answer: `api{method, path, query, body}`, `snapshot` (full state resync), `ping`, `lyrics{track}` |
| `res` | engine to UI | echoes id | `{ok:true,data}` or `{ok:false,err:{code,status?,message}}`. Codes: `not_signed_in`, `rate_limited{retry_after}`, `http{status}`, `bridge_unavailable`, `timeout`, `unsupported` |
| `evt` | engine to UI | no | `hello`, `bridge_ready`, `auth{state}`, `playback{state, position_ms, at_ms, track?, duration, volume, shuffle, repeat, loading, seq}`, `queue{items, index, rev}`, `error`, `log`, `pong` |

**Handshake:** engine sends `evt hello{protocol:1, engine:"cef|electron|cdp", engine_version, bridge_version, capabilities:[...]}` first. The core refuses a different major `protocol` and shows an actionable error ("engine X is older than this UI"). Minor additions are ignored-if-unknown: unknown `op`/`evt` names are logged and dropped, unknown JSON fields ignored (`#[serde(default)]`, no `deny_unknown_fields`). `capabilities` gates optional UI (lyrics, queue editing, stations) so a bridge patched against a changed Apple player degrades instead of breaking. `bridge_version` is reported separately because bridge.js is hot-patchable.

**Timeouts:** the core owns the request table (`HashMap<id, Pending{deadline, reply}>`). Defaults: `api` 20 s, `snapshot` 5 s, `ping` 2 s. Timeout resolves the request with `timeout` and drops the late response if it ever arrives (ids never reused within a process). The engine also enforces a per-request page-side deadline slightly shorter than the core's so the bridge can return a specific error. `generation` counters stay in core `Event`s exactly as spotifast does, so UI-level stale-answer dropping still works.

**Crash, hang and reload recovery:**

| Failure | Detection | Recovery |
|---------|-----------|----------|
| Engine process exits | EOF on stdout, `wait()` | Fail all pending requests with `bridge_unavailable`; emit `Event::Engine(Restarting)`; restart with backoff 0.5 s, 1 s, 2 s, then 5 s cap; after N=5 failures in 10 min, stop and show a persistent error (mirrors spotifast `RECONNECT_WINDOW`/`RECONNECT_LIMIT` flap guard) |
| Engine hang (alive but silent) | `ping` every 3 s; 3 missed pongs; also no `playback` evt while state says Playing past `position + 10 s` | SIGTERM, 2 s grace, SIGKILL, then the crash path |
| Page reload or bridge reinjection (Apple navigated, renderer crashed, bridge.js hot-reloaded) | engine sends `evt bridge_ready` again | Core fails pending requests, re-requests `snapshot`, re-sends desired subscriptions. No process restart |
| Renderer crash inside Chromium | Engine reports it as `evt error{renderer_crashed}` and reloads the page itself | Same as bridge reload |
| Resume after restart | Core keeps `Interrupted{track_id, position_ms, playing}` (copy spotifast `player::Interrupted`) | After `auth: signed_in` and `bridge_ready`, issue `set_queue` + `seek`; resume playing only if it was playing and the user has not issued a command meanwhile |
| Protocol mismatch | `hello.protocol` | Do not restart-loop; surface error |

**Desired-state replay:** core stores the last user intent (volume, shuffle, repeat) and re-applies after every `bridge_ready`. Do not assume the page remembers them.

## Queue ownership

**Recommendation: MusicKit owns the queue. Rust holds a read-only mirror plus an optimistic overlay.**

Reasons (LOW confidence on API specifics, verify in the spike):
- MusicKit JS handles continuation: autoplay after the queue ends, stations, "play this album then similar" and Apple's own gapless/prefetch. A Rust-owned queue that loads one item at a time through `setQueue({song: id})` loses prefetch and gapless and re-implements radio.
- Two sources of truth is the failure the project named. With MusicKit as owner there is exactly one: Rust never decides "what plays next", it only displays and requests.
- Spotifast's queue contract (`docs/_reference/queue.md`) has the same shape: the service owns the queue, the UI shows it optimistically and reconciles. Port the rules, not a new model.

Mechanics:
- `set_queue{source}` where source is `{album|playlist|station|songs[]: id(s), start_index}`. Maps to `music.setQueue(...)`.
- `queue_add{ids, next}` maps to `playNext`/`playLater`. `queue_remove{index}` and reordering depend on what MusicKit exposes (`queue.remove` may not exist; LOW). If removal is impossible, the fallback is rebuild the queue from the mirror via `setQueue` with `startPosition` and a seek back to the position. That fallback is the only place Rust writes a full queue, and it is a single atomic command, not ongoing ownership. Decide in the spike which queue edits are native.
- The bridge emits `queue{items, index, rev}` on `queueItemsDidChange`/`queuePositionDidChange`. `rev` is a bridge-incremented counter; the core ignores events with `rev` lower than the last applied.
- Optimistic overlay (spotifast rule 4 and 8): Next pops the head immediately in the UI; the overlay is dropped when an engine event with a later `rev` arrives that agrees or supersedes. A lagging event older than the user's action is ignored, never applied (hold shown state and re-request `snapshot`).
- The queue is not persisted by Rust as authoritative. Persist `Interrupted` (current item, position) for resume only, plus the item list as a courtesy so the panel is not empty on launch. After restart it is re-applied through the single `set_queue` command.

If the spike shows MusicKit's queue is not readable (items hidden for stations), the UI shows "Up next" from `nextPlayableItem` only and disables the queue panel for stations. Capability flag `queue_read`.

## Auth and session lifecycle

Rust holds no Apple tokens. The only auth state Rust sees is a boolean-ish enum emitted by the bridge.

```
EngineStarting -> BridgeLoading -> SignedOut --(user: Sign in)--> SigningIn --> SignedIn
       ^                               ^                              |             |
       +---- restart/reload -----------+--- session expired / signed out elsewhere --+
```

`AuthStatus` (UI-facing, replaces spotifast's `AuthStatus`): `Starting`, `SignedOut`, `SigningIn` (engine window shown), `SignedIn { display_hint: Option<String>, storefront: Option<String> }`, `Failed(String)`, `EngineUnavailable`.

- Sign-in: the "Sign in" button sends `show_window{reason:"signin"}`. The engine reveals its Chromium window on the Apple login page (via `music.authorize()` popup or the site's own button; spike decides). When `auth{SignedIn}` arrives the core sends `hide_window`. This is the only moment the engine is user-visible. If the Apple flow opens popups (2FA, Apple ID domain), the engine window must allow them; this is an engine-feasibility item.
- Persistence: a dedicated Chromium profile dir at `$XDG_DATA_HOME/presto/engine-profile` (not the user's browser profile). Cookies and the page's stored music-user-token live there. The profile is the credential: file mode 0700, and "Sign out" clears it by sending `sign_out` (calls `music.unauthorize()`) then deleting the profile subdirs for cookies/storage on next engine start.
- Expiry: the bridge reports `auth{SignedOut}` when `authorizationStatus` changes or an `api` call returns 401/403; the core drops to SignedOut, keeps cached library visible read-only (stale banner), and prompts re-sign-in. No automatic popup.
- No keyring for Apple. `credentials.rs` is not ported. The architecture rule "Rust holds no tokens" is enforced structurally: `api` proxy requests carry path/query/body only, and the bridge injects authorization by calling MusicKit's own `music.api`, never by exposing `music.developerToken` or `musicUserToken`. Add a lint-style test: no IPC message type has a field named token/authorization, and the bridge never forwards headers.
- Storefront: bridge fills `{storefront}` in paths (`/v1/me/library/...`, `/v1/catalog/{storefront}/...`). Rust writes paths with the literal placeholder.

## Data layer and cache

Flow: UI `Action` -> `Command::Fetch(Request, generation)` -> cache lookup -> (stale-while-revalidate) `req api` -> `res` -> parse to core models -> cache write -> `Event::Data{generation, ...}`.

| Data | Where | Policy |
|------|-------|--------|
| Artwork | disk, `ArtLoader` pattern (hash of resolved URL, atomic write), bounded size with LRU prune by mtime | Request at the pixel size needed (Apple URL template); never refetch if file exists. Also required by MPRIS (`art_file`) |
| Library snapshot (songs, albums, artists, playlists) | JSON files keyed by `account/storefront/kind`, atomic write | Show immediately on launch, revalidate in background after sign-in, diff by id + `lastModifiedDate`. Paginated by `offset`/`limit` with the API's `next` link until exhausted |
| Playlist and album details | JSON keyed by id | TTL 10 min for catalog, revalidate-on-open for library playlists |
| Search, browse, recommendations, recently played | memory only, short TTL | Not worth persisting |
| Profile/storefront hint | settings file | |

Storage format: JSON files for v1 (spotifast does the same, `PlaylistCache`, `liked::Cache`). Ceiling: libraries beyond roughly 20k songs make load/parse slow; the switch is `rusqlite` with one table per kind, behind the same `Cache` functions. Do not start with SQLite.

Reads run on the tokio runtime or `spawn_blocking`, never on the UI thread. All cache entries carry a `schema` integer; mismatch means ignore and refetch.

Do not cache anything that is a token or contains per-request authorization. Cache is keyed by account id derived from the bridge (opaque hash) so account switches do not leak data.

## MPRIS

Use `fastframe-now-playing` unchanged. The core owns one `NowPlaying` instance created on the main thread (macOS constraint, same on Linux for symmetry) with `App::new("presto", "Presto")`; `bus_name` `presto` yields `org.mpris.MediaPlayer2.presto`.

- Source of truth is the engine `playback` event, not UI state. The adapter converts `PlaybackEvent -> State` and calls `update` on change; the crate republishes position once a second itself and expects an explicit `seeked` for jumps (detect via `seq` or position discontinuity > 1.5 s from `position + elapsed`).
- Commands from the desktop go through the same `Command` path as UI buttons so optimistic behavior is identical.
- Artwork: wait for the disk cache file before the first `update` for a track (or send the update without art and a second one with it).
- **Chromium has its own MPRIS and media-key handling** (MediaSession service, `org.mpris.MediaPlayer2.chromium.instanceN`), and music.apple.com sets `navigator.mediaSession`. Left on, the desktop shows two players and media keys can be double-handled (LOW, from training; verify in the spike). Disable in the engine: `--disable-features=HardwareMediaKeyHandling,MediaSessionService` and, if the engine allows, override `navigator.mediaSession` in the bridge to a no-op. The spike should check `busctl --user list | grep mpris` while playing.
- Keep MPRIS alive with the window closed (tray mode), as spotifast does through `fastframe-shell`.

## Audio path (PipeWire)

Audio never touches Rust. Chromium decodes (Widevine CDM, CDM-decrypted audio stays in the browser process path) and outputs through its audio service.

- On Linux Chromium builds use the PulseAudio client API (and ALSA fallback), not native PipeWire audio. On a PipeWire system `pipewire-pulse` provides that API, so the stream appears as a normal playback node in `pw-top`/`wpctl`. Native PipeWire in Chromium is used for screen capture only (LOW, training knowledge; verify with `pw-cli ls Node` during the spike).
- Consequence: no per-app code for PipeWire, no `cpal`/`fastframe-audio`, no gapless control, no EQ/visualizer (see dead weight above).
- Set a sensible stream identity so the mixer shows "Presto": pass `--audio-app-name`-style option if the engine has one, or set `PULSE_PROP_application.name=Presto` and `PULSE_PROP_media.role=music` in the engine's environment (PulseAudio client reads `PULSE_PROP_*`; LOW whether Chromium's stream overrides it).
- Volume: the app-level volume slider maps to `set_volume` (MusicKit `volume` 0..1) so it is per-stream, not the system sink. Do not touch system volume.
- Headless/CI: engine runs with `PULSE_SERVER` pointing nowhere; only the mock engine is used in tests.
- Output-device selection is a Chromium/Pulse matter (`setSinkId` in the page). Treat as a later optional capability, not a v1 item.

## Mock engine and demo mode

The mock is the second implementation of the engine side of the protocol, and the reason `presto-ipc` has a `Transport` trait.

- `presto-engine --mock` runs the mock over stdio (integration tests of the supervisor: kill it, hang it, make it emit a stale `rev`, answer slowly, return `rate_limited`).
- `presto --demo` creates an in-process transport pair and runs the same mock in a thread. No subprocess, no Chromium, no network. This is the equivalent of spotifast `--demo` but exercises the real core path (reducers, cache, optimistic overlay) instead of injecting state. Spotifast's approach (`set_offline` plus a 9k-line state injector) tests only the view layer.
- Mock content: a fixed JSON catalog under `presto-engine/src/mock/` (artists, albums, playlists, tracks with a fake artwork generated or bundled), plausible latency, a real clock advancing `position_ms`, queue semantics matching the MusicKit decision, failure injection via `--mock-fault=hang|crash|auth_expired|slow`.
- Screenshot and UI tests can reuse spotifast's `--demo-page`/`--demo-show` flags as thin options that select the first page and open panels, on top of the mock.
- The mock is the contract test for the protocol: the same scenario suite runs against mock and, manually or in a gated job, against the real engine.

## Patterns to Follow

### Pattern 1: Command/Event pair as the only seam
Same as spotifast `Backend`. UI emits `Action`, core turns it into `Command`, events return through `poll()`. No trait objects for the engine; the variability lives behind the `Transport` and the protocol.

### Pattern 2: Generation counters on every list request
Carry `generation: u64` in requests and echo in events so a slow page for a previous view is dropped. Spotifast does this on every `ApiRequest`.

### Pattern 3: Optimistic overlay with reconciliation by revision
UI applies the user's action instantly into an overlay; engine events carry a monotonically increasing `rev`/`seq`; an event older than the overlay's base is ignored and a `snapshot` is requested.

### Pattern 4: Supervisor owns lifecycle, client owns protocol
Supervisor knows processes, backoff and flap limits; EngineClient knows ids, timeouts and framing. Neither knows egui or MusicKit.

### Pattern 5: Bridge as versioned adapter
All MusicKit and Apple page specifics live in `bridge.js`. The Rust side sees only the protocol. When Apple changes the player, patch the file; `bridge_version` and `capabilities` tell the UI what still works.

## Anti-Patterns to Avoid

### Anti-Pattern 1: Rust-side queue plus MusicKit queue
Two queues drift on autoplay, station refill, and failed loads. Single owner, mirror only.

### Anti-Pattern 2: A `Player` trait with Spotify and Apple implementations
Spotifast has no such trait and the two services differ in what a track, a queue and a device are. Share the UI's Command/Event surface and models, not a generic backend trait.

### Anti-Pattern 3: Forwarding raw Apple JSON to the UI
Apple API shapes (relationships, `attributes`, includes) change per endpoint. Parse once in core into Presto models; keep the raw payload only in cache with a schema number.

### Anti-Pattern 4: Blocking the egui thread on IPC
Same AGENTS.md rule: all engine and cache work off the UI thread; events wake with `Waker`.

### Anti-Pattern 5: Hot-patch without a contract test
A runtime-loaded bridge with no self-check breaks silently. The bridge runs a startup self-test (MusicKit present, `api` reachable, event hooks bound) and reports `capabilities`.

### Anti-Pattern 6: Tokens crossing IPC "just for debugging"
Never. Redact `authorization` and music-user-token strings in the engine log capture.

## Scalability Considerations

| Concern | One user, 1k songs | 20k-song library | Very large / many playlists |
|---------|--------------------|-------------------|------------------------------|
| Library load | JSON snapshot, full page walk | Paginate in background, show first page immediately, incremental merge | Switch cache to SQLite; virtualized lists (spotifast uses egui row virtualization) |
| Artwork | Disk cache | LRU size cap (default 500 MB) | Same |
| IPC volume | Negligible | Pages of 100 items per request, serial per kind with a small concurrency cap (3) to avoid rate limits | Same, honor `retry_after` |
| Engine memory | One page, Chromium baseline of hundreds of MB | Same | Same; engine memory is the main footprint, so recycle the renderer on a day-long idle timer if leaks appear |

## Suggested build order (dependencies)

1. **presto-ipc types + protocol doc + mock engine** (no deps). Define frames, ops, errors, version. Unblocks everything and lets UI work proceed without Chromium.
2. **Engine feasibility spike** (needs only the `hello`/`req`/`res`/`evt` skeleton): Chromium host loads music.apple.com with Widevine, bridge injection, sign-in persistence, play a full track from a Rust command, proxied library call, event stream. Verify the LOW items: queue API surface, mediaSession/MPRIS duplication, audio node visibility, popup login. Stop for go-ahead (per PROJECT.md).
3. **presto-core Backend + EngineClient + Supervisor** against the mock, then the real engine. Includes handshake, timeouts, recovery tests with mock fault injection.
4. **Models + data layer + cache** (translation from Apple JSON, library walk, artwork).
5. **presto-ui port**: scaffolding with fastframe shell first (window, theme, i18n), then player bar and queue, then library/search/album/artist/playlist views. Port in the order the data layer delivers. Depends on 3 and 4, but views can start against the mock after step 1 if the models are fixed first.
6. **MPRIS and tray wiring** (needs playback events from step 3 and artwork files from step 4).
7. **Auth UX polish**: sign-in window reveal, expiry handling, sign-out (needs the real engine).
8. **Hardening**: hang/crash/reload recovery on the real engine, flap guard, bridge self-test and hot-reload path, resume after restart.
9. **Packaging**: engine payload, Widevine CDM provisioning (CDM is not redistributable; flag for AUR/Flatpak), desktop files, portable archive.

Roadmap flags: step 2 and step 8 need phase-specific research; step 9 needs a legal/distribution review before any public packaging; step 5 is large and should be split by view group.

## Sources

- crmne/spotifast v0.12.0 source (cloned and read): `src/backend.rs`, `src/player.rs`, `src/api/*`, `src/credentials.rs`, `src/images.rs`, `src/demo.rs`, `src/entrypoint.rs`, `Cargo.toml`, `AGENTS.md`, `docs/_reference/queue.md`, `PACKAGING.md`. HIGH
- crmne/fastframe v0.4.1 (`fastframe-now-playing`, `fastframe-shell` docs in source). HIGH
- MusicKit JS queue/playback API names (`setQueue`, `playNext`, `playLater`, `queueItemsDidChange`, `nextPlayableItem`, `music.api`): training knowledge, not verified against Apple's current docs in this pass. LOW
- Chromium audio backend (PulseAudio client, not native PipeWire) and MediaSession/MPRIS flags: training knowledge, unverified. LOW
- Engine choice (CEF vs castlabs Electron vs CDP Chrome) is out of this file's scope and is decided by the spike; the protocol is deliberately engine-neutral.

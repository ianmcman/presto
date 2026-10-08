# Phase 6: Desktop Integration - Context

**Gathered:** 2026-10-08
**Status:** Ready for planning

<domain>
## Phase Boundary

Presto behaves like a Linux desktop media player: MPRIS now-playing and controls, media keys via MPRIS, and a `presto` CLI that controls the running app and prints status. Covers DESK-01 and DESK-02. Tray, hide-on-close and a mini-player are out of scope (TRAY-01, v2). Packaging is Phase 7.

</domain>

<decisions>
## Implementation Decisions

### CLI transport and process model
- **D-01:** The CLI talks to the running app over its own control socket at `$XDG_RUNTIME_DIR/presto/ctl.sock`, NDJSON, same style as the engine IPC. Not MPRIS/D-Bus.
- **D-02:** With no running instance, every CLI command prints "presto is not running" and exits 1. No auto-launch.
- **D-03:** Plain `presto` with an instance already running raises/focuses the existing window via the control socket and the second process exits. One instance per profile (the engine profile lock forbids two).
- **D-04:** Clap subcommands; launching the GUI is the default. Existing `--demo`, `--fault`, `--engine-dir` stay on the GUI path. No separate `prestoctl` binary.

### Window lifecycle and media keys
- **D-05:** Closing the window quits the app (current Phase 5 `on_exit` shutdown). Media keys and MPRIS work while the process lives, including minimized.
- **D-06:** Hide-on-close and a tray icon are deferred to a later phase (TRAY-01). Do not add them here.
- **D-07:** Media keys are MPRIS only. No global-shortcuts portal. Phase 5 in-window keyboard map is unchanged.
- **D-08:** Presto is a plain MPRIS player: accurate PlaybackStatus, no stealing focus or pausing other players.

### MPRIS surface
- **D-09:** Full player: Metadata (trackid, title, artist, album, length, artUrl), PlaybackStatus, Position, Rate fixed 1.0, Volume, Shuffle, LoopStatus, CanSeek/Seek/SetPosition, Play/Pause/PlayPause/Stop/Next/Previous, root Raise/Quit, Identity "Presto".
- **D-10:** `mpris:artUrl` is a `file://` path into the Phase 4 artwork cache. The shell never fetches Apple's CDN.
- **D-11:** The bus name is owned from startup. When signed out, loading or the engine is restarting: PlaybackStatus Stopped, empty metadata, CanPlay false, control methods are no-ops. `busctl --user list | grep mpris` shows exactly one player.
- **D-12:** The engine's own MediaSession stays disabled (Phase 2 default switches); do not re-enable.

### CLI commands and status
- **D-13:** Subcommands: `play`, `pause`, `toggle`, `next`, `prev`, `stop`, `seek <time|+/-secs>`, `volume <0-100|+/-n>`, `shuffle [on|off]`, `repeat [off|all|one]`, `status`, `raise`, `quit`. No queue editing, search or play-by-id.
- **D-14:** `status --json` prints stable keys: `state` (playing/paused/stopped/loading), `position_ms`, `duration_ms`, `volume`, `shuffle`, `repeat`, `track {id,title,artist,album,artwork_path}`, `auth`, `engine` (ready/starting/restarting). Carries a `proto` version.
- **D-15:** Plain `status` prints one line, e.g. `Playing: Title - Artist (1:12/3:40)`, or `Stopped`.
- **D-16:** `status --json --watch` streams NDJSON, one line per change, via a subscribe op on the control socket.

### Claude's Discretion
- MPRIS crate choice: `fastframe-now-playing` (spotifast's, already resolvable) vs `mpris-server` + `zbus`. Researcher evaluates fit with the eframe event loop and Rust-owned queue mirror.
- Control socket frame schema, exit codes beyond 0/1, time/volume argument parsing details, stale socket cleanup.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements and roadmap
- `.planning/REQUIREMENTS.md` — DESK-01, DESK-02; TRAY-01 (v2, deferred)
- `.planning/ROADMAP.md` §Phase 6 — goal and success criteria

### Prior findings
- `.planning/phases/02-engine-feasibility-spike-gate/SPIKE-REPORT.md` §MPRIS — engine MediaSession must stay disabled; Presto registers its own name
- `docs/SPOTIFAST-SEAMS.md` — `RemoteAction` seam (backend.rs:88) is where media-key actions plug in; fastframe crates incl. `fastframe-instance`, `fastframe-now-playing` resolve at v0.4.1
- `docs/PROTOCOL.md` — engine IPC conventions (NDJSON, versioned) to mirror for the control socket
- `.planning/phases/05-playback-ui-and-demo-mode/05-CONTEXT.md` — D-06 (media keys deferred to this phase), v2 exclusions
- `.planning/phases/04-data-layer-and-cache/04-CONTEXT.md` — artwork cache location for `artUrl`

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/presto/src/cli.rs`: clap `Cli` struct (flat flags today); needs subcommands.
- `crates/presto/src/backend.rs`: UI-side Backend handle with command send and event poll; control socket and MPRIS commands route through the same send path.
- `crates/presto-core/src/mirror.rs`: Rust queue mirror and playback state, the source for MPRIS properties and `status`.
- `crates/presto-core/src/paths.rs`: runtime/cache dir resolution (socket path, artwork cache).
- `crates/presto-engine-mock`: lets MPRIS and CLI be tested under `--demo` without Widevine.

### Established Patterns
- NDJSON with a `proto` version and per-kind timeouts; schema snapshot and token-guard tests.
- Rust holds no Apple tokens; nothing in status/MPRIS may expose one.
- Pure helpers for path resolution; disposable caches.

### Integration Points
- App startup in `crates/presto/src/main.rs` / `launch.rs`: single-instance check and MPRIS name acquisition.
- `App::on_exit` shutdown: close socket and release the bus name.
- Window raise: control socket `raise` and MPRIS `Raise` both drive the egui viewport.

</code_context>

<specifics>
## Specific Ideas

- Waybar users get `status --json --watch` for a custom module; MPRIS covers the standard module.

</specifics>

<deferred>
## Deferred Ideas

- Tray icon with hide-on-close (TRAY-01; user wanted it, deferred to its own phase to keep Phase 6 scoped). Hide-on-close was to quit when not playing.
- Global shortcuts portal for media keys.
- Queue/search/play-by-id CLI commands.

</deferred>

---

*Phase: 06-desktop-integration*
*Context gathered: 2026-10-08*

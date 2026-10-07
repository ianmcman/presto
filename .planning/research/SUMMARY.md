# Project Research Summary

**Project:** Presto
**Domain:** Native Linux Apple Music client (Rust/egui UI over a hidden Widevine Chromium engine running music.apple.com)
**Researched:** 2026-10-07
**Confidence:** MEDIUM

## Executive Summary

Presto is a native egui front end ported from spotifast. Playback, auth and all Apple API access live in a hidden Chromium engine that runs the logged-in web player. A bridge script wraps the page's MusicKit instance and talks to Rust over NDJSON IPC. Rust holds no Apple tokens, never sees PCM, and proxies every API call through the page origin. The only existing proof of this shape on Linux is Sidra (castlabs Electron, hidden window, MusicKit event hook, MPRIS). Cider also uses castlabs Electron for Widevine.

Recommended approach: castlabs Electron (ECS) as the engine, system Chrome over CDP as fallback, CEF deferred. Keep a small engine-neutral IPC contract, a mock engine implementing the same protocol (demo mode and test harness), and MusicKit as the single queue owner with a read-only mirror in Rust. Reuse spotifast's Backend/Command/Event seam, fastframe crates, theming, i18n, artwork cache and CLI. Drop its audio-path features (EQ, visualizers, Winamp, sink): Rust never sees samples. That is the largest silent scope cut.

Main risks: (1) Widevine/CDM acceptance and false spike success from 30 s previews, (2) MusicKit being an unstable private surface, (3) hidden-window throttling and Wayland behavior, (4) Apple ToS and anti-automation exposure, (5) CDM redistribution blocking Flatpak/AppImage. The engine spike is a go/no-go gate for all five. Nothing downstream should be planned in detail until it reports.

## Key Findings

### Recommended Stack

Rust, eframe/egui 0.36 on the crmne/egui fork (rev `ba6790fe`), tokio, serde. Engine is a separate binary: castlabs ECS (e.g. `v44.1.0+wvcus`, tag unconfirmed) with `show:false`, a persistent profile under `$XDG_DATA_HOME/presto`, and `bridge.js` loaded from disk at runtime. Detail in STACK.md.

- castlabs Electron: engine, CDM auto-downloaded via component updater, proven by Sidra and Cider
- System Chrome + CDP: fallback engine, best licensing story, weaker hidden-window story
- cef-rs: deferred, CDM must be supplied, no Apple Music precedent
- egui/eframe 0.36 (fork pin): UI, shared with spotifast
- tokio + serde_json + tokio-util codec: async runtime and NDJSON framing
- reqwest: artwork CDN only, never api.music.apple.com
- fastframe-* (git tag v0.4.1): shell, tray, i18n, MPRIS (`fastframe-now-playing`), single instance

### Expected Features

Everything is bounded by what the web player's MusicKit and same-origin requests expose. Detail in FEATURES.md.

**Must have:** persistent sign-in and re-auth; playback controls, now-playing bar, queue, shuffle/repeat; library, search, album/artist/playlist pages, recently played, basic recommendations home; MPRIS and media keys; storefront awareness, unavailable-track handling, error/recovery states; artwork cache, library snapshots, theming, i18n.

**Should have:** demo mode on a mock engine (build early); CLI control and `status --json`; offline-ish browsing from snapshots; synced lyrics (if reachable), playlist editing, favorites; engine supervisor with resume after restart.

**Defer (v2+):** tray/mini-player, scrobbling, notifications, radio/stations (conflicts with queue decisions), macOS/Windows, video, podcasts, Flatpak.

**Never:** lossless/Atmos toggles, offline downloads, audio capture/EQ, token extraction, plugin marketplace, multi-account.

### Architecture Approach

Four crates plus a JS bridge. The UI sees only core models, `Command` and `Event`. Detail in ARCHITECTURE.md.

1. `presto-ipc`: protocol types, framing, version, `Transport` trait (stdio real, in-process for demo)
2. `presto-core`: Backend handle, EngineClient (request table, timeouts), Supervisor (backoff, ping, flap guard), caches, MPRIS adapter
3. `presto-ui`: ported spotifast views, Action to Command after frame, optimistic overlay
4. `presto-engine`: Chromium host, bridge loader, `--mock` mode
5. `bridge.js`: versioned MusicKit adapter with capability handshake and self-test

Patterns: generation counters on list requests, `rev`/`seq` reconciliation, MusicKit owns the queue (Rust mirrors, one atomic `set_queue` fallback), Apple JSON parsed once into Presto models, no token fields in any IPC type (schema test).

### Critical Pitfalls

1. **CDM not accepted, or preview-only false positive.** Spike must play a full >3 min track past 60 s and across a seek on a real subscriber account.
2. **MusicKit instance treated as stable.** Runtime-loaded bridge, capability handshake, scheduled smoke test, documented lookup fallback chain.
3. **Tokens leaking to Rust.** Proxy via page-origin `music.api`; schema test and log redaction.
4. **Anti-automation and ToS exposure.** Headed hidden window, no `--enable-automation`/webdriver, human sign-in only, personal-use stance recorded before the spike.
5. **Hidden-window throttling, Wayland quirks, double MPRIS.** Throttling flags, 2 h hidden playback test on Wayland and X11, disable Chromium MediaSession, check `busctl --user list | grep mpris`.

Also: engine RSS likely 300-800 MB (measure and state a budget); the CDM is never shipped in any artifact.

## Implications for Roadmap

1. **IPC Contract and Mock Engine.** No dependencies; fixes the engine-neutral contract. Delivers `presto-ipc`, protocol doc, `Transport`, mock engine with fault injection, demo mode.
2. **Engine Feasibility Spike (go/no-go).** castlabs ECS hosting music.apple.com, bridge injection, persistent sign-in, full-track playback from a Rust command, proxied library call, event stream. Measure RSS, hidden-window behavior (Wayland/X11), codec, queue API surface, MPRIS duplication, popup login, lyrics reachability. Fallback matrix against system Chrome. Stop for go-ahead.
3. **Core Backend, Supervisor, Auth.** Backend/EngineClient/Supervisor, timeouts, crash/hang/reload recovery, auth state machine, sign-in window reveal, expiry handling, queue mirror.
4. **Models, Data Layer, Cache.** Presto models, Apple JSON translation, paginated library walk, storefront, artwork cache, versioned snapshots.
5. **UI Port (split by view group).** Largest phase; `app.rs` (23k lines) is the coupling surface.
6. **MPRIS, Media Keys, CLI.** `fastframe-now-playing` fed from engine events, engine MediaSession disabled.
7. **Hardening.** Flap guard, bridge self-test and hot reload, resume after restart, 2 h soak, navigation allowlist, memory budget.
8. **Packaging.** AUR first (CDM fetched at runtime), portable archive; Flatpak and AppImage deferred pending legal review.

### Research Flags

Needs `/gsd:research-phase`: Phase 2 (MusicKit lookup, ECS tag, Wayland hidden window, queue API, ToS), Phase 7, Phase 8 (CDM licensing, castlabs/EVS terms, Flathub policy), light on Phase 4 (endpoint shapes from live traffic).

Standard patterns: Phases 1, 3, 5, 6.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | MEDIUM | Sidra and Cider support ECS; versions from READMEs; fastframe and fork availability not fully verified |
| Features | MEDIUM | Client norms solid; lyrics, library writes, radio, web codec unverified |
| Architecture | MEDIUM | spotifast/fastframe read from source (HIGH); MusicKit queue API and Chromium flags are LOW |
| Pitfalls | MEDIUM-LOW | Cider/CDM facts checked; Apple detection, ToS, Wayland, memory unverified |

**Overall confidence:** MEDIUM

### Gaps to Address

- **IPC transport conflict:** STACK.md says Unix socket, ARCHITECTURE.md says child stdio. Default suggestion: stdio (engine is always a supervised child); decide in Phase 1.
- **Seam as trait vs concrete:** follow ARCHITECTURE (shared Command/Event surface and models, no generic backend trait).
- **fastframe and egui fork availability:** confirm fetchable in Phase 1.
- **ECS tag:** v44.1.0 is a README example; confirm in the spike.
- **Web player codec and Atmos:** assume AAC, log the real stream, promise nothing.
- **Queue edit APIs:** remove, reorder, station queue readability unknown; capability flags decide UI.
- **Apple ToS and anti-automation:** read current Media Services terms before the spike.
- **Lyrics, playlist write, ratings endpoints:** verify against live traffic in the spike.

## Sources

### Primary (HIGH)
- crmne/spotifast v0.12.0 source; crmne/fastframe v0.4.1

### Secondary (MEDIUM)
- castlabs/electron-releases README and wiki; wimpysworld/sidra README; spotifast Cargo.toml and README; tauri-apps/cef-rs and CEF widevine_loader; AUR `electron*-castlab-bin`; Apple lossless support page; Cider press coverage

### Tertiary (LOW)
- MusicKit JS and Apple Music API shapes, Chromium audio/MediaSession flags, Wayland behavior, memory figures, Apple ToS posture, Flathub policy (training data or unverified)

---
*Research completed: 2026-10-07*
*Ready for roadmap: yes*

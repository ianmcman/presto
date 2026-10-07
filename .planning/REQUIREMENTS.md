# Requirements: Presto

**Defined:** 2026-10-07
**Core Value:** Full-catalog Apple Music playback and library browsing from a fast native Linux UI, without an Apple Developer account.

## v1 Requirements

### IPC and Mock Engine

- [x] **IPC-01**: Protocol crate defines versioned command, request/response and event messages as JSON, with request IDs and per-kind timeouts
- [x] **IPC-02**: No IPC type carries an Apple token field, enforced by a schema test
- [ ] **IPC-03**: A mock engine implements the same protocol, with fault injection (hang, crash, auth_expired, slow)
- [ ] **IPC-04**: User can launch `presto --demo` and use the UI with no account or Widevine

### Engine Spike

- [ ] **SPIKE-01**: Spike confirms music.apple.com exposes a usable MusicKit instance, or documents the alternative
- [ ] **SPIKE-02**: Sign-in through Apple's login page works in the engine and persists across engine restarts
- [ ] **SPIKE-03**: A command from a Rust process plays a full catalog track past 60 s and across a seek on a subscriber account
- [ ] **SPIKE-04**: A library API call (`/v1/me/library/playlists`) proxied through the page returns JSON to Rust
- [ ] **SPIKE-05**: Playback state events stream to Rust
- [ ] **SPIKE-06**: Spike measures idle and playing RSS, hidden-window behavior on Wayland and X11, stream codec, queue API surface, MPRIS duplication, and compares engine candidates with a recommendation

### Core and Auth

- [ ] **CORE-01**: Supervisor detects engine crash or hang, restarts with backoff, and restores queue and position
- [ ] **CORE-02**: Bridge script loads from a standalone file at runtime and reports version and capabilities in a handshake
- [ ] **CORE-03**: MusicKit owns the queue; Rust holds a read-only mirror reconciled by revision counter
- [ ] **AUTH-01**: User signs in once through Apple's own login flow shown in the engine window, then it is hidden
- [ ] **AUTH-02**: App detects an expired or signed-out session from engine events and offers re-auth, keeping cached data visible
- [ ] **AUTH-03**: Engine profile directory is created with mode 0700

### Data

- [ ] **DATA-01**: User can see library playlists, albums, artists and songs, paginated
- [ ] **DATA-02**: User can search the catalog
- [ ] **DATA-03**: User can see recently played and a recommendations home
- [ ] **DATA-04**: Storefront is read from the account and applied to catalog requests
- [ ] **DATA-05**: Proxy errors and rate limits surface as UI states
- [ ] **DATA-06**: Artwork and library snapshots are cached on disk so browsing works offline-ish

### Playback UI

- [ ] **PLAY-01**: User can play, pause, seek, skip, shuffle, repeat and set volume
- [ ] **PLAY-02**: User can see and use the queue
- [ ] **PLAY-03**: User can open album, artist and playlist pages and play from them
- [ ] **PLAY-04**: Unavailable tracks show a clear state

### Desktop Integration

- [ ] **DESK-01**: MPRIS exposes now-playing and controls from engine events; media keys work; engine's own MediaSession is disabled
- [ ] **DESK-02**: CLI controls playback and prints `status --json`

### Packaging

- [ ] **PKG-01**: AUR package builds with no Widevine CDM in any artifact; CDM fetched at runtime
- [ ] **PKG-02**: Distribution blockers (CDM licensing, ToS, Flathub policy) are documented

## v2 Requirements

- **LYR-01**: Synced lyrics, if the endpoint is reachable from the page
- **PLED-01**: Playlist editing and favorites, if library writes are reachable
- **TRAY-01**: Tray and mini-player
- **SCRB-01**: Scrobbling and notifications
- **RADIO-01**: Radio/stations (conflicts with queue ownership)
- **PLAT-01**: macOS and Windows
- **PKG-03**: Flatpak and AppImage

## Out of Scope

| Feature | Reason |
|---------|--------|
| DRM circumvention | Only Apple's web player with a legitimate CDM |
| Developer account, .p8, minted or extracted tokens | User constraint |
| Direct api.music.apple.com calls from Rust | Rust holds no tokens |
| WebKitGTK/wry engine | Cannot run Widevine |
| Lossless, Atmos, EQ, visualizer, Winamp UI | Rust never sees PCM; web player codec unconfirmed |
| Offline downloads, audio capture | DRM and scope |
| Spotify Connect, devices, social | Spotify-only |
| Video, podcasts, plugin marketplace, multi-account | Not core |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| IPC-01 | Phase 1 | Complete |
| IPC-02 | Phase 1 | Complete |
| IPC-03 | Phase 1 | Pending |
| IPC-04 | Phase 5 | Pending |
| CORE-01 | Phase 3 | Pending |
| CORE-02 | Phase 3 | Pending |
| CORE-03 | Phase 3 | Pending |
| AUTH-01 | Phase 3 | Pending |
| AUTH-02 | Phase 3 | Pending |
| AUTH-03 | Phase 3 | Pending |
| PLAY-01 | Phase 5 | Pending |
| PLAY-02 | Phase 5 | Pending |
| PLAY-03 | Phase 5 | Pending |
| PLAY-04 | Phase 5 | Pending |
| DESK-01 | Phase 6 | Pending |
| DESK-02 | Phase 6 | Pending |
| PKG-01 | Phase 7 | Pending |
| PKG-02 | Phase 7 | Pending |
| SPIKE-01 | Phase 2 | Pending |
| SPIKE-02 | Phase 2 | Pending |
| SPIKE-03 | Phase 2 | Pending |
| SPIKE-04 | Phase 2 | Pending |
| SPIKE-05 | Phase 2 | Pending |
| SPIKE-06 | Phase 2 | Pending |
| DATA-01 | Phase 4 | Pending |
| DATA-02 | Phase 4 | Pending |
| DATA-03 | Phase 4 | Pending |
| DATA-04 | Phase 4 | Pending |
| DATA-05 | Phase 4 | Pending |
| DATA-06 | Phase 4 | Pending |

**Coverage:**
- v1 requirements: 30 total
- Mapped to phases: 30
- Unmapped: 0

---
*Requirements defined: 2026-10-07*

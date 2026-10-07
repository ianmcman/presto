# Feature Research

**Domain:** Native desktop Apple Music client (Linux), UI over a hidden Widevine Chromium engine running music.apple.com
**Researched:** 2026-10-07
**Confidence:** MEDIUM (desktop-client norms from training data plus Cider/Apple docs; Apple web-player quality limits are LOW, search sources were weak and Apple's lossless page does not list the web player)

## Hard constraint that shapes everything

Every feature is limited to what the logged-in web player's MusicKit JS instance and its page-origin API calls expose. Rust holds no tokens. So "feature X" means "bridge script can reach X through MusicKit or a proxied same-origin request". Anything that needs a developer token outside the page is out.

## Feature Landscape

### Table Stakes (Users Expect These)

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Sign-in via Apple's own flow, session persists | Nothing works without it | MEDIUM | Engine window shown only at sign-in; profile dir persists cookies. Re-auth expiry must be detected and surfaced |
| Play/pause/next/prev/seek/volume | Basic player | LOW | Commands to MusicKit; state events back |
| Queue view (play next, add to queue, reorder, remove, clear) | Every client has it | HIGH | Depends on queue-owner decision. MusicKit queue API is limited; Rust-owned queue avoids desync but needs per-track `setQueue` handoff |
| Shuffle and repeat (off/one/all) | Standard | LOW | MusicKit has shuffleMode and repeatMode |
| Now-playing bar with artwork, progress, time | Standard | LOW | Event-driven from engine |
| Library: songs, albums, artists, playlists, recently added | Core value | MEDIUM | Paged `/v1/me/library/*` via proxy; large libraries need pagination and snapshot cache |
| Playlists: view, play, create, add/remove tracks, rename | Standard | MEDIUM | Library write endpoints exist in the web player; verify create/edit in spike |
| Search (catalog + library), with suggestions | Standard | MEDIUM | Catalog `/v1/catalog/{sf}/search` and hints, library search. Debounce, cancel stale requests |
| Album, artist, playlist detail pages | Standard | MEDIUM | Artist page: top songs, albums, related |
| Recently played | Standard | LOW | `/v1/me/recent/played` |
| Recommendations / Listen Now / Browse | Apple's home surface | MEDIUM | `/v1/me/recommendations` returns heterogenous groups; render a subset (albums/playlists/stations), skip exotic types |
| Add to library / remove, favorite (love) | Standard | LOW | Library add and ratings endpoints |
| Media keys + MPRIS (metadata, playback status, position, Seek, SetPosition, volume, shuffle/loop, art URL) | Linux desktop baseline; waybar/KDE/GNOME/playerctl | MEDIUM | Driven from engine events only. `mpris:artUrl` should point at the local cache file. Position needs Seeked signals on jumps |
| Storefront awareness | Catalog differs by country; wrong storefront gives 404s/empty | LOW | Read the user's storefront from the account, never hardcode `us` |
| Explicit/unavailable track handling (greyed out, skip) | Region and licensing gaps are common | LOW | Respect `playParams` / `isPlayable` flags |
| Loading, error, offline states | Engine crash/hang/token expiry are normal | MEDIUM | Needs IPC recovery states in UI. Never a blank screen |
| Keyboard navigation and in-app shortcuts | Desktop expectation | LOW | Inherited from spotifast |
| Theming and i18n | Already in spotifast | LOW | Port, not build |
| Artwork cache (disk, LRU) | Fast scrolling | LOW | Keyed by URL template + size |
| Settings (cache size, engine path, theme, language) | Standard | LOW | |

### Differentiators (Competitive Advantage)

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Native egui speed and low RAM vs Electron (Cider, web player) | The reason presto exists | MEDIUM | Engine still costs a Chromium; claim only that UI is native. Keep engine hidden and idle-throttled |
| CLI control (`presto play`, `next`, `status --json`) | Scriptable, fits Linux workflow; spotifast has it | LOW | Port; talks to running instance over a socket |
| Demo mode with mock engine (`--demo`) | Dev, CI, screenshots, no account needed | MEDIUM | Mock engine speaks same IPC. Also the test harness for UI. Build early |
| Offline-ish browsing (library snapshot + artwork cache) | Launch instantly, browse with no network or with engine down | MEDIUM | Snapshot per list with timestamp; refresh in background; mark stale. Playback still needs network (no downloads) |
| Synced lyrics (time-synced, line highlight) | Cider and Apple apps have it; strong daily value | MEDIUM | Web player fetches lyrics via `/v1/catalog/{sf}/songs/{id}/lyrics` (syllable/line TTML). Verify reachability in spike. Hide panel when absent |
| Waybar/status-bar friendly output | Linux power-user draw | LOW | Falls out of CLI `status --json` and MPRIS |
| Last.fm / ListenBrainz scrobbling | Cider has Last.fm; Apple has none | MEDIUM | Driven by playback events. Optional, P3 |
| Desktop notifications on track change | Cider does it | LOW | Optional setting |
| Mini-player / tray | Cider has tray | MEDIUM | egui multi-viewport and tray on Wayland are inconsistent. Defer |
| Resilient engine supervisor (auto-restart, resume position) | Hybrid design's main risk becomes invisible | HIGH | Part of IPC phase, not UI |
| Radio / stations (Apple Music 1, genre/artist stations) | Apple differentiator | MEDIUM | Station queue is endless and MusicKit-owned; conflicts with Rust-owned queue |

### Anti-Features (Commonly Requested, Often Problematic)

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Lossless / Hi-Res / Dolby Atmos claims or toggles | Apple Music's headline feature | Apple's lossless support page lists Apple devices, Android, Windows and does not list the web player. Web player is understood to serve lossy AAC via Widevine (LOW confidence, unverified in spike). Cider's spatial audio and EQ do not translate to a hidden-web-player design | Show "AAC via web player" honestly. Spike should log the actual stream codec/bitrate. Do not build a toggle |
| Offline downloads / saved-for-offline | Spotify and Apple apps have it | Needs DRM key storage outside the web player; edges into circumvention | Offline-ish browsing only (metadata + art) |
| Local audio capture, ripping, or EQ on decoded stream | Audiophile asks | Audio is inside Chromium's protected pipeline; capture is circumvention. Rust never sees PCM | Use system-level EQ (PipeWire/EasyEffects) |
| Direct api.music.apple.com calls from Rust or token extraction | Faster, no page dependency | Violates project constraints and ToS posture | Proxy requests through the page origin |
| Own recommendation engine | Better than Apple's | Needs listening data and effort for no core-value gain | Render Apple's recommendations |
| Music video playback | Cider supports video | Adds a second render path and Chromium window handling | Defer; maybe open in engine window later |
| Podcasts | Cider supports | Separate API surface and UI | Out of v1 |
| Social features, Apple Music Connect, shared listening | Spotify has Jam/Connect | No web API path; Connect is librespot-specific in spotifast | Drop spotifast's Connect views |
| Plugin/theme marketplace (Cider-style) | Customization | Large surface, security burden | Plain theme files |
| Multi-account | Occasional ask | Separate engine profiles, state, cache namespaces | One account; profile dir is the account |
| Full macOS/Windows parity in v1 | Reach | Per PROJECT.md out of scope | Keep paths XDG-isolated behind small traits |
| Custom window chrome for the engine | Make engine feel native | Engine should stay invisible | Show only for sign-in/CAPTCHA/consent |

## Feature Dependencies

```
Sign-in + persistent profile
    └──requires──> Engine host + bridge + IPC (commands, proxied requests, events)
                       ├──requires──> Playback control ──> Queue owner decision
                       │                                       └──requires──> Queue UI, shuffle/repeat
                       ├──requires──> Library/search/browse proxy
                       │                  └──requires──> Storefront resolution
                       │                  └──enhances──> Library snapshot + artwork cache (offline-ish)
                       └──requires──> Event stream ──> MPRIS, notifications, scrobbling, CLI status

Lyrics ──requires──> Playback position events + lyrics endpoint reachable
Radio/stations ──conflicts──> Rust-owned queue
Demo mode ──requires──> IPC protocol defined (mock implements it); enhances all UI work
CLI control ──requires──> Single instance + local socket
```

### Dependency Notes

- **Everything requires the spike result:** reachability of MusicKit, library proxy, lyrics endpoint, and codec facts decide scope.
- **Queue UI requires queue-owner decision:** Rust-owned gives deterministic UI and MPRIS but loses MusicKit-native radio and autoplay. MusicKit-owned keeps those but queue mutation APIs are thin. Decide before any queue UI work.
- **Demo mode requires a frozen IPC schema:** build the mock right after the protocol so UI work does not need a live account.
- **Offline-ish browsing requires a stable library data model:** snapshot format should be versioned from the first write.
- **MPRIS requires events, not polling:** position/seek correctness depends on the event stream.

## MVP Definition

### Launch With (v1)

- [ ] Engine spike outcome: sign-in, play a full track from Rust, proxied library call, events back
- [ ] IPC protocol plus mock engine / demo mode
- [ ] Playback controls, now-playing bar, queue, shuffle/repeat
- [ ] Library (songs/albums/artists/playlists), search, album/artist/playlist pages
- [ ] Recently played and a basic recommendations home
- [ ] MPRIS + media keys
- [ ] Artwork cache and library snapshots
- [ ] Storefront handling, error/recovery states
- [ ] Port of theming, i18n, CLI control from spotifast

### Add After Validation (v1.x)

- [ ] Synced lyrics, once the endpoint is confirmed reachable
- [ ] Playlist editing (create/add/remove) if not in v1 cut
- [ ] Favorites/ratings, add-to-library
- [ ] Notifications, scrobbling
- [ ] Stations/radio, tied to queue-owner choice

### Future Consideration (v2+)

- [ ] Tray / mini-player
- [ ] macOS/Windows
- [ ] Video, podcasts
- [ ] Packaging beyond AUR/AppImage (Flatpak blocked on CDM redistribution questions)

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Playback + queue | HIGH | HIGH | P1 |
| Library + search + detail pages | HIGH | MEDIUM | P1 |
| MPRIS/media keys | HIGH | MEDIUM | P1 |
| Demo mode | MEDIUM | MEDIUM | P1 |
| Artwork/library cache | HIGH | LOW | P1 |
| Recommendations home | MEDIUM | MEDIUM | P1 (basic) |
| Playlist editing | MEDIUM | MEDIUM | P2 |
| Synced lyrics | MEDIUM | MEDIUM | P2 |
| CLI control | MEDIUM | LOW | P2 (port) |
| Scrobbling | LOW | MEDIUM | P3 |
| Notifications | LOW | LOW | P3 |
| Tray/mini-player | LOW | MEDIUM | P3 |

## Competitor Feature Analysis

| Feature | Cider (Electron/Rust hybrid) | Apple Music web player | spotifast | Our Approach |
|---------|------------------------------|------------------------|-----------|--------------|
| Library/playlists | Yes, synced to account | Yes | Yes (Spotify) | Native views over proxied API |
| Lyrics | Panel, live lyrics | Time-synced lyrics | Via separate source | Apple's lyrics via bridge, if reachable |
| MPRIS | Yes | Browser's Media Session only | Yes | Engine-event driven |
| Spatial/EQ | Advertised | Spatial availability unverified | n/a | Not promised |
| Plugins/themes | Yes | No | Themes | Themes only |
| Scrobbling | Last.fm | No | Optional | Later, optional |
| Resource use | Heavy | Browser tab | Light | Light UI, one hidden engine |
| Demo mode | No | No | `--demo` | Mock engine |

## Sources

- Cider overview and features (MPRIS, lyrics, Last.fm, themes, plugins): https://www.omgubuntu.co.uk/2022/07/cider-is-an-open-source-apple-music-client-for-linux-desktops, https://en.ubunlog.com/cider-available-linux-windows/ (MEDIUM, dated press)
- Apple lossless support page, web player not listed: https://support.apple.com/en-us/118295 (MEDIUM for "not listed", LOW for any claim about actual web codec)
- Web-player codec behavior: low-quality aggregator pages only (LOW). Verify in spike by inspecting the playing stream.
- Apple Music API endpoint shapes (library, recent, recommendations, lyrics, storefront): training data (LOW-MEDIUM). Verify against live web-player network traffic during the spike.
- spotifast feature set: PROJECT.md description (not re-read from repo here).

## Gaps

- Whether lyrics, library writes, and radio are reachable through page-origin requests is unverified.
- Actual web-player codec/bitrate and any Atmos availability unverified.
- Cider 2026 current feature list and its engine approach not confirmed; check its repo before the engine decision.

---
*Feature research for: native Apple Music desktop client (Linux)*
*Researched: 2026-10-07*

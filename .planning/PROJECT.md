# Presto

## What This Is

A native, lightweight Apple Music desktop client for Linux (primary), with macOS/Windows as later nice-to-haves. Rust + egui UI structured after crmne/spotifast (MIT), with playback and library access delegated to a hidden Widevine-capable Chromium engine that runs Apple's own web player. Personal use first.

## Core Value

Full-catalog Apple Music playback and library browsing from a fast native Linux UI, without an Apple Developer account.

## Requirements

### Validated

- Spotifast seams documented (`docs/SPOTIFAST-SEAMS.md`). Validated in Phase 1: IPC Contract and Mock Engine
- IPC wire contract, version handshake, token-name guard and fault-injecting mock engine. Validated in Phase 1: IPC Contract and Mock Engine

### Active

- [ ] Engine feasibility spike proves: MusicKit instance reachable, sign-in persists, full-track playback from a Rust command, library API proxied to Rust, state events streamed back
- [ ] Native egui UI ported from spotifast (views, theming, i18n, CLI control, MPRIS)
- [ ] presto-engine hosts music.apple.com in a hidden Widevine Chromium engine and injects a runtime-loaded bridge script
- [ ] JSON IPC protocol (commands, proxied requests with IDs/timeouts, events, versioning, crash/hang/reload recovery)
- [ ] Single queue owner (decided in planning) to avoid desync
- [ ] Sign-in via Apple's own login flow; session persists in the engine profile
- [ ] Library, playlists, search, albums, artists, recently played, recommendations, lyrics (if available) via the proxy
- [ ] Local cache (artwork, library snapshots) for fast, offline-ish browsing
- [ ] MPRIS driven by engine events (media keys, KDE/GNOME/waybar)
- [ ] Demo mode with a mock engine (equivalent to spotifast `--demo`)
- [ ] Packaging (AUR, AppImage, Flatpak where feasible)

### Out of Scope

- DRM circumvention of any kind — only Apple's web player with a legitimate Widevine CDM
- MusicKit .p8 key, self-minted developer token, or Apple Developer account — user does not have or want one
- Extracting or reusing Apple's developer token outside music.apple.com's origin
- Direct calls to api.music.apple.com from the Rust process; Rust holds no Apple tokens
- WebKitGTK/wry as the engine — cannot run Widevine
- macOS/Windows support in v1 — nice-to-have later
- Public distribution in v1 — personal use first; blockers get flagged

## Context

- Reference: https://github.com/crmne/spotifast, a Rust + egui Spotify client using a forked librespot (auth, Connect, playback) and reqwest (Web API), with a fastframe shell, MPRIS, CLI control, keyring, i18n, caching.
- Seams where presto-engine plugs in: spotifast's playback interface and API-client interface (to be documented in Step 1).
- Engine candidates: CEF (cef-rs or C++ host) with libwidevinecdm.so; castlabs Electron (ECS) hidden; system Chrome/Chromium via CDP with a dedicated profile; others to be found. Check what Cider and similar projects do in 2026.
- Bridge script lives as a standalone runtime-loaded file so it can be patched when Apple changes the web player without rebuilding.
- Test environment: modern Linux, Wayland and X11, PipeWire.

## Constraints

- **Tech stack**: Rust stable, egui/eframe matching spotifast versions unless justified — shared UI code with the reference
- **Architecture**: Two processes, native `presto` and replaceable `presto-engine`, JSON IPC between them — engine is an implementation detail, never visible except possibly at sign-in
- **Auth**: No developer account, no .p8, no extracted tokens — user constraint
- **Secrets**: None in repo or binary
- **Workflow**: Planning only until the user approves the plan; engine spike is planned first, then stops for go-ahead
- **Distribution**: Personal use first; flag anything blocking Flatpak/AUR (CDM redistribution, ToS)

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Hybrid native UI + hidden Chromium engine | Only legitimate full-playback path on Linux is the web player with Widevine | — Pending |
| Rust holds no Apple tokens | Avoid token extraction and keep the trust boundary at the page origin | — Pending |
| Runtime-loaded bridge script | Apple web player changes should not need a rebuild | — Pending |
| Engine choice (CEF / castlabs Electron / CDP Chrome) | Decided by the feasibility spike | — Pending |
| Queue owner (Rust vs MusicKit) | Decided in full plan to avoid two sources of truth | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd:transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd:complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-10-07 after Phase 1*

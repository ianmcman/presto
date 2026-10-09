# Presto

## What This Is

A native, lightweight Apple Music desktop client for Linux (primary), with macOS/Windows as later nice-to-haves. Rust + egui UI structured after crmne/spotifast (MIT), with playback and library access delegated to a hidden Widevine-capable Chromium engine that runs Apple's own web player. Personal use first.

## Core Value

Full-catalog Apple Music playback and library browsing from a fast native Linux UI, without an Apple Developer account.

## Requirements

### Validated

- ✓ Spotifast seams documented (`docs/SPOTIFAST-SEAMS.md`) (v1.0)
- ✓ IPC wire contract, version handshake, token-name guard and fault-injecting mock engine (v1.0)
- ✓ Engine feasibility spike: go on castlabs ECS v44.5.1+wvcus (v1.0)
- ✓ Supervised hidden engine with recovery, runtime bridge, queue mirror, Apple sign-in that persists (v1.0)
- ✓ Data layer: ApiClient, SQLite page cache, artwork cache, search, offline serving, sign-out wipe (v1.0)
- ✓ Native egui UI with demo mode (`presto --demo`) (v1.0)
- ✓ MPRIS, media keys, `presto <subcommand>` control socket, single instance (v1.0)
- ✓ Arch packaging (`presto-git`, `presto-bin`) with the CDM fetched at runtime; distribution blockers documented (v1.0)

### Active

- [ ] Publish to the AUR
- [ ] AppImage and Flatpak packaging (blocked or deferred per `docs/DISTRIBUTION.md`)
- [ ] Lyrics, if the web player exposes them
- [ ] Re-check Apple, Flathub and castlabs terms against the live pages
- [ ] Offline first-run behavior verified (v1.0 checklist item 7)

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
| Hybrid native UI + hidden Chromium engine | Only legitimate full-playback path on Linux is the web player with Widevine | ✓ Good |
| Rust holds no Apple tokens | Avoid token extraction and keep the trust boundary at the page origin | ✓ Good |
| Runtime-loaded bridge script | Apple web player changes should not need a rebuild | ✓ Good |
| Engine choice (CEF / castlabs Electron / CDP Chrome) | Decided by the feasibility spike | ✓ Good: castlabs ECS v44.5.1+wvcus |
| Queue owner (Rust vs MusicKit) | Decided in full plan to avoid two sources of truth | ✓ Good |
| CDM downloaded at first run, never shipped | Widevine cannot be redistributed | ✓ Good |
| Prebuilt `presto-bin` from a GitHub release | Source build takes about 9 minutes | ✓ Good |

## Current State

v1.0 shipped 2026-10-09: about 17.9k lines of Rust, 0.7k of JavaScript. Installs on Arch via `presto-bin` (no compile) or `presto-git`. Release v0.1.0 is public. Known gaps are listed in `.planning/MILESTONES.md`.

## Next Milestone Goals

AUR publication, packaging beyond Arch if the blockers allow, and any playback bugs found in daily use.

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
*Last updated: 2026-10-09 after v1.0 milestone*

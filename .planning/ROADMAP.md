# Roadmap: Presto

## Overview

Fix the engine-neutral IPC contract and a mock engine first, then run the engine feasibility spike as a go/no-go gate. Phases 3 to 7 are outlined only; they get detailed planning after the spike is approved and its findings (engine choice, queue API surface, hidden-window behavior) are folded back in.

## Phases

- [ ] **Phase 1: IPC Contract and Mock Engine** - Versioned JSON protocol, token-free schema, mock engine with fault injection, spotifast seam notes
- [ ] **Phase 2: Engine Feasibility Spike (GATE)** - Prove full-track playback, persistent sign-in, proxied library calls and events from a hidden Widevine engine; stop for approval
- [ ] **Phase 3: Core Backend, Supervisor, Auth** - Engine supervision and recovery, runtime bridge handshake, queue mirror, sign-in and re-auth
- [ ] **Phase 4: Data Layer and Cache** - Library, search, home, storefront, error states, disk caches
- [ ] **Phase 5: Playback UI and Demo Mode** - Ported egui views and playback controls, `presto --demo`
- [ ] **Phase 6: Desktop Integration** - MPRIS, media keys, CLI
- [ ] **Phase 7: Packaging and Distribution Notes** - AUR package with runtime CDM fetch, documented blockers

## Phase Details

### Phase 1: IPC Contract and Mock Engine
**Goal**: A stable, engine-neutral protocol exists and a mock engine speaks it, so everything downstream can be built and tested without Widevine.
**Depends on**: Nothing (first phase)
**Requirements**: IPC-01, IPC-02, IPC-03
**Success Criteria** (what must be TRUE):
  1. A developer can read one protocol doc and the `presto-ipc` crate and see every command, request/response and event with a version and per-kind timeout.
  2. A schema test fails if any IPC type gains a token-like field.
  3. The mock engine answers the full protocol over the chosen transport, and each fault (hang, crash, auth_expired, slow) can be triggered on demand.
  4. A short document records the spotifast Backend/Command/Event and API-client seams the UI depends on, and confirms the egui fork and fastframe crates are fetchable.
**Plans**: 4 plans

Plans:
- [x] 01-01-PLAN.md: Toolchain, workspace, presto-ipc wire types, timeouts, socket transport (wave 1)
- [x] 01-02-PLAN.md: Schema snapshot and token guard, PROTOCOL.md with coverage test, spotifast seam notes (wave 2)
- [x] 01-03-PLAN.md: Mock engine: catalog, playback clock, protocol loop and integration test (wave 2)
- [x] 01-04-PLAN.md: Mock fault injection (hang, crash, auth_expired, slow) by flag and live frame (wave 3)

### Phase 2: Engine Feasibility Spike (GATE)
**Goal**: Decide go/no-go on the hidden-Chromium approach and pick the engine, with measured evidence. Work stops here for user approval before Phases 3 to 7 are planned in detail.
**Depends on**: Phase 1
**Requirements**: SPIKE-01, SPIKE-02, SPIKE-03, SPIKE-04, SPIKE-05, SPIKE-06
**Success Criteria** (what must be TRUE):
  1. The user signs in through Apple's login page in the engine, restarts the engine, and is still signed in.
  2. A command sent from a Rust process plays a full catalog track on a subscriber account past 60 s and across a seek.
  3. A Rust process calls `/v1/me/library/playlists` through the page and receives JSON, with no Apple token visible to Rust.
  4. Play, pause and progress events arrive in Rust while a track plays.
  5. A written report gives idle and playing RSS, hidden-window behavior on Wayland and X11, stream codec, queue API surface, MPRIS duplication, a candidate comparison, and a go/no-go recommendation for the user to approve.
**Plans**: 6 plans

Plans:
- [x] 02-01-PLAN.md: Terms gate (D-10), install castlabs ECS at the newest wvcus tag (wave 1)
- [x] 02-02-PLAN.md: presto-spike Rust driver (checklist, signin, measure) proven against the mock (wave 1)
- [x] 02-03-PLAN.md: Engine main.js, preload.js, bridge.js with stub-MusicKit tests (wave 2)
- [x] 02-04-PLAN.md: Live run: smoke, hand sign-in, hidden restart checklist on Wayland (wave 3)
- [x] 02-05-PLAN.md: Measurements: hidden-window and MPRIS matrix, RSS, codec, queue surface, listening check (wave 4)
- [ ] 02-06-PLAN.md: SPIKE-REPORT.md, D-15 fallback check, go/no-go approval (wave 5)

### Phase 3: Core Backend, Supervisor, Auth
**Goal**: The app runs a supervised engine reliably and the user can sign in once and stay signed in. (Outline; detail after spike approval.)
**Depends on**: Phase 2 approved
**Requirements**: CORE-01, CORE-02, CORE-03, AUTH-01, AUTH-02, AUTH-03
**Success Criteria** (what must be TRUE):
  1. After killing or hanging the engine, playback resumes at the same queue and position after a backoff restart.
  2. Editing the bridge script on disk and restarting the engine changes behavior with no rebuild, and the handshake shows its version and capabilities.
  3. First launch shows Apple's login in the engine window, which hides after sign-in and stays hidden on later launches.
  4. When the session expires, the UI offers re-auth while cached data stays visible.
  5. The queue shown in Rust matches MusicKit's queue after rapid changes, and the profile directory has mode 0700.
**Plans**: TBD

### Phase 4: Data Layer and Cache
**Goal**: Library and catalog data flows from the engine into Presto models and is cached for fast, offline-ish browsing. (Outline.)
**Depends on**: Phase 3
**Requirements**: DATA-01, DATA-02, DATA-03, DATA-04, DATA-05, DATA-06
**Success Criteria** (what must be TRUE):
  1. User can page through library playlists, albums, artists and songs.
  2. User can search the catalog and get results in the account's storefront.
  3. User sees recently played and a recommendations home.
  4. A rate limit or proxy error shows a visible UI state instead of a blank view.
  5. With the engine offline, previously viewed library pages and artwork still display.
**Plans**: TBD

### Phase 5: Playback UI and Demo Mode
**Goal**: The ported egui UI lets the user browse and play, and runs against the mock engine with no account. (Outline; largest phase, may split by view group.)
**Depends on**: Phase 4
**Requirements**: PLAY-01, PLAY-02, PLAY-03, PLAY-04, IPC-04
**Success Criteria** (what must be TRUE):
  1. User can play, pause, seek, skip, shuffle, repeat and set volume from the UI.
  2. User can view the queue and play any item in it.
  3. User can open album, artist and playlist pages and start playback from them.
  4. An unavailable track shows a clear state instead of failing silently.
  5. `presto --demo` launches and every view works with no account and no Widevine.
**Plans**: TBD

### Phase 6: Desktop Integration
**Goal**: Presto behaves like a Linux desktop media player. (Outline.)
**Depends on**: Phase 5
**Requirements**: DESK-01, DESK-02
**Success Criteria** (what must be TRUE):
  1. KDE/GNOME/waybar show now-playing and media keys control playback.
  2. `busctl --user list | grep mpris` shows exactly one player.
  3. `presto` CLI commands control playback and `status --json` prints current state.
**Plans**: TBD

### Phase 7: Packaging and Distribution Notes
**Goal**: Presto installs on Arch with no CDM bundled, and distribution blockers are written down. (Outline.)
**Depends on**: Phase 6
**Requirements**: PKG-01, PKG-02
**Success Criteria** (what must be TRUE):
  1. The AUR package builds and runs on a clean Arch system, and no artifact contains libwidevinecdm.
  2. First run fetches the CDM at runtime and playback works.
  3. A document lists CDM licensing, Apple ToS and Flathub policy blockers.
**Plans**: TBD

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. IPC Contract and Mock Engine | 0/4 | Not started | - |
| 2. Engine Feasibility Spike (GATE) | 0/6 | Not started | - |
| 3. Core Backend, Supervisor, Auth | 0/TBD | Not started | - |
| 4. Data Layer and Cache | 0/TBD | Not started | - |
| 5. Playback UI and Demo Mode | 0/TBD | Not started | - |
| 6. Desktop Integration | 0/TBD | Not started | - |
| 7. Packaging and Distribution Notes | 0/TBD | Not started | - |

# Roadmap: Presto

## Overview

Fix the engine-neutral IPC contract and a mock engine first, then run the engine feasibility spike as a go/no-go gate. Phases 3 to 7 are outlined only; they get detailed planning after the spike is approved and its findings (engine choice, queue API surface, hidden-window behavior) are folded back in.

## Phases

- [ ] **Phase 1: IPC Contract and Mock Engine** - Versioned JSON protocol, token-free schema, mock engine with fault injection, spotifast seam notes
- [x] **Phase 2: Engine Feasibility Spike (GATE)** - Prove full-track playback, persistent sign-in, proxied library calls and events from a hidden Widevine engine; stop for approval (completed 2026-10-08)
- [x] **Phase 3: Core Backend, Supervisor, Auth** - Engine supervision and recovery, runtime bridge handshake, queue mirror, sign-in and re-auth
- [ ] **Phase 4: Data Layer and Cache** - Library, search, home, storefront, error states, disk caches
- [x] **Phase 5: Playback UI and Demo Mode** - Ported egui views and playback controls, `presto --demo` (completed 2026-10-08)
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
- [x] 02-06-PLAN.md: SPIKE-REPORT.md, D-15 fallback check, go/no-go approval (wave 5)

### Phase 3: Core Backend, Supervisor, Auth
**Goal**: The app runs a supervised engine reliably and the user can sign in once and stay signed in.
**Depends on**: Phase 2 approved
**Requirements**: CORE-01, CORE-02, CORE-03, AUTH-01, AUTH-02, AUTH-03
**Success Criteria** (what must be TRUE):
  1. After killing or hanging the engine, playback resumes at the same queue and position after a backoff restart.
  2. Editing the bridge script on disk and restarting the engine changes behavior with no rebuild, and the handshake shows its version and capabilities.
  3. First launch shows Apple's login in the engine window, which hides after sign-in and stays hidden on later launches.
  4. When the session expires, the UI offers re-auth while cached data stays visible.
  5. The queue shown in Rust matches MusicKit's queue after rapid changes, and the profile directory has mode 0700.
**Plans**: 15 plans

Plans:
- [x] 03-01-PLAN.md: Protocol 1.1 (bridge_ready, show_window, set_queue play) and mock support (wave 1)
- [x] 03-02-PLAN.md: presto-core crate: 0700 paths, pidfile sweep, backoff (wave 1)
- [x] 03-03-PLAN.md: Engine: bridge path override, bridge_ready, Rust-driven window, close-to-hide, hard exit (wave 1)
- [x] 03-04-PLAN.md: Supervisor actor: spawn, handshake, heartbeat, backoff restart, drift (wave 2)
- [x] 03-05-PLAN.md: Queue mirror, auth gate, restore after crash and re-auth (wave 3)
- [x] 03-06-PLAN.md: Live driver and manual checklist on the real engine (wave 4)
- [x] 03-07-PLAN.md: Gap: restore ends with Play/Pause and verified seek, quirky-mock tests (wave 1)
- [x] 03-08-PLAN.md: Gap: re-run live crash/hang checklist C and D (wave 2)
- [x] 03-09-PLAN.md: Gap: mock autoplay-after-load, restore waits out loading and holds paused (wave 1)
- [x] 03-10-PLAN.md: Gap: live re-run of C2, C3, D; mark CORE-01 on pass (wave 2)
- [x] 03-11-PLAN.md: Gap: mute engine during load restore, mock audibility tests (wave 1)
- [x] 03-12-PLAN.md: Gap: live re-run of C1, C2, C3, D; mark CORE-01 on pass (wave 2)
- [x] 03-13-PLAN.md: Gap: diagnose live restore Seek timeout, mock unanswered seek, RED tests (wave 1)
- [x] 03-14-PLAN.md: Gap: restore survives a Seek error, Pause before unmute on every exit (wave 2)
- [x] 03-15-PLAN.md: Gap: live re-run of C1, C2, C3 at track start, D (wave 3)

### Phase 4: Data Layer and Cache
**Goal**: Library and catalog data flows from the engine into Presto models and is cached for fast, offline-ish browsing.
**Depends on**: Phase 3
**Requirements**: DATA-01, DATA-02, DATA-03, DATA-04, DATA-05, DATA-06
**Success Criteria** (what must be TRUE):
  1. User can page through library playlists, albums, artists and songs.
  2. User can search the catalog and get results in the account's storefront.
  3. User sees recently played and a recommendations home.
  4. A rate limit or proxy error shows a visible UI state instead of a blank view.
  5. With the engine offline, previously viewed library pages and artwork still display.
**Plans**: 9 plans

Plans:
- [x] 04-01-PLAN.md: Mock rate_limited and signed_out faults, Home/search routes, storefront check, 10k library flag (wave 1)
- [x] 04-02-PLAN.md: Live probe for server-side sort, limits and shapes; user confirms (wave 1)
- [x] 04-03-PLAN.md: Deps, cache paths and install id, data module tree, UiErrorKind, SQLite store (wave 1)
- [x] 04-04-PLAN.md: ApiClient: storefront, fail-fast gating, error mapping (wave 2)
- [x] 04-05-PLAN.md: Artwork disk cache with 500 MB LRU (wave 2)
- [x] 04-06-PLAN.md: Models, view keys, sort support, list state, TTL and retry policy (wave 2)
- [x] 04-07-PLAN.md: DataHandle: cache-then-revalidate, lazy paging, Home, error states (wave 3)
- [x] 04-08-PLAN.md: Debounced search with hints, scope and history (wave 3)
- [x] 04-09-PLAN.md: Lifecycle: offline cache, D-04 revalidation, sign-out wipe, Clear cache (wave 4)

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
**Plans**: 11 plans

Plans:
- [x] 05-01-PLAN.md: presto crate, egui fork pins, fastframe theme/icons/i18n, CLI (wave 1)
- [x] 05-02-PLAN.md: Mock catalog for every view, unavailable and failing tracks (wave 1)
- [x] 05-03-PLAN.md: Core detail data, track lists, playable flag, engine errors in CoreState (wave 2)
- [x] 05-04-PLAN.md: Pure playback, seek, volume, skip guard and queue-edit logic (wave 2)
- [x] 05-05-PLAN.md: Shared widgets and toasts (wave 2)
- [x] 05-06-PLAN.md: Backend over presto-core, demo/real launch, UI contract types (wave 3)
- [x] 05-07-PLAN.md: App shell, sidebar, status panels, shortcuts, main (wave 4)
- [x] 05-08-PLAN.md: Player bar and queue panel (wave 5)
- [x] 05-09-PLAN.md: Home, Library, Search, Settings (wave 5)
- [x] 05-10-PLAN.md: Album, playlist and artist pages (wave 5)
- [ ] 05-11-PLAN.md: Suite, manual demo pass, live API checks (wave 6, checkpoint)

### Phase 6: Desktop Integration
**Goal**: Presto behaves like a Linux desktop media player. (Outline.)
**Depends on**: Phase 5
**Requirements**: DESK-01, DESK-02
**Success Criteria** (what must be TRUE):
  1. KDE/GNOME/waybar show now-playing and media keys control playback.
  2. `busctl --user list | grep mpris` shows exactly one player.
  3. `presto` CLI commands control playback and `status --json` prints current state.
**Plans**: 5 plans

Plans:
- [x] 06-01-PLAN.md: ctl wire types in presto-ipc, clap subcommands, seek/volume parsers (wave 1)
- [ ] 06-02-PLAN.md: pure op planner and status/MPRIS state mapping, fastframe-now-playing dep (wave 2)
- [ ] 06-03-PLAN.md: control socket server, CLI client, flock single instance, main routing (wave 3)
- [ ] 06-04-PLAN.md: MPRIS pump task, startup wiring, private-bus test, MediaSession guard (wave 4)
- [ ] 06-05-PLAN.md: validation map, full suite, manual Wayland checklist (wave 5, checkpoint)

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
| 2. Engine Feasibility Spike (GATE) | 6/6 | Complete   | 2026-10-08 |
| 3. Core Backend, Supervisor, Auth | 15/15 | Complete | 2026-10-08 |
| 4. Data Layer and Cache | 0/9 | Planned | - |
| 5. Playback UI and Demo Mode | 0/11 | Complete    | 2026-10-08 |
| 6. Desktop Integration | 1/5 | In Progress|  |
| 7. Packaging and Distribution Notes | 0/TBD | Not started | - |

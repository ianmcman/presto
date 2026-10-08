# Phase 5: Playback UI and Demo Mode - Context

**Gathered:** 2026-10-08
**Status:** Ready for planning

<domain>
## Phase Boundary

The ported egui UI lets the user browse and play, and runs against the mock engine with no account. Covers PLAY-01 to PLAY-04 and IPC-04. MPRIS, media keys and CLI control (Phase 6) and packaging (Phase 7) are out of scope.

</domain>

<decisions>
## Implementation Decisions

### Port scope and structure
- **D-01:** Port core browse views plus any other generic spotifast view that needs no new backend capability: shell, sidebar, Home, Library tabs, Search, Album/Artist/Playlist pages, Queue, now-playing bar, Settings (clear cache), and portable extras. Spotify-only views stay skipped (per `docs/SPOTIFAST-SEAMS.md`). v2 features (lyrics, tray, playlist editing) are not ported even if spotifast has them.
- **D-02:** One phase, plans split into waves: UI crate and Backend glue, player and queue, browse views, detail pages, demo wiring.
- **D-03:** New binary crate `crates/presto` with a spotifast-shaped `Backend` (send/poll) wrapping presto-core's supervisor, queue mirror and data layer.
- **D-04:** Theming from fastframe-theme/fonts/icons. i18n plumbing is wired, English strings only.

### Player bar
- **D-05:** Bottom bar in spotifast layout: artwork and title/artist left, transport and seek centre, volume/shuffle/repeat/queue toggle right.
- **D-06:** Keyboard shortcuts port spotifast's map as-is, minus Spotify-only keys. Global media keys are Phase 6.

### Queue
- **D-07:** Queue is a right side panel toggled from the player bar, showing Now Playing then Up Next. Clicking a row plays it.
- **D-08:** Queue editing: play-from-here, remove, and Play Next / Add to Queue from row actions. Every edit rebuilds the queue through `SetQueue` keeping the current track and position, then reconciles with the revision mirror. No drag reorder.

### Detail pages
- **D-09:** Album and playlist pages: hero (artwork, title, artist/curator, year, track count, duration), Play and Shuffle buttons, numbered track list, double-click plays from that track. No Add to Library (PLED-01 is v2).
- **D-10:** Artist page: hero with Play/Shuffle (top songs), Top Songs list, horizontal Albums and Singles & EPs shelves with See all.

### Unavailable tracks
- **D-11:** Known-unavailable tracks render as a dimmed row with an Unavailable badge and a tooltip/inline reason on click. Playback passing one in a queue skips it with a toast.
- **D-12:** A runtime playback failure shows a toast with the error kind, marks the track unavailable for the session and auto-skips to the next. Playback stops if every queue item fails.

### Demo mode
- **D-13:** `presto --demo` spawns `presto-engine-mock` through the normal supervisor path, signed in from the start. The mock binary is found next to the presto binary or via an env override.
- **D-14:** A small DEMO chip is always shown in the status area and in the window title.
- **D-15:** The mock catalog is extended so every view works: Home shelves, library tabs, search, album/artist/playlist pages, at least one unavailable track, and enough items to exercise paging.
- **D-16:** Fault injection is `--fault` flag passthrough only (`presto --demo --fault slow`). No in-app fault controls.

### Claude's Discretion
- Crate-internal module layout, Backend event plumbing and view state structure.
- Toast and badge styling, tooltip wording, DEMO chip placement details.
- Which "portable extras" qualify under D-01 once the spotifast source is read; flag any that need new backend work.

</decisions>

<specifics>
## Specific Ideas

Spotifast is the layout and shortcut reference. music.apple.com layout remains the reference for Home and search results (Phase 4).

</specifics>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Seams and protocol
- `docs/SPOTIFAST-SEAMS.md` — Backend/Command/Event seams, spotifast-to-presto-ipc mapping, fork and fastframe pins
- `docs/PROTOCOL.md` — Commands, events, error enum, timeouts
- `docs/DATA-CACHE.md` — Cache freshness, paging, offline behavior used by views

### Prior decisions
- `.planning/phases/01-ipc-contract-and-mock-engine/01-CONTEXT.md` — Typed commands, queue snapshots with revision, mock engine and fault flags (D-06, D-07, D-10 to D-13)
- `.planning/phases/03-core-backend-supervisor-auth/03-CONTEXT.md` — Sign-in panel, re-auth banner, drift panel, recovery (D-06 to D-17)
- `.planning/phases/04-data-layer-and-cache/04-CONTEXT.md` — Cache-then-revalidate, lazy paging, error banners, search, Home shelves (D-01 to D-25)

### Planning
- `.planning/ROADMAP.md` — Phase 5 goal and success criteria
- `.planning/REQUIREMENTS.md` — PLAY-01 to PLAY-04, IPC-04

### External
- crmne/spotifast at `995c768dcba4da7f93302bf2107af6b7ec3df02b` — view, theme and shortcut source to port

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/presto-core`: supervisor, queue mirror (`mirror.rs`), auth state (`auth.rs`), data layer (`data/`: client, store, search, artwork, view models)
- `crates/presto-ipc`: Command, Event, Request types
- `crates/presto-engine-mock`: catalog (`catalog.rs`) and playback clock (`player.rs`) to extend

### Established Patterns
- Rust holds no Apple tokens; all API access is proxied `req` frames
- Cache-then-revalidate with 1-hour TTL, page size 100, lazy paging

### Integration Points
- New `crates/presto` binary: Backend over presto-core; `--demo` swaps the engine binary to the mock
- No egui code exists in the repo yet; the crmne/egui fork pin is only resolved in the throwaway crate

</code_context>

<deferred>
## Deferred Ideas

- Drag-and-drop queue reorder — not selected; revisit if wanted
- In-app fault-injection menu — not selected
- Lyrics, tray/mini-player, playlist editing — already v2 (LYR-01, TRAY-01, PLED-01)

</deferred>

---

*Phase: 05-playback-ui-and-demo-mode*
*Context gathered: 2026-10-08*

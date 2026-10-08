# Phase 5: Playback UI and Demo Mode - Research

**Researched:** 2026-10-08
**Domain:** Rust egui desktop UI (crmne fork, fastframe v0.4.1) over presto-core; mock-engine demo mode
**Confidence:** MEDIUM (repo and spotifast source read directly; Apple API shapes for detail pages and "unavailable" detection are not live-verified)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Port core browse views plus any other generic spotifast view that needs no new backend capability: shell, sidebar, Home, Library tabs, Search, Album/Artist/Playlist pages, Queue, now-playing bar, Settings (clear cache), and portable extras. Spotify-only views stay skipped (per `docs/SPOTIFAST-SEAMS.md`). v2 features (lyrics, tray, playlist editing) are not ported even if spotifast has them.
- **D-02:** One phase, plans split into waves: UI crate and Backend glue, player and queue, browse views, detail pages, demo wiring.
- **D-03:** New binary crate `crates/presto` with a spotifast-shaped `Backend` (send/poll) wrapping presto-core's supervisor, queue mirror and data layer.
- **D-04:** Theming from fastframe-theme/fonts/icons. i18n plumbing is wired, English strings only.
- **D-05:** Bottom bar in spotifast layout: artwork and title/artist left, transport and seek centre, volume/shuffle/repeat/queue toggle right.
- **D-06:** Keyboard shortcuts port spotifast's map as-is, minus Spotify-only keys. Global media keys are Phase 6.
- **D-07:** Queue is a right side panel toggled from the player bar, showing Now Playing then Up Next. Clicking a row plays it.
- **D-08:** Queue editing: play-from-here, remove, and Play Next / Add to Queue from row actions. Every edit rebuilds the queue through `SetQueue` keeping the current track and position, then reconciles with the revision mirror. No drag reorder.
- **D-09:** Album and playlist pages: hero (artwork, title, artist/curator, year, track count, duration), Play and Shuffle buttons, numbered track list, double-click plays from that track. No Add to Library (PLED-01 is v2).
- **D-10:** Artist page: hero with Play/Shuffle (top songs), Top Songs list, horizontal Albums and Singles & EPs shelves with See all.
- **D-11:** Known-unavailable tracks render as a dimmed row with an Unavailable badge and a tooltip/inline reason on click. Playback passing one in a queue skips it with a toast.
- **D-12:** A runtime playback failure shows a toast with the error kind, marks the track unavailable for the session and auto-skips to the next. Playback stops if every queue item fails.
- **D-13:** `presto --demo` spawns `presto-engine-mock` through the normal supervisor path, signed in from the start. The mock binary is found next to the presto binary or via an env override.
- **D-14:** A small DEMO chip is always shown in the status area and in the window title.
- **D-15:** The mock catalog is extended so every view works: Home shelves, library tabs, search, album/artist/playlist pages, at least one unavailable track, and enough items to exercise paging.
- **D-16:** Fault injection is `--fault` flag passthrough only (`presto --demo --fault slow`). No in-app fault controls.

### Claude's Discretion
- Crate-internal module layout, Backend event plumbing and view state structure.
- Toast and badge styling, tooltip wording, DEMO chip placement details.
- Which "portable extras" qualify under D-01 once the spotifast source is read; flag any that need new backend work.

### Deferred Ideas (OUT OF SCOPE)
- Drag-and-drop queue reorder
- In-app fault-injection menu
- Lyrics, tray/mini-player, playlist editing (LYR-01, TRAY-01, PLED-01)
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PLAY-01 | play, pause, seek, skip, shuffle, repeat, volume | Commands map 1:1 to `Command` variants; position interpolation from `PlayerMirror`; seek-drag rule; shortcut map (Standard Stack, Patterns 2 and 3) |
| PLAY-02 | see and use the queue | `QueueMirror` + `SetQueue` rebuild helper; revision reconcile; queue-edit pitfalls (Pattern 4) |
| PLAY-03 | open album/artist/playlist pages and play | Detail data layer is MISSING in presto-core; needs new `ViewKey`/loader and model fields (Gap G1) |
| PLAY-04 | unavailable tracks show a clear state | `QueueItem.playable`; browse rows need a derived flag; mock needs unavailable track and failure path (Gaps G2, G5; Pattern 5) |
| IPC-04 | `presto --demo` with no account or Widevine | Launcher closure over `presto-engine-mock`, isolated Paths and socket, mock catalog extension (Pattern 6, Gaps G3 to G6) |
</phase_requirements>

## Summary

The repo has no UI code yet. The backend half exists and is solid: `Core::start` supervisor with `CoreHandle::{state() -> watch<CoreState>, command, request}`, `QueueMirror`/`PlayerMirror` inside `CoreState`, and `DataHandle` (list/shelves/load_more/refresh/retry/clear_cache) plus `Search`. The UI crate is a new binary that owns a tokio runtime, builds a `CoreConfig`, and exposes spotifast's `send`/`poll` shape on top of it.

Three facts shape the plan. (1) Spotifast's views are not portable by copy: every view is `fn show(app: &mut App, ui, ...)` against a 905 KB `app.rs` (`collection.rs` 3.9k lines, `sidebar.rs` 2.6k, `widgets.rs` 4.1k; 82 to 228 `app.` references per file). Port the visual structure, theme and widget primitives; write a small Presto `App` and rewrite each view against it. (2) The data layer has no album, artist or playlist detail support and no `playable` flag on browse items, so PLAY-03 and PLAY-04 need presto-core additions. (3) The mock engine cannot yet serve a demo: `set_queue` only accepts the six hard-coded song ids (it rejects the generated `i.NNNNN` library ids), there are no detail routes, no unavailable track, and artwork URLs on `example.invalid` are rejected by the artwork cache.

**Primary recommendation:** Wave 1 builds the crate skeleton, fork pins, theme/icons, and Backend glue. Wave 2 does player bar, queue and the pure-logic state machines. Wave 3 extends presto-core (detail loaders, model fields) and the mock catalog together, then Waves 4 and 5 build browse/detail views and demo wiring on that.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| eframe / egui / egui_extras | 0.36 (fork resolves to egui 0.36.1) | UI | Spotifast's pin; verified fetchable in Phase 1 (SPOTIFAST-SEAMS) |
| crmne/egui fork | rev `ba6790fe7cf46e58e8d27ce1524cbfdee745e938` via `[patch.crates-io]` | Wayland frame pacing, accesskit focus | Copy the patch block from spotifast `Cargo.toml` lines 232-267 into the WORKSPACE root `Cargo.toml` (patch only works at the root) |
| crmne/winit | rev `ed7caa9023f10b397f5b6ec8284a840cbd8a6f65` | same patch block | Pair with egui fork |
| fastframe-theme, -fonts, -icons, -text | tag v0.4.1 | Palette catalog, Inter font, `icons!` macro, text snapping | D-04 |
| fastframe-i18n | tag v0.4.1 | `gettext`/`pgettext`, `build::compile_catalogs` in `build.rs` | D-04 plumbing; ship only `assets/i18n/presto.pot` and an empty/English catalog dir |
| tokio | workspace | runtime owned by `presto` main | existing |
| clap | workspace | `--demo`, `--fault` | existing |
| presto-core, presto-ipc | path | backend | existing |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| egui_extras features `image`, `svg`, `file` | fork | `install_image_loaders`; `file` enables `file://` URIs (verified in fork Cargo.toml) | Render artwork straight from `ArtCache` paths with `egui::Image::new(format!("file://{}", path))`; no custom loader needed |
| image | 0.25 (`jpeg`,`png`) | decode via egui_extras | as spotifast |
| fastframe-emoji | v0.4.1 | colour emoji plugin | optional; skip unless titles need it |
| egui_kittest | 0.36.2 (also in fork checkout) | headless widget/snapshot tests | optional smoke tests; compatibility with the patched fork unverified (LOW) |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `eframe::run_native` directly | `fastframe-shell` `Shell` | Shell exists to keep the app alive without a window (tray). Tray is v2. Use plain eframe; add Shell in Phase 6/v2 |
| Spotifast `ArtLoader` (`images.rs`, 51 KB, HTTP + sha1 + Http type) | presto-core `ArtCache` + egui `file://` | ArtCache already enforces mzstatic-only, caps and LRU. Do not port `images.rs` |
| Port `fastframe-update`, `-tray`, `-audio`, `-macos`, `-scroll`, `-log`, `-now-playing`, `-instance`, `-shell` | none | Not needed this phase. `fastframe-scroll` (wheel/touchpad feel) is a possible later polish |

**Installation:** add `crates/presto` to the workspace (already `members = ["crates/*"]`); put the `[patch.crates-io]` block in root `Cargo.toml`; workspace deps for eframe, egui_extras, fastframe-* with `git = "https://github.com/crmne/fastframe", tag = "v0.4.1"`. Features for eframe: `default-features = false, features = ["accesskit","glow","default_fonts","links","wayland","x11","persistence"]` (spotifast's list). Run `cargo tree -i egui --depth 0` to confirm the single fork egui; the first build is slow.

**Version verification:** egui_kittest 0.36.2 confirmed on crates.io (`cargo search`). Fork and fastframe pins were verified fetchable in Phase 1 (docs/SPOTIFAST-SEAMS.md); not rebuilt in this research.

## Architecture Patterns

### Recommended Project Structure
```
crates/presto/
├── Cargo.toml
├── build.rs                 # fastframe_i18n::build::compile_catalogs("assets/i18n")
├── assets/{icons/,i18n/}    # copy the SVGs actually used + LICENSE.txt from spotifast (MIT)
└── src/
    ├── main.rs              # clap args, runtime, Launcher, eframe::run_native
    ├── cli.rs               # --demo, --fault (repeatable, passthrough)
    ├── backend.rs           # Backend { send, poll } over CoreHandle/DataHandle/Search
    ├── model.rs             # Action, Page, Event, Toast (no egui)
    ├── playback.rs          # pure: position interpolation, seek-drag, unavailable/skip guard
    ├── queue_ops.rs         # pure: play_from, remove, play_next, add -> (ids, start) for SetQueue
    ├── theme.rs             # Palette (Apple dark), impl fastframe_theme::Palette, Icon enum
    ├── app.rs               # App state, per-frame logic()/ui()
    └── ui/{mod,keys,sidebar,player_bar,queue,home,library,search,album,playlist,artist,settings,widgets,toasts}.rs
```
Keep `playback.rs` and `queue_ops.rs` free of egui so they unit-test without a display.

### Pattern 1: Backend = spotifast shape over presto-core
**What:** `Backend::send(Command)` fires and forgets onto the tokio runtime; results return as `Event`s through a `std::sync::mpsc` the UI drains in `poll()` each frame. State that already lives in `watch` channels (`CoreState`, `ListState<Item>`, `SearchState`) is read directly with `borrow()` in the frame; do not copy it into events.
**Wake-up:** the UI must repaint when a watch changes. Spawn one task per watch (`rx.changed().await; ctx.request_repaint()`), and request a repaint every ~250 ms while `PlayState::Playing` for the interpolated progress. Without this the UI only updates on input (idle-loop pitfall).
**Spotifast ref:** `Backend` at `src/backend.rs:876`, `poll` at `1222` is just `events.try_iter().collect()`.
**Notes:** `Command::Player(..)` maps to `CoreHandle::command`, API commands go through `DataHandle`. Build `DataHandle::new(core.clone(), &paths)` and `Search::new(data.client().clone(), data.store().clone())` inside the runtime (both require `Handle::current()`).

### Pattern 2: Player state and seek
**What:** read `CoreState.player` (`PlayerMirror`: state, position_ms, duration_ms, volume, shuffle, repeat, track, seq). Record `Instant::now()` when a `position_ms` change is observed and interpolate while `Playing` (`position_ms + elapsed`, clamped to duration). The mirror already drops stale `seq`.
**Seek bar:** while dragging show the drag value; on release send exactly one `Command::Seek { ms }`. Do not send on drag. Keep the optimistic position for ~1 s or until the next `Progress` with the new `seq`, otherwise the thumb snaps back.
**Toggle:** `Play` when state is `Paused|Stopped|Ended`, else `Pause` (docs/SPOTIFAST-SEAMS mapping).
**Volume:** `SetVolume` takes 0.0..=1.0 f32. Mute is UI-side: remember the pre-mute volume and send 0.0, restore on unmute. Throttle drag sends (send on change at most every ~50 ms and on release).

### Pattern 3: Shortcut map (D-06) and a spec conflict
Spotifast `src/ui/keys.rs` map (Spotify-only keys removed): Space play/pause (unless a text field has focus), Ctrl+Left/Right previous/next, Ctrl+Up/Down volume +-5, Shift+Left/Right seek +-10 s, M mute, S shuffle, R cycle repeat, Q toggle queue (when not typing), Ctrl+Shift+Q toggle queue, Ctrl+F or `/` focus search, Ctrl+B toggle sidebar, Ctrl+, settings, Ctrl+H home, Ctrl+Q quit, Alt+Left/Right back/forward, `?` shortcuts dialog. Drop: Ctrl+L (Liked Songs), Ctrl+M/Ctrl+Shift+K (Winamp/MilkDrop), L (lyrics), B (save), Ctrl+Shift+A/B (open current artist/album: these are generic and may stay).
**Conflict:** UI-SPEC's "minimum" list (Left/Right seek 5 s, Up/Down volume, N/P) does not match spotifast's map. D-06 is locked ("port as-is"), so follow the spotifast map. N/P can be added as extra aliases at no cost; plain arrows seek only where focus is not in a list. Flag to the user.
Use `input.consume_key` with Shift variants checked before plain ones (spotifast comment: egui ignores an extra Shift).

### Pattern 4: Queue edits through SetQueue (D-08)
`queue_ops` returns `(ids, start)` from the `QueueMirror` snapshot: play_from(i) -> same ids, start=i; remove(i) -> ids without i, start adjusted (if removing before current, start-1; removing the current track plays the next); play_next(id) inserts at index+1; add(id) appends. Ids sent to `SetQueue` are the item ids already in the mirror, plus the catalog id for new items.
**Critical pitfalls:**
- `SetQueue` restarts the current track. To keep position: send `SetQueue { play: was_playing }` then a verified `Seek`. Phase 3 found Seek to targets under about 3 s is never answered by live MusicKit and that restore must wait out `Loading` (STATE.md). Reuse that knowledge: for an edit that keeps the current track, if position < 3 s, skip the seek. The stepwise restore lives inside the supervisor and is not exposed; either add a small `CoreHandle` helper or accept an audible restart on non-current edits and document it.
- After sending, ignore mirror snapshots with `rev` older than the one at send time; the mirror already drops `rev <= current`, but a new generation (bridge reload) resets to 0, so key optimistic UI state on `generation` as well.
- Disable edits while `EngineStatus != Ready`.
- Remove of the only item = `Pause` plus clear UI; `SetQueue` with empty ids is not valid in the mock (`items.is_empty()` errors).
- `QueueItem.id` for library songs is the library id (`i.xxx`); verify the real bridge accepts library ids in `setQueue({songs})` (live check, see Open Questions).

### Pattern 5: Unavailable tracks (D-11, D-12)
Two sources: (a) `QueueItem.playable == false` from the engine (`isPlayable !== false` in bridge.js); (b) a session-local `HashSet<String>` of ids that failed at runtime. Browse rows have only `Item.play_params` today, so derive `Item.playable = play_params.is_some()` for songs (Apple omits `playParams` on unplayable catalog songs; MEDIUM, not live-verified) and mark rows from the union with the session set.
Guard state machine (pure, in `playback.rs`): on `TrackChanged(item)` with unplayable item -> toast "Skipped ... unavailable" and send `Next`; on `Event::Error` while a track is current or `PlayState` goes `Stopped|Ended` immediately after `Loading` -> add id to session set, toast with kind, send `Next`. Count consecutive skips; when it reaches queue length with repeat off (or any count >= queue length), stop and toast "Nothing in the queue can be played." Reset the counter on a `Playing` state with progress > 0. Without that bound a repeat-all queue of unavailable tracks loops forever.
Unknown: what real MusicKit does on an unplayable queue item (skip itself, error, or stall). Treat the guard as defensive and confirm live (Open Questions).

### Pattern 6: Demo mode (IPC-04, D-13 to D-16)
- `Launcher` closure: `Launch { program: mock_path, args: faults.flatten() }`. The supervisor already appends `--socket` and `--profile`. Mock path: sibling of `current_exe()` or `PRESTO_ENGINE_MOCK`. In dev, `cargo run -p presto -- --demo` does not build the mock; build both or declare it a path/artifact dependency (bindeps is unstable), and fail with a message naming the missing path.
- Isolate everything: `Paths::under(state/presto/demo)` (cache, db, artwork, install-id, pidfile, profile) and `socket` = `$XDG_RUNTIME_DIR/presto/engine-demo.sock`. Otherwise the demo mock storefront `us` and its fake catalog pollute the real cache keyed `storefront:install-id`, and a running real instance's socket/pidfile sweep (`sweep_stale`) could kill its engine. Wipe the demo dir at start (cache TTL is 1 h; a stale cached mock catalog would hide mock changes).
- Pass `--auth signed_in` (default) and the user's `--fault` values straight through; the existing `--fault` parser (`FaultSpec: FromStr`) can validate early.
- DEMO chip in sidebar status area plus viewport title `Presto (Demo)` via `ViewportBuilder::with_title`.
- Demo must open directly on Home: no sign-in panel. Phase 3's sign-in panel logic keys off `CoreState.auth`; the mock sends `signed_in`, so no special-casing.

### Pattern 7: Theme and icons
Define Presto's own `Palette` (spotifast's is green; fields: window, panel, surface, surface_hover, surface_active, outline, text, secondary, dim, accent, accent_hover, on_accent, danger, warning, overlay, shadow) and `impl fastframe_theme::Palette` (`base(Base::Dark|Light)`). Dark values from UI-SPEC: window #000000, panel/surface #1C1C1E, hover #2C2C2E, text #FFFFFF, secondary #98989F, accent #FA243C, danger #FF453A, warning #FF9F0A. Light is not specified: map `Base::Light` to the dark palette or a plain light variant but do not design it. Icons: `fastframe_icons::icons!` with `directory: "../assets/icons/"` and `lucide "name"` for shared icons; copy the non-Lucide SVGs actually used (skip-back/forward, play/pause-filled, shuffle, repeat, repeat-1, volume, volume-1, list-music, house, library, music, disc-3, list-plus, list-end, loader-circle, audio-lines) with `LICENSE.txt`. Call `egui_extras::install_image_loaders` and `fastframe_icons::install::<Icon>` once at startup (spotifast `theme::install`).

### Anti-Patterns to Avoid
- **Copying spotifast views verbatim:** they take `&mut App` and reference Spotify models, `Action`, `Page`, settings, lyrics and Winamp. Port layout and widget code, not the files.
- **Porting `images.rs`, `api/`, `http.rs`, `limiter.rs`, `session_reads.rs`:** Rust never calls Apple; artwork goes through `ArtCache`.
- **Reading `CoreState` by cloning it every frame into events:** borrow the watch.
- **Blocking the UI thread on `CoreHandle` calls:** every call is async; spawn on the runtime.
- **A second cache of `ListState` in the UI:** hold the `watch::Receiver`s per open view and drop them when the view closes.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Artwork fetch/cache | HTTP loader | `data::artwork::ArtCache::get(url)` -> `ArtState::{Ready(path),Pending,Failed}` + egui `file://` image | Host allowlist, 10 MB cap, LRU already done (quick task 261008-bsb) |
| Template size | string replace | `artwork::expand(template, px)` | Snaps to cached sizes 160/320/640 |
| Paging, TTL, retry ladder, offline | custom | `DataHandle::list/shelves/load_more/refresh/retry` and `ListState.{phase,error,from_cache,has_more}` | Phase 4 semantics (D-xx) already tested |
| Error copy | new strings | `UiErrorKind::message()` and `retryable()`; `error_display()` (Banner vs FullView) | Consistent with Phase 4; UI-SPEC copy for generic cases overrides where it differs |
| Search debounce/hints/history | custom | `data::search::Search` (`input`, `submit`, `set_scope`, `state()`) | 300 ms debounce, history of 10 |
| Queue reconcile | own revision logic | `QueueMirror::apply` / `PlayerMirror::apply` already inside `CoreState` | rev/seq/generation handled |
| Restart/recovery, auth banners | UI state machines | `CoreState.engine` (`Starting/Ready/Restarting/Drift/Failed`) and `auth` | Phase 3 |
| Virtualised long lists | manual windowing | `egui::ScrollArea::show_rows` (rows are fixed 48 pt) | 100+ item lists; spotifast `virtual_rows` is similar |
| Slider/seek widget | new | port spotifast `thin_slider` (`widgets.rs:2570`) and adapt | Hover grow, wheel, a11y already handled |
| Toasts | none exist as a lib | small own `Vec<Toast>` (3 max, 5 s, hover pause) | trivial, per UI-SPEC |

**Key insight:** the UI should be a thin projection of `CoreState`, `ListState` and `SearchState`; new logic belongs in the two pure modules (`playback`, `queue_ops`) so it is testable without a GL context.

## Gaps in existing crates (must be planned)

**G1. Detail data layer (PLAY-03).** `ViewKey` has only `Library`, `RecentlyPlayed`, `Recommendations`, `Shelf{path}`. `models::parse_item` ignores `relationships`, `releaseDate`, `trackCount`, genres. Add:
- `Item` fields: `release_date: Option<String>`, `track_count: Option<u32>`, `playable: bool`.
- Track lists: add `ViewKey::Tracks { path }` (page size 100, same `Item` paging) and build `path` in the UI glue with the storefront (`client.cached_storefront()`) because `ViewKey::request` has no storefront. Catalog: `/v1/catalog/{sf}/albums/{id}/tracks`, `/v1/catalog/{sf}/playlists/{id}/tracks`. Library: `/v1/me/library/albums/{id}/tracks`, `/v1/me/library/playlists/{id}/tracks`. Choose by `Item.library`. Page-size limits for these routes are unverified (LOW); start at 100 and fall back to the API default if rejected. `Shelf{path}` would also work but pages at 10.
- Heads: the clicked `Item` already has name, artwork, subtitle. Year, track count and duration come from a one-shot `GET .../albums/{id}` (the mock already serves it with `relationships.tracks`). Compute total duration by summing track `duration_ms` once all pages are loaded.
- Artist: `GET /v1/catalog/{sf}/artists/{id}?views=top-songs,full-albums,singles` (Apple's `views` parameter, MEDIUM from training, verify live). A new small loader modelled on `Search` (a `watch<ArtistState>` with `top_songs`, `albums`, `singles`, error). Library artists: map to the catalog artist via `/v1/me/library/artists/{id}/catalog` or search by name (LOW); the simplest plan is to open the catalog artist for catalog ids and, for library artists, request `/v1/me/library/artists/{id}?include=catalog`.
- Cache: reuse `Store` get_page/put_page keyed by `req_key` for the TTL/offline behaviour; copy the `first()`/`fetch0()` flow rather than reimplementing offline rules.

**G2. Playable flag on browse items** (see Pattern 5). Mock must omit `playParams` on the unavailable track and set `QueueItem.playable=false` when queued.

**G3. Mock `set_queue` ids.** `catalog::song()` only resolves SONGS; library-song ids from `--library-songs` and any new demo songs fail with `not_found`. Make `song()` resolve every generated id and the extended catalog.

**G4. Mock catalog extension (D-15).** Currently: 6 songs, 2 artists, 2 albums, 2 playlists, 2 recommendation groups, no artist route. Need: more albums/playlists/artists (Home shelves and library tabs), artist detail with `views` (top-songs, full-albums, singles), `/albums/{id}/tracks` and `/playlists/{id}/tracks` routes (paged), one unavailable track, one very long title, a library list over 100 items by default (the `--library-songs` flag defaults to 0; the demo launcher must pass e.g. `--library-songs 250` or the default must change), and more recently-played entries. Keep existing tests green: Phase 1/4 tests assert specific counts and ids, so add data rather than change existing records, or update the snapshots deliberately.

**G5. Runtime playback failure (D-12).** The mock cannot fail playback. Add a mock behaviour: when the current queue item is the designated unavailable song, emit `Error{kind: unavailable}` (and optionally `PlaybackState Stopped`) instead of playing; or a new `--fault play_error` is NOT allowed to be required (D-16 only forbids in-app controls; a new startup fault is a protocol-doc change since `docs/PROTOCOL.md` and `doc_covers_variants` test list fault kinds). Prefer deriving it from the unavailable track so no new fault kind is needed.

**G6. Demo artwork.** Mock URLs are `https://example.invalid/artwork/{id}/{w}x{h}.jpg`; `ArtCache::url_allowed` rejects non-mzstatic hosts (the localhost allowance is `cfg(test)` only and must stay that way), so every cover becomes `ArtState::Failed`. UI-SPEC wants deterministic placeholders and no network. Recommended: UI-side placeholder painter that detects a demo/`example.invalid` URL (or `ArtState::Failed`) and paints a solid colour from a hash of the id with a music glyph. This does not exercise the artwork pipeline offline; the pipeline is covered by Phase 4 tests. Alternative (not recommended): generate PNGs in the mock and widen the allowlist, which weakens the hardening just added.

**G7. Settings "Clear cache".** `DataHandle::clear_cache()` exists; Settings only needs the confirm dialog from UI-SPEC copy.

## Portable extras (D-01 discretion)
From spotifast's module list, qualifies without new backend work: toasts, shortcuts dialog (`?`), back/forward navigation history (`history.rs`, 11 KB, check for Spotify types before copying), sidebar collapse toggle (Ctrl+B), table column header widget. Requires new backend work, so skip: Liked Songs, Radio, Shows/podcasts, Devices, EQ, Winamp, MilkDrop, Lyrics, tray, mini player, auto-update, playlist editing/folders/drag-drop, "save/like" (B key).

## Common Pitfalls

### Pitfall 1: UI only redraws on input
**What goes wrong:** progress bar freezes, queue changes appear late.
**Why:** egui is reactive; watch channel updates do not wake it.
**How to avoid:** repaint-on-change tasks plus `request_repaint_after(250ms)` while playing.
**Warning signs:** position advances only when the mouse moves.

### Pitfall 2: Seek echo and snap-back
**What goes wrong:** thumb jumps back after release; double seeks.
**How to avoid:** single Seek on release, keep optimistic value until a `Progress` with a newer `seq`; drop the drag value if the track changes.

### Pitfall 3: Queue rebuild glitches
**What goes wrong:** every Play Next restarts the song; edits race with natural advance (`rev` bumps on advance).
**How to avoid:** compute edits from the latest mirror at send time; if `rev` changed between click and send, recompute; skip seek for position < 3 s; avoid edits while `Loading`.

### Pitfall 4: Skip loops
Auto-skip with all tracks unavailable and repeat-all. Bound by consecutive-failure counter (Pattern 5).

### Pitfall 5: Demo contaminates or collides with real state
Shared cache, socket or pidfile (Pattern 6). Isolate and wipe.

### Pitfall 6: Mock binary not found
`cargo run -p presto -- --demo` builds only `presto`. Resolve sibling of `current_exe()`, then `PRESTO_ENGINE_MOCK`, and print an actionable error ("run cargo build -p presto-engine-mock"). Tests use the same sibling trick (`tests/common/mod.rs::mock_bin`).

### Pitfall 7: Text and copy rules
UI-SPEC has 4 sizes and 2 weights only; Inter from fastframe-fonts must provide a 600 weight (verify the family has semibold; spotifast uses `fastframe_fonts::INTER`). Truncate with ellipsis plus tooltip. Quotes in copy ("No results for "{query}"") need translation-safe `gettext` strings with named placeholders.

### Pitfall 8: Headless CI
eframe cannot open a GL window in CI. Do not make `cargo test` depend on a display. Logic tests are pure; any widget smoke test uses egui_kittest or is manual.

### Pitfall 9: Engine states in the UI
`EngineStatus::Restarting/Drift/Failed` and `auth` expired/signed-out (Phase 3 panels, banners) still must render in the new shell. Phase 3 and 4 defined behaviour (re-auth banner, drift panel, Failed with log tail) but no UI exists yet; include them in the shell wave or the demo `--fault auth_expired` / `--fault crash` paths have no visible result.

## Code Examples

### Building the core for demo (sketch, from presto-core tests/common and config.rs)
```rust
// Source: crates/presto-core/tests/common/mod.rs, src/config.rs
let paths = Paths::under(state_base.join("demo"));
let cfg = CoreConfig {
    launcher: Arc::new(move |_attempt| Launch { program: mock.clone(), args: fault_args.clone() }),
    socket: runtime_dir.join("presto/engine-demo.sock"),
    paths: paths.clone(),
    timings: Timings::default(),
};
let core = Core::start(cfg).await?;
let data = DataHandle::new(core.clone(), &paths)?;
let search = Search::new(data.client().clone(), data.store().clone());
```

### Reading player state per frame
```rust
// Source: crates/presto-core/src/state.rs, mirror.rs
let s = core_rx.borrow();            // watch::Receiver<CoreState>
let p = &s.player;                   // state, position_ms, duration_ms, volume, shuffle, repeat, track
let q = &s.queue;                    // rev, items, index
```

### Artwork in a cell
```rust
// Source: crates/presto-core/src/data/artwork.rs; egui_extras `file` feature
match art.get(&artwork::expand(&a.url, 160)) {
    ArtState::Ready(p) => { ui.add(egui::Image::new(format!("file://{}", p.display())).fit_to_exact_size(vec2(40.0, 40.0))); }
    ArtState::Pending => { ui.ctx().request_repaint_after(Duration::from_millis(100)); paint_placeholder(ui, id) }
    ArtState::Failed => paint_placeholder(ui, id),
}
```

### Shortcut handling shape (from spotifast keys.rs)
```rust
// Source: spotifast src/ui/keys.rs @ 995c768
let editing_text = ctx.text_edit_focused();
ctx.input_mut(|i| {
    if !editing_text && i.consume_key(Modifiers::NONE, Key::Space) { actions.push(Action::TogglePlay); }
    if !editing_text && i.consume_key(Modifiers::COMMAND, Key::ArrowRight) { actions.push(Action::Next); }
    // Shift variants before plain ones.
});
```

## State of the Art

| Old Approach | Current Approach | Impact |
|--------------|------------------|--------|
| Spotifast `Backend` runs librespot + Web API | Presto `Backend` is a facade over `CoreHandle`/`DataHandle` | Same call shape, none of the Spotify state |
| Spotifast demo (feature `demo`, `src/demo.rs` 365 KB fixture data inside the UI) | Demo = real UI over mock engine process | Demo exercises supervisor, IPC and data layer; fixtures live in the mock |
| Custom image loader (`images.rs`) | `ArtCache` + `file://` | Hardened fetch reused |

## Open Questions

1. **Does the real bridge accept library ids (`i.xxx`) and mixed lists in `setQueue({songs})`?**
   - Known: bridge calls `mk.setQueue({ songs: cmd.ids })`; Phase 4 data layer exposes both catalog and library ids; live Phase 2/3 only played catalog ids as far as the docs show.
   - Unclear: library song ids and ids from library albums/playlists in a live session.
   - Recommendation: a one-line live check with `crates/presto-core/examples/live.rs` (`queue i.xxx`) before finalising the Play-from-library plan; if library ids fail, use `playParams.catalogId` from `Item.play_params` (add accessor) as the queue id.
2. **What does MusicKit do with an unplayable item?** Treat the guard as required; confirm live.
3. **Artist `views` parameter and library-artist to catalog-artist mapping** (G1). Verify with the `probe` command in `live.rs`.
4. **Demo artwork approach** (G6): placeholder painter recommended; confirm with user only if they want real pipeline exercise.
5. **UI-SPEC shortcut minimum vs D-06** (Pattern 3): resolved in favour of D-06, note for the user.
6. **Page-size limits of `/tracks` relationship routes** (LOW).
7. **Window state persistence** (`persistence` feature) would store demo window geometry in the real profile; give demo its own storage path or disable persistence in demo (spotifast uses `demo_native_options(.., demo_storage)`).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust libtest via `cargo test` (workspace), `insta` for snapshots (existing), tokio test-util |
| Config file | none; workspace `Cargo.toml`, `rust-toolchain.toml` 1.98.0 |
| Quick run command | `cargo test -p presto --lib` |
| Full suite command | `cargo build -p presto-engine-mock && cargo test --workspace` |

### Phase Requirements to Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PLAY-01 | position interpolation, seek-drag single send, toggle mapping, repeat cycle, mute restore | unit | `cargo test -p presto playback::` | Wave 0 |
| PLAY-01 | Backend send maps each action to the right `Command` against the mock | integration | `cargo test -p presto --test backend_mock` | Wave 0 |
| PLAY-02 | `queue_ops` play_from/remove/play_next/add index math | unit | `cargo test -p presto queue_ops::` | Wave 0 |
| PLAY-02 | edit via SetQueue reconciles with mirror rev | integration (mock) | `cargo test -p presto --test queue_edit` | Wave 0 |
| PLAY-03 | model parses release date, track count, relationships; Tracks view key path per storefront/library | unit | `cargo test -p presto-core models:: view::` | extend existing |
| PLAY-03 | detail loader against mock: album, playlist, artist pages, paging | integration | `cargo test -p presto-core --test data_detail` | Wave 0 |
| PLAY-04 | guard: skip on unplayable, runtime failure marks + skips, all-failed stop, counter reset | unit | `cargo test -p presto playback::guard` | Wave 0 |
| PLAY-04 | mock: unavailable song has no `playParams`, queued item `playable=false`, error on play | integration | `cargo test -p presto-engine-mock --test protocol` | extend |
| IPC-04 | `--demo` launches mock, reaches Ready/SignedIn, isolated paths, fault passthrough parses | integration | `cargo test -p presto --test demo_boot` | Wave 0 |
| IPC-04 | extended catalog: library > 100 items, long title, every view route returns data | unit | `cargo test -p presto-engine-mock catalog::` | extend |
| UI states | widget smoke (optional) via egui_kittest | smoke | `cargo test -p presto --test ui_smoke` | optional, LOW |
| IPC-04 | `presto --demo` window opens, Home visible, DEMO chip | manual | run `cargo run -p presto -- --demo` | manual-only (needs display/GL) |

### Sampling Rate
- **Per task commit:** `cargo test -p presto --lib` (and `-p presto-core` / `-p presto-engine-mock` when touched)
- **Per wave merge:** full suite command
- **Phase gate:** full suite green plus a manual `--demo` pass (play, seek, queue edit, album/artist/playlist, unavailable track, `--fault slow`, `--fault auth_expired`, `--fault crash`)

### Wave 0 Gaps
- [ ] `crates/presto` crate with `tests/common` (copy `mock_bin`, `rig`, `wait_for` from presto-core tests)
- [ ] `playback.rs`, `queue_ops.rs` with unit tests (pure)
- [ ] mock catalog extension tests; update `docs/PROTOCOL.md` only if protocol surface changes (it should not)
- [ ] optional `egui_kittest` dev-dependency (verify it builds against the fork)

## Sources

### Primary (HIGH confidence, read directly)
- Repo: `crates/presto-core/src/{lib,state,mirror,auth,config,supervisor,paths}.rs`, `data/{mod,view,models,client,artwork,search,error}.rs`, `crates/presto-ipc/src/{command,event,kind}.rs`, `crates/presto-engine-mock/src/{catalog,player,main}.rs`, `engine/bridge.js`, `docs/{PROTOCOL,DATA-CACHE,SPOTIFAST-SEAMS}.md`
- spotifast at 995c768 (cloned): `Cargo.toml` (deps, patch block 232-267), `build.rs`, `src/{theme,i18n,entrypoint,backend}.rs`, `src/ui/{mod,keys,widgets,player_bar,queue,artist,library}.rs`, `assets/icons`
- egui fork checkout `ba6790f`: `crates/egui_extras/Cargo.toml` (`file` feature), `crates/egui_kittest` present

### Secondary (MEDIUM)
- `cargo search egui_kittest` -> 0.36.2
- Apple `playParams` absent on unplayable songs, `views=` parameter on artists (training knowledge, not verified live)

### Tertiary (LOW)
- `/tracks` relationship page limits; library-artist to catalog mapping; real MusicKit behaviour on unplayable items

## Metadata

**Confidence breakdown:**
- Standard stack: MEDIUM-HIGH, pins from spotifast and Phase 1 verification; not rebuilt here
- Architecture: MEDIUM, the Backend/watch design follows existing crate APIs, view rewrite effort is the main risk
- Gaps (G1 to G7): HIGH that they exist (read from source); MEDIUM on the Apple endpoints chosen to fill them
- Pitfalls: MEDIUM

**Research date:** 2026-10-08
**Valid until:** 2026-11-07 (fork rev and fastframe tag are pinned; Apple API assumptions need a live check sooner)

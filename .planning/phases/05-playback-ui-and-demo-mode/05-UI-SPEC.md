---
phase: 5
slug: playback-ui-and-demo-mode
status: draft
shadcn_initialized: false
preset: none
created: 2026-10-08
---

# Phase 5 - UI Design Contract

> Native Rust + egui (crmne fork). No web, no shadcn. Units are egui logical points (pt), 1pt = 1px at 100% scale. Layout and shortcuts follow spotifast at 995c768d. Where fastframe-theme supplies a value, the theme wins and the value below is the fallback to set only if the theme has none.

---

## Design System

| Property | Value |
|----------|-------|
| Tool | none (egui `Style`/`Visuals` set once at startup) |
| Preset | not applicable |
| Component library | egui + egui_extras, fastframe-shell/theme |
| Icon library | fastframe-icons (no new icon fonts, no emoji as icons) |
| Font | fastframe-fonts (default proportional); monospace only for time readouts |
| Theme | dark only in this phase; light is not specified |
| i18n | strings via the spotifast i18n plumbing, English only (D-04) |

---

## Spacing Scale

All in pt, multiples of 4.

| Token | Value | Usage |
|-------|-------|-------|
| xs | 4 | Icon gaps, badge padding, row inner gap |
| sm | 8 | Compact spacing, button padding, shelf card gap |
| md | 16 | Default panel padding, hero gaps |
| lg | 24 | Section padding, hero inner padding |
| xl | 32 | Layout gaps between shelves |
| 2xl | 48 | Major section breaks, empty-state top offset |
| 3xl | 64 | Hero top padding on detail pages |

Fixed layout dimensions (exceptions to the scale, structural not spacing):

| Element | Size |
|---------|------|
| Sidebar width | 224 |
| Queue side panel width | 320 (min 280) |
| Player bar height | 88 |
| Track row height | 48 (artwork 40 in rows) |
| Shelf card artwork | 160 square (artist cards circular) |
| Hero artwork | 200 square |
| Player bar artwork | 56 |
| Icon-only button hit target | 32 minimum, transport play/pause 40 |
| Seek bar thickness | 4 (hover 8) |

Exceptions: row height 48 and player bar 88 are not on the 8-point ladder by choice of spotifast density; target 32 matches the ladder.

---

## Typography

Exactly 4 sizes, 2 weights (Regular 400, Semibold 600).

| Role | Size | Weight | Line Height |
|------|------|--------|-------------|
| Body (rows, buttons, queue, tooltips) | 14 | 400 | 1.5 |
| Label (secondary text, artist, timestamps, badges, column headers) | 12 | 400 | 1.5 |
| Heading (shelf titles, section headers) | 20 | 600 | 1.2 |
| Display (hero title on album/artist/playlist pages) | 32 | 600 | 1.2 |

Rules: row titles use Body 400; the now-playing title uses Body at 600. Time readouts use Label with tabular figures (monospace). No other sizes or weights anywhere. Truncate single-line text with ellipsis and show full text in a tooltip.

---

## Color

| Role | Value | Usage |
|------|-------|-------|
| Dominant (60%) | #121214 | Central panel, page background |
| Secondary (30%) | #1C1C20 | Sidebar, player bar, queue panel, cards, hovered rows (#26262B) |
| Text primary | #F2F2F5 | Titles, body |
| Text secondary | #9A9AA3 | Artist, timestamps, disabled-looking metadata |
| Accent (10%) | #FA243C | See list below |
| Destructive | #E5484D | Destructive actions and error toasts only |
| Warning | #E0A030 | Offline/rate-limit banners, Unavailable badge text |

Accent reserved for: the play/pause button fill in the player bar, the Play button on hero sections, the seek bar and volume bar filled portion, the currently playing row's title and its playing glyph, the active sidebar item indicator bar, active shuffle/repeat/queue toggle state, keyboard focus ring. Not for links, hover states, Shuffle buttons (outlined, text primary), badges or the DEMO chip.

Unavailable row: contents at 40% opacity, badge uses Warning text on #26262B.

DEMO chip: outlined, Warning color text and border, Label 12 at weight 600, 4 vertical / 8 horizontal padding, corner radius 8.

Contrast: Text primary on Dominant and Secondary meets 4.5:1; Text secondary on Secondary is at least 4.5:1 (verify with theme; lighten to #A8A8B2 if not).

---

## Layout and Components

Shell (D-01): left sidebar (224) | central panel | optional right queue panel (320, D-07) | bottom player bar (88) spanning full width. Central panel scrolls vertically; content max width unrestricted, horizontal padding md (16), top padding lg (24).

Sidebar order: Home, Search, then a "Library" group: Playlists, Albums, Artists, Songs; Settings pinned at the bottom. Status area at the sidebar bottom above Settings holds the DEMO chip (D-14), offline/engine state dot and re-auth entry from Phases 3 and 4. Window title in demo: `Presto (Demo)`; otherwise `Presto`.

Player bar (D-05), three zones:
- Left: artwork 56, title (Body 600) over artist (Label secondary), both truncating; clicking the artist opens the artist page.
- Centre: shuffle, previous, play/pause (40, accent fill), next, repeat (off/all/one icon states), below them elapsed time, seek bar (fills available width, max 480), remaining/total time.
- Right: queue toggle, volume icon (click mutes) with a 96 wide volume bar.

Seek: click and drag scrubs, release sends a single Seek command; while dragging show the drag position, not engine position. Keyboard shortcuts port spotifast's map (D-06) minus Spotify-only keys; planner extracts the map from source. Minimum: Space play/pause, Left/Right seek 5s, Up/Down volume, N/P next/previous.

Queue panel (D-07, D-08): header "Queue" (Heading) with close button; "Now Playing" section (Label caps-free, secondary) with one row, then "Up Next" rows. Row: 40 artwork, title/artist, duration. Clicking a row plays it (D-07); a play glyph shows on hover. Row context menu / hover actions: Play from here, Play next, Add to queue, Remove. No drag handles (reorder deferred).

Track list rows (album/playlist/artist top songs): columns number (or playing glyph), title (+ artist when not an album page), duration. Double-click plays from that row (D-09). Hover shows play glyph in the number cell and a row action menu (Play next, Add to queue).

Detail heroes (D-09, D-10): artwork 200 left, right column: Label type ("Album", "Playlist", "Artist"), Display title, Label secondary line (artist/curator, year, N songs, duration). Buttons below: Play (accent filled) and Shuffle (outlined). No Add to Library. Artist page: circular 200 artwork, Play/Shuffle act on top songs, then Top Songs list (first 5, "Show more" expands), then Albums and Singles & EPs shelves: horizontal scroll, card 160 art plus title and year, "See all" link (text primary, underlined on hover) at the shelf heading's right.

Library tabs and Home/Search reuse Phase 4 view models; lists are lazy-paged (page 100) with a spinner row at the end while loading.

Focal points: Home, first shelf (top-left card). Search, the search field, then the first result row. Album/playlist/artist pages, hero artwork plus the accent Play button. Library tabs, first list row. Queue panel, the Now Playing row.

Interaction states required on every view: loading (skeleton rows or spinner, no layout jump), loaded, empty, error banner with Try again (from UiErrorKind), offline with cached data shown (non-blocking banner), re-auth banner (Phase 3).

Unavailable tracks (D-11, D-12): dimmed row, "Unavailable" badge (Label, Warning) after the title, click shows inline reason line under the row (and tooltip on badge): reason maps from error kind. Playback skips such rows with a toast. Runtime failure: toast with error kind, row becomes unavailable for the session, auto-skip; if all queue items fail, stop and show toast "Nothing in the queue can be played."

Toasts: bottom-right above the player bar (offset md), width 320, Secondary background with 4pt left border (Destructive for errors, Warning for skips, none for info), auto-dismiss 5 s, hover pauses, max 3 stacked, close button 32 target.

Motion: none beyond egui defaults; hover transitions 100ms; no animated page transitions.

Accessibility: AccessKit is enabled; every icon-only button has a label (tooltip text doubles as accessible name); focus ring 2pt accent; all actions reachable by keyboard.

---

## Copywriting Contract

| Element | Copy |
|---------|------|
| Primary CTA (heroes) | Play |
| Secondary CTA | Shuffle |
| Empty library tab | "Nothing here yet" / "Songs you add to your Apple Music library will appear here." |
| Empty search results | "No results for "{query}"" / "Check the spelling or try a different search." |
| Empty queue | "Queue is empty" / "Play a song or album to fill it." |
| Empty Home | "Nothing to show yet" / "Play some music and your recent activity will appear here." |
| Error banner (generic) | "Couldn't load this. Check your connection and try again." + Try again button |
| Rate limited | "Apple Music is busy. Retrying shortly." |
| Unavailable badge | "Unavailable" |
| Unavailable reason (inline) | "This track can't be played right now. It may not be available in your region or on your plan." |
| Skip toast | "Skipped "{title}": unavailable." |
| Playback failure toast | "Couldn't play "{title}" ({error kind}). Skipping to the next track." |
| All failed toast | "Nothing in the queue can be played." |
| Queue row actions | Play from here, Play next, Add to queue, Remove from queue |
| Toast after Play next / Add to queue | "Added to Up Next" / "Added to queue" |
| DEMO chip | "DEMO" ; tooltip "Running against the mock engine. No account needed." |
| Destructive: Remove from queue | No confirmation (reversible by re-adding) |
| Destructive: Clear cache (Settings) | "Clear cache": confirm "Clear cached library data and artwork? It will be downloaded again as you browse." buttons "Clear cache" (Destructive) and "Keep cache" |

Destructive actions in this phase: Clear cache only (Phase 4 behavior surfaced in Settings).

---

## Demo Mode Visual Requirements (IPC-04)

- Launch: `presto --demo [--fault <kind>]`; opens directly to Home, signed in, no sign-in panel.
- DEMO chip in sidebar status area and `Presto (Demo)` window title, always visible.
- Mock catalog artwork: mock engine serves deterministic solid-color or generated placeholder art so the artwork pipeline is exercised offline; no network image fetches in demo.
- Catalog must populate every view state above, including one unavailable track, one long title (truncation check), and more than 100 items in one library list (paging).

---

## Registry Safety

| Registry | Blocks Used | Safety Gate |
|----------|-------------|-------------|
| shadcn official | none (not applicable, native egui) | not required |
| Third-party | none | not applicable |

Crate sources (fastframe-* v0.4.1, crmne/egui fork) are pinned in docs/SPOTIFAST-SEAMS.md and verified fetchable in Phase 1.

---

## Checker Sign-Off

- [ ] Dimension 1 Copywriting: PASS
- [ ] Dimension 2 Visuals: PASS
- [ ] Dimension 3 Color: PASS
- [ ] Dimension 4 Typography: PASS
- [ ] Dimension 5 Spacing: PASS
- [ ] Dimension 6 Registry Safety: PASS

**Approval:** pending

# Phase 4: Data Layer and Cache - Context

**Gathered:** 2026-10-08
**Status:** Ready for planning

<domain>
## Phase Boundary

Library and catalog data flows from the engine into Presto models and is cached for fast, offline-ish browsing: library playlists/albums/artists/songs (paged), catalog search, recently played and recommendations home, storefront applied to catalog requests, visible error states, and disk caches for snapshots and artwork. UI views are ported in Phase 5; this phase delivers the data layer and the state shapes the UI reads.

</domain>

<decisions>
## Implementation Decisions

### Cache freshness
- **D-01:** Opening a view with a cached snapshot shows the cache immediately and revalidates in the background, then swaps in fresh data with a small "updated" marker.
- **D-02:** A snapshot older than 1 hour is stale and refetched on view open.
- **D-03:** Each view has a Refresh button that bypasses the TTL.
- **D-04:** Engine ready and auth-restored events invalidate and revalidate the open view.
- **D-05:** Recently Played revalidates on every open regardless of TTL. All other Home shelves follow the 1-hour policy.

### Library loading
- **D-06:** Lists load lazily, one page as the user nears the end of the list. No full background sync.
- **D-07:** Page size is 100 (Apple max).
- **D-08:** Sort choice (including alphabetical) is done server-side via Apple's sort parameter so paging stays lazy. Researcher verifies which library endpoints support it; where unsupported, fall back to Apple's order for that list and flag it.
- **D-09:** Must stay smooth with libraries up to ~10k songs: virtualized list data access and paged cache reads.

### Errors and offline
- **D-10:** With cached data present, errors show as an inline banner above the list and the cache stays visible.
- **D-11:** `rate_limited` auto-retries after `retry_after_ms` with a visible countdown plus a Retry button, capped at 3 attempts, then manual only.
- **D-12:** With no cache and a failed request, show a full-view error with kind-specific text (rate_limited, unavailable, upstream status, not_found, timeout) and Retry.
- **D-13:** Engine offline or restarting shows one global status chip. Views read the cache and pending requests fail fast. Consistent with Phase 3 D-12 (no API requests while signed out or expired).

### Cache storage
- **D-14:** Library and shelf snapshots are stored in SQLite (adds rusqlite).
- **D-15:** Artwork is cached on disk, capped at 500 MB with LRU eviction.
- **D-16:** Cache is keyed by account and storefront. Needs a stable account identifier from the engine without exposing any credential (IPC token guard still applies).
- **D-17:** Settings has a Clear cache button. Sign-out wipes the cache.

### Search
- **D-18:** Search-as-you-type with 300 ms debounce, showing hints while typing; full results on Enter or pause.
- **D-19:** Result groups: top result, songs, albums, artists, playlists.
- **D-20:** A scope toggle switches between catalog and my library.
- **D-21:** Keep the last 10 searches locally, clearable.
- **D-22:** Storefront comes from the account and is applied to every catalog request automatically (DATA-04).

### Home
- **D-23:** Home shows Recently Played first, then Apple's recommendation shelves in Apple's order with Apple's titles.
- **D-24:** Each shelf is a horizontal row of about 10 items with See all opening a full lazily-paged page.
- **D-25:** Recently Played also has a sidebar entry for its full list.

### Claude's Discretion
- SQLite schema, migration approach and file location under the XDG cache dir.
- Debounce and in-flight request cancellation details.
- Whether hints use a dedicated endpoint or the search endpoint with limits.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### IPC and errors
- `docs/PROTOCOL.md` §API requests, §Errors — `req` shape, closed error set, rate_limited retry_after_ms
- `crates/presto-ipc` — wire types and token guard (no credential fields)

### Prior phase decisions
- `.planning/phases/03-core-backend-supervisor-auth/03-CONTEXT.md` — D-12 (cache-only while signed out), D-09 sign-in panel, supervisor and ready semantics
- `.planning/phases/02-engine-feasibility-spike-gate/SPIKE-REPORT.md` — proven proxy path (`/v1/me/library/playlists`), api arity, 11 playlists no `next`

### Reference app
- `docs/SPOTIFAST-SEAMS.md` — spotifast API client/cache seams the UI depends on

### Requirements
- `.planning/REQUIREMENTS.md` DATA-01 to DATA-06
- `.planning/ROADMAP.md` Phase 4 success criteria

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/presto-core` (auth.rs, backoff.rs, mirror.rs, paths.rs, state.rs, supervisor.rs): supervisor, auth state, XDG paths; new data layer lives beside these.
- `crates/presto-ipc`: `req`/`res` with `ok`/`err` outcome and error codes already cover proxied reads.
- `crates/presto-engine-mock`: catalog and fault injection (rate_limited, slow, hang) for testing data layer without Widevine.

### Established Patterns
- Rust never calls api.music.apple.com; all catalog and library reads go through proxied `req` frames, engine owns auth.
- Timeouts: 20s read, 30s write. Backoff helpers exist in backoff.rs.

### Integration Points
- Data layer consumes supervisor ready/auth events (D-04, D-13) and exposes models and view state to the Phase 5 egui views.
- paths.rs for cache location.

</code_context>

<specifics>
## Specific Ideas

No specific requirements beyond the decisions above. Web player (music.apple.com) layout is the reference for Home and search results.

</specifics>

<deferred>
## Deferred Ideas

- Full background library sync for complete offline coverage — revisit if lazy cache proves too sparse offline.
- Client-side sort over a fully synced library — only if server-side sort is unavailable for key endpoints.

Note: the library search scope toggle (D-20) goes slightly beyond DATA-02 (catalog search); kept at user request.

</deferred>

---

*Phase: 04-data-layer-and-cache*
*Context gathered: 2026-10-08*

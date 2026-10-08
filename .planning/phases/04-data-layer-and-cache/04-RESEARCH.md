# Phase 4: Data Layer and Cache - Research

**Researched:** 2026-10-08
**Domain:** Rust data layer over proxied Apple Music API reads, SQLite snapshot cache, disk artwork cache
**Confidence:** MEDIUM (Apple endpoint behavior is partly undocumented; a live probe is required, see Open Questions)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Opening a view with a cached snapshot shows the cache immediately and revalidates in the background, then swaps in fresh data with a small "updated" marker.
- **D-02:** A snapshot older than 1 hour is stale and refetched on view open.
- **D-03:** Each view has a Refresh button that bypasses the TTL.
- **D-04:** Engine ready and auth-restored events invalidate and revalidate the open view.
- **D-05:** Recently Played revalidates on every open regardless of TTL. All other Home shelves follow the 1-hour policy.
- **D-06:** Lists load lazily, one page as the user nears the end of the list. No full background sync.
- **D-07:** Page size is 100 (Apple max).
- **D-08:** Sort choice (including alphabetical) is done server-side via Apple's sort parameter so paging stays lazy. Researcher verifies which library endpoints support it; where unsupported, fall back to Apple's order for that list and flag it.
- **D-09:** Must stay smooth with libraries up to ~10k songs: virtualized list data access and paged cache reads.
- **D-10:** With cached data present, errors show as an inline banner above the list and the cache stays visible.
- **D-11:** `rate_limited` auto-retries after `retry_after_ms` with a visible countdown plus a Retry button, capped at 3 attempts, then manual only.
- **D-12:** With no cache and a failed request, show a full-view error with kind-specific text (rate_limited, unavailable, upstream status, not_found, timeout) and Retry.
- **D-13:** Engine offline or restarting shows one global status chip. Views read the cache and pending requests fail fast. Consistent with Phase 3 D-12 (no API requests while signed out or expired).
- **D-14:** Library and shelf snapshots are stored in SQLite (adds rusqlite).
- **D-15:** Artwork is cached on disk, capped at 500 MB with LRU eviction.
- **D-16:** Cache is keyed by account and storefront. Needs a stable account identifier from the engine without exposing any credential (IPC token guard still applies).
- **D-17:** Settings has a Clear cache button. Sign-out wipes the cache.
- **D-18:** Search-as-you-type with 300 ms debounce, showing hints while typing; full results on Enter or pause.
- **D-19:** Result groups: top result, songs, albums, artists, playlists.
- **D-20:** A scope toggle switches between catalog and my library.
- **D-21:** Keep the last 10 searches locally, clearable.
- **D-22:** Storefront comes from the account and is applied to every catalog request automatically (DATA-04).
- **D-23:** Home shows Recently Played first, then Apple's recommendation shelves in Apple's order with Apple's titles.
- **D-24:** Each shelf is a horizontal row of about 10 items with See all opening a full lazily-paged page.
- **D-25:** Recently Played also has a sidebar entry for its full list.

### Claude's Discretion
- SQLite schema, migration approach and file location under the XDG cache dir.
- Debounce and in-flight request cancellation details.
- Whether hints use a dedicated endpoint or the search endpoint with limits.

### Deferred Ideas (OUT OF SCOPE)
- Full background library sync for complete offline coverage.
- Client-side sort over a fully synced library.
- Note: the library search scope toggle (D-20) goes slightly beyond DATA-02; kept at user request.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DATA-01 | Library playlists, albums, artists, songs, paginated | Library endpoints, `next`/offset paging, page-row cache, sort probe |
| DATA-02 | Catalog search | `/v1/catalog/{sf}/search`, hints endpoint, debounce + generation counter |
| DATA-03 | Recently played and recommendations home | `/v1/me/recent/played` (limit 10), `/v1/me/recommendations` |
| DATA-04 | Storefront read from account, applied to catalog requests | `/v1/me/storefront` (already in mock), path builder |
| DATA-05 | Proxy errors and rate limits surface as UI states | `ErrorKind` to `UiError` map, retry state machine |
| DATA-06 | Artwork and snapshots cached on disk | rusqlite page cache, hashed-file artwork LRU |
</phase_requirements>

## Summary

All reads already flow through `CoreHandle::request(ApiRequest) -> Outcome` (`crates/presto-core/src/supervisor.rs:171`). The bridge forwards `req.path` and `req.query` verbatim to `mk.api.music(path, query)` and returns the JSON body (`engine/bridge.js:107`). So the data layer is pure Rust: build requests, parse JSON into models, cache raw pages, expose view state. No IPC change is needed, with one exception: the mock lacks a `rate_limited` fault (CONTEXT claims it has one; `crates/presto-ipc/src/fault.rs` and `PROTOCOL.md` list only none, hang, crash, auth_expired, slow). Adding one changes the IPC schema snapshot and PROTOCOL.md (a test enforces both).

Cache raw JSON pages in SQLite, one row per (account key, request key, offset). Parse into typed models on read. This is the simplest design that satisfies D-09 (a 100-item page is one indexed row read) and keeps the cache valid across model changes. Artwork: fetch with reqwest from Apple's CDN (allowed by CLAUDE.md; only api.music.apple.com is banned), store as hashed files, evict by mtime.

Three things cannot be settled from docs and need a live probe before the plan commits: server-side `sort` support (D-08), the account identifier (D-16), and exact param limits on recently played / search / recommendations. Plan a short Wave 0 probe task using the existing `crates/presto-core/examples/live.rs`.

**Primary recommendation:** Add `data/` modules inside `presto-core` (client, models, store, artwork, views); cache raw page JSON in SQLite; expose per-view `watch` state to the UI; probe sort and account key live first.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| rusqlite | 0.40.2 (`cargo search`, 2026-10-08) with feature `bundled` | snapshot cache, search history | D-14. `bundled` avoids a system libsqlite3 dependency (matters for AUR/Flatpak) |
| reqwest | 0.12 per project stack; 0.13.5 is current on crates.io | artwork fetch only | CLAUDE.md pins 0.12 rustls. Take 0.13.x only if it needs no extra feature work; otherwise 0.12 |
| sha2 | 0.11.0 | stable hash of artwork URL to filename | `DefaultHasher` is not stable across Rust versions |
| tokio (existing) | 1.x | spawn_blocking for SQLite, timers, watch channels | already in workspace |
| serde / serde_json (existing) | 1.x | page parsing | already in workspace |
| tempfile (existing) | 3 | test cache dirs | already in workspace |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Raw-JSON page rows | Normalized tables per entity | Normalized needed only for local sort/filter, which is deferred. Skip |
| `sha2` filename hash | std `DefaultHasher` | Unstable across toolchains, would orphan the cache on upgrade |
| `directories` crate | hand-rolled XDG like `Paths::from_env` | `paths.rs` already hand-rolls XDG; follow it (`XDG_CACHE_HOME`, else `$HOME/.cache`) |
| `lru` crate | mtime-based eviction on disk | Eviction state must survive restarts; files plus mtime need no extra structure |

**Installation (workspace Cargo.toml additions, then `presto-core` deps):**
```bash
# workspace.dependencies
# rusqlite = { version = "0.40", features = ["bundled"] }
# reqwest  = { version = "0.12", default-features = false, features = ["rustls-tls"] }
# sha2     = "0.11"
```
Verify with `cargo tree -p presto-core | grep -E "rusqlite|reqwest"` after adding; reqwest 0.12 vs 0.13 feature names differ, check at add time.

## Architecture Patterns

### Recommended Project Structure
```
crates/presto-core/src/data/
├── mod.rs        # DataHandle: open(view), load_more(view), refresh(view), search(...)
├── client.rs     # ApiClient: builds ApiRequest, applies storefront, semaphore, timeouts, error map
├── models.rs     # Song/Album/Artist/Playlist/Shelf/Artwork, tolerant parsing (serde default, Unknown variant)
├── store.rs      # SQLite: pages, search_history; one Connection on a dedicated thread
├── artwork.rs    # URL template expand, disk LRU, reqwest fetch
├── view.rs       # ViewKey, ListState<T>, UiError, RetryState
└── account.rs    # account key + storefront resolution
crates/presto-core/paths.rs  # add cache dir: $XDG_CACHE_HOME/presto/{presto.db, artwork/}
```
Add `cache` and `artwork` to `Paths` and `prepare()` using `ensure_private_dir` (0700).

### Pattern 1: Cache-then-revalidate view state (D-01..D-05)
**What:** `DataHandle::open(ViewKey)` returns `watch::Receiver<ViewState>`. It first publishes the cached pages (`from_cache=true`), then, if stale (>1h, or Recently Played, or refresh), issues the page-0 request and publishes fresh data with `updated_at` set so the UI can show the marker.
**Shape the UI reads (Phase 5, polled each frame like spotifast `Backend::poll`):**
```rust
pub struct ListState<T> {
    pub items: Vec<T>,            // all loaded pages concatenated
    pub total: Option<u64>,       // meta.total when present, for scrollbar sizing
    pub has_more: bool,           // from `next`, never from len < limit
    pub phase: Phase,             // Idle | Loading | Revalidating | LoadingMore
    pub from_cache: bool,
    pub fetched_at: Option<SystemTime>,
    pub just_updated: bool,       // drives the "updated" marker
    pub error: Option<UiError>,   // banner if items non-empty, full-view if empty (D-10/D-12)
}
pub struct UiError { pub kind: UiErrorKind, pub retry_at: Option<Instant>, pub attempts: u8, pub auto_retry: bool }
```
Revalidate fetches page 0 only. If page 0 differs, replace page 0 and drop cached pages beyond it (their offsets may have shifted); they reload lazily. Do not splice.

### Pattern 2: Raw page cache keyed by request
Key = `(account_key, normalized_request, offset)` where `normalized_request` is path + sorted query minus `offset`/`limit`. Table:
```sql
CREATE TABLE IF NOT EXISTS pages(
  account TEXT NOT NULL, req TEXT NOT NULL, off INTEGER NOT NULL,
  fetched_at INTEGER NOT NULL, body BLOB NOT NULL,
  PRIMARY KEY(account, req, off)) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS search_history(
  account TEXT NOT NULL, term TEXT NOT NULL, at INTEGER NOT NULL, PRIMARY KEY(account, term));
PRAGMA journal_mode=WAL; PRAGMA user_version = 1;
```
Migrations: `PRAGMA user_version` match; on unknown newer version or open failure, delete the db file and recreate (it is a cache). `account` includes storefront.
TTL is judged from page 0's `fetched_at`.

### Pattern 3: Request client with generation counters (cancellation)
`CoreHandle::request` has no cancel. Dropping the future does not stop the engine call. So: tag each logical request with a per-view/per-search generation `AtomicU64`; discard results whose generation is stale (spotifast does the same, see SPOTIFAST-SEAMS `generation`). Bound concurrency with a `tokio::sync::Semaphore` (4 permits) so a fast scroll or typing burst cannot queue dozens of 20 s calls in the engine; search supersedes by generation before acquiring a permit. Check `CoreState` first: if `engine != Ready` or `auth` blocks traffic (`AuthMachine::allows_traffic`), return `UiErrorKind::Offline` immediately without sending (D-13).

### Pattern 4: Error mapping (DATA-05)
| `ErrorKind` | UI kind | Behavior |
|---|---|---|
| `RateLimited{retry_after_ms}` | RateLimited | auto-retry after `retry_after_ms.unwrap_or(backoff step)`, countdown via `retry_at`, 3 attempts, then manual |
| `Timeout` | Timeout | manual Retry |
| `Unavailable` | Unavailable | manual Retry |
| `Upstream{status}` | Upstream(status) | manual Retry |
| `NotFound` | NotFound | no retry |
| `AuthExpired` | AuthExpired | no request retry; supervisor/auth flow owns re-sign-in; cache stays visible |
| `Internal` | Internal | manual Retry |
Bridge emits `retry_after_ms: null` for 429 (`bridge.js:47`), so the `None` default path is the common live case. Reuse `backoff.rs` or a fixed 5 s/10 s/20 s ladder. Countdown is a timestamp; the UI repaints from `retry_at`, no ticking task.

### Pattern 5: Storefront and account key (DATA-04, D-16, D-22)
- Storefront: `GET /v1/me/storefront` -> `data[0].id` (the mock already serves it, `catalog.rs:139`). Fetch once after Ready+signed_in, cache in memory and in the db, re-fetch on auth transitions. All catalog paths are built by one `catalog_path(&self, rest)` helper that prefixes `/v1/catalog/{sf}/`; no call site formats a catalog path by hand.
- Account identifier: no Apple endpoint exposes a stable user id without a token, and no IPC field may be token-like. Recommendation: key = `storefront + ":" + local install id`, where the install id is a random UUID Presto writes in its own state dir (0600) on first run. Sign-out wipes the cache (D-17), so an account switch always passes through a wipe. Residual gap: an `expired` -> re-auth as a different Apple ID with the same storefront would show the old cache until revalidated; accept and note it. If the user wants true account binding, the engine would need a new non-secret `account_hint` event, which is an IPC change. Raise as an open question rather than building it.

### Pattern 6: SQLite on a blocking thread
rusqlite is sync. Own one `Connection` on a dedicated `std::thread` fed by an `mpsc` channel (or `tokio::task::spawn_blocking` with `Arc<Mutex<Connection>>`). WAL mode. Page reads are single-row primary-key lookups, so 10k songs = 100 rows read on demand, never all at once. Parse JSON on the same blocking thread or in `spawn_blocking`, not on the UI thread.

### Pattern 7: Artwork cache (DATA-06, D-15)
- Apple artwork URLs are templates: `https://is1-ssl.mzstatic.com/image/thumb/.../{w}x{h}bb.jpg`. Substitute `{w}`/`{h}`; request from a small fixed set of sizes (e.g. 160, 320, 640) so one cached file serves many views. (Format suffix variants like `bb`/`sr` come from the template itself; keep them.)
- Filename: hex `sha256(expanded_url)` under `artwork/<first 2 hex>/`. On hit, `File::set_modified(now)` (touch) for LRU. On write, if total size > 500 MB, delete oldest-mtime files until under ~450 MB. Track total size in memory after one startup scan.
- Write via temp file + rename so a crash never leaves a partial image. Dedupe in-flight fetches by URL (`HashMap<String, Shared future>` or a pending set).
- reqwest is independent of the engine, so artwork loads when the engine is down but the network is up; with no network, serve disk hits only and show a placeholder otherwise.
- Library items sometimes have no `artwork` attribute; models must hold `Option<Artwork>`.

### Pattern 8: Search (DATA-02, D-18..D-21)
- Debounce in the data layer: `search(term)` bumps a generation and sleeps 300 ms; if the generation moved, return. Hints: `GET /v1/catalog/{sf}/search/hints?term=&limit=10` (dedicated, cheap, MEDIUM confidence). Full results on Enter or after the pause: `GET /v1/catalog/{sf}/search?term=&types=songs,albums,artists,playlists&limit=25&with=topResults` (MEDIUM; the top-result shape needs live confirmation). Per-type limit max is 25; page more per group with `offset`.
- Library scope: `GET /v1/me/library/search?term=&types=library-songs,library-albums,library-artists,library-playlists&limit=25`.
- Search results are not snapshot-cached (only history is kept, D-21); do not write them to `pages`.
- History: `search_history` table, keep 10 most recent per account, `DELETE ... WHERE term NOT IN (SELECT term ... ORDER BY at DESC LIMIT 10)`, plus a clear.

### Pattern 9: Home (DATA-03, D-23..D-25)
- Recently Played: `GET /v1/me/recent/played?limit=10&offset=N`. Apple caps `limit` at 10 here (forum evidence, MEDIUM), so D-07's 100 does not apply; page size for this list is 10. Mixed resource types (albums, playlists, stations); model as an enum with an `Other` fallthrough. `/v1/me/recent/played/tracks` returns songs (limit reportedly up to 30, LOW, probe).
- Recommendations: `GET /v1/me/recommendations` returns groups; each has a title (`attributes.title.stringForDisplay`) and `relationships.contents.data` (MEDIUM from training, probe). Keep Apple's order and titles. Shelf row = first ~10 contents; See all = lazily paged `.../relationships/contents` or the group's own `href` (probe which works; if neither, See all shows the shelf's full `contents` list from the first response).
- Recently Played revalidates on every open (D-05), others follow 1 h TTL.

### Anti-Patterns to Avoid
- **Decide `has_more` from `items.len() < limit`:** use `next` presence. Apple can return short pages.
- **Hand-formatting catalog paths:** storefront leaks stale or missing; always go through the client helper.
- **Holding a `Connection` mutex on the UI thread:** causes frame hitches at 10k songs.
- **Caching `res` for `auth_expired`/error outcomes:** only cache `Ok` bodies.
- **Logging response bodies at info:** PROTOCOL.md forbids it.
- **Splicing fresh page 0 onto old pages 1..n:** offsets shift; invalidate the tail.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Embedded DB | file-per-snapshot JSON store | rusqlite `bundled` | atomicity, WAL, indexed lookups; D-14 |
| Stable URL hash | custom hash | `sha2` | cross-version stable |
| Atomic file write | direct write | tempfile in same dir + `rename` | crash safety |
| Backoff | new timer logic | `crates/presto-core/src/backoff.rs` or fixed ladder | already tested |
| Error type for the wire | new enums on the wire | existing `presto_ipc::ErrorKind` | closed set, schema-tested |
| Private dirs | mkdir + chmod | `paths::ensure_private_dir` | symlink and ownership checks |

## Runtime State Inventory
Not a rename or migration phase. Omitted.

## Common Pitfalls

### Pitfall 1: Server-side sort may not exist
**What goes wrong:** D-08 assumes a `sort` param. Apple's documented library list endpoints (songs, albums, artists, playlists) list `limit`, `offset`, `include`, `extend`, `l`, and document no `sort`; the albums endpoint says results come "in alphabetical order". Web search found no documentation of `sort=` on `amp-api` either.
**How to avoid:** Wave 0 live probe: for each of the four endpoints, request with no sort, `sort=name`, `sort=-name`, `sort=dateAdded`, `sort=-dateAdded`; record whether order changes (compare first 5 names) and whether an unsupported value errors (400) or is ignored. Encode result as a `const SORT_SUPPORT` table; unsupported lists show Apple's order and the sort control is hidden or disabled (D-08 fallback). Confidence that sort works: LOW. Likely outcome: albums/artists/playlists alphabetical, songs in some fixed order, no user sort; the deferred client-side sort then becomes the only path and the user should be told.

### Pitfall 2: Recently played page cap is 10
`limit>10` errors on `/v1/me/recent/played`. Use page size per endpoint, not a global 100. Make page size a field of the view definition.

### Pitfall 3: Bridge is GET-only and 4 MB capped
`bridge.js` rejects non-GET and any response over 4,000,000 chars (`response too large; lower limit`, mapped to `internal`). 100 songs with default attributes is far below this, but never add `include=`/`extend=` params that balloon pages (e.g. `extend=editorialNotes` on albums). Keep requests minimal; on that specific `internal` message, halve the limit and retry once.

### Pitfall 4: Offline path must not touch the engine
Views must load cache without any request. Gate network calls on `CoreState` (D-13). Also snapshots keyed by account need the account key available while the engine is down: persist the last known key and storefront in the db/state dir so offline start can still read cache. Wipe only on explicit sign-out or Clear cache, not on `expired` (AUTH-02 keeps cached data visible).

### Pitfall 5: Sign-out wipe vs expired
Phase 3 distinguishes `signed_out` from `expired`. D-17 wipes on sign-out. Wipe on the `auth` event `signed_out` observed after having been `signed_in` in this session (a transition), not on the initial `signed_out` at startup before restore completes, or the cache is destroyed on every cold start. Verify with the Phase 3 auth tests' event order.

### Pitfall 6: Library ids differ from catalog ids
Library items use ids like `l.xxxx`, `i.xxxx`, `p.xxxx`. Playback (Phase 5) may need the catalog id from `attributes.playParams` (use `playParams.id`/`catalogId`). Store `play_params` raw in the model now so Phase 5 is not blocked; do not decide playback semantics here.

### Pitfall 7: Rate limit retry stampede
Several views revalidating at engine-ready plus scroll prefetch can trigger 429. The semaphore (Pattern 3) plus only revalidating the single open view (D-04) keeps request volume low. Do not prefetch beyond one page.

### Pitfall 8: egui image loading is Phase 5
Phase 4 hands back a file path or bytes plus a ready/pending/failed state per URL; do not pull egui or image decoding into presto-core.

## Code Examples

### Request with storefront applied
```rust
// Source: crates/presto-ipc/src/request.rs (ApiRequest), presto-core supervisor.rs (request)
fn catalog_req(&self, rest: &str, q: &[(&str, &str)]) -> ApiRequest {
    let mut r = ApiRequest::get(format!("/v1/catalog/{}/{rest}", self.storefront));
    r.query = q.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    r
}
// paging: call with offset; stop when response has no "next"
```

### Page parse tolerant of unknown fields
```rust
#[derive(Deserialize)]
struct Page<T> { #[serde(default)] data: Vec<T>, next: Option<String>, #[serde(default)] meta: Meta }
#[derive(Deserialize, Default)]
struct Meta { total: Option<u64> }
```

### Eviction sketch
```rust
// after write: if total > CAP { sort files by mtime asc; delete until total <= CAP * 9/10 }
```

## State of the Art

| Old Approach | Current Approach | Impact |
|--------------|------------------|--------|
| spotifast `src/api/`, `http.rs`, `limiter.rs` client | proxied `req` frames | no token, no HTTP client for the API; limiter replaced by `rate_limited` handling |
| spotifast Spotify paging (`next` URL) | Apple `next` relative path with `offset` | parse offset from `next` or just track offset ourselves (simpler, do this) |

## Open Questions

1. **Does any library list endpoint honor `sort`?** Known: undocumented. Unknown: live behavior. Recommendation: Wave 0 probe, then lock the `SORT_SUPPORT` table. Surface the result to the user because D-08 may collapse to "Apple's order only".
2. **Account identifier.** Recommendation above (install UUID + storefront) avoids an IPC change but cannot detect an Apple ID switch via re-auth. Ask the user if that is acceptable.
3. **Top-result and recommendations response shapes**, `/v1/me/recommendations` See-all paging, and `recent/played/tracks` limit. Probe live and save sanitized JSON fixtures into `crates/presto-engine-mock/` for the mock.
4. **Mock gaps (plan work):** `catalog.rs` lacks `recent/played`, `recommendations`, `search/hints`, `me/library/search`, sort handling, and `Upstream`/`rate_limited` injection. Add a `rate_limited` (and ideally `upstream`) mock fault: touches `presto-ipc/src/fault.rs`, the insta schema snapshot, PROTOCOL.md (doc coverage test), and mock CLI parsing. The mock library list is tiny; add a generated 10k-song library behind a flag for D-09 testing.
5. **reqwest 0.12 vs 0.13.** Project doc says 0.12; crates.io shows 0.13.5. Pick at implementation; no API difference matters here.
6. **Apple's data Terms** (STATE blocker "read Apple Media Services terms") may bear on persisting API responses and artwork. Not researched here; flag to the user for the PKG-02 doc.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test (built-in), insta for snapshots, tempfile |
| Config file | workspace `Cargo.toml`; no extra config |
| Quick run command | `cargo test -p presto-core data::` |
| Full suite command | `cargo build -p presto-engine-mock && cargo test --workspace` |

Integration tests in `crates/presto-core/tests/` use `tests/common/mod.rs` (`Rig`, `mock_bin`, `fast_timings`); the mock binary must be built first.

### Phase Requirements to Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DATA-01 | paging follows `next`, stops when absent; 10k mock songs page lazily | integration (mock) | `cargo test -p presto-core --test data_library` | Wave 0 |
| DATA-01 | page row read is bounded (no full-table load) | unit (store) | `cargo test -p presto-core data::store` | Wave 0 |
| DATA-02 | debounce drops superseded term; stale generation discarded | unit (tokio paused time) | `cargo test -p presto-core data::search` | Wave 0 |
| DATA-02 | search results/groups parse | integration (mock) | `cargo test -p presto-core --test data_search` | Wave 0 |
| DATA-03 | recently played revalidates every open; shelves obey TTL | unit (fake clock) | `cargo test -p presto-core data::view` | Wave 0 |
| DATA-04 | every catalog path carries the account storefront | unit | `cargo test -p presto-core data::client` | Wave 0 |
| DATA-05 | each `ErrorKind` maps to a UI state; 429 auto-retry max 3 then manual; banner vs full-view | unit + integration (needs mock `rate_limited` fault) | `cargo test -p presto-core --test data_errors` | Wave 0 |
| DATA-06 | cache survives engine kill: views serve cache offline | integration (mock `--fault crash`/hang) | `cargo test -p presto-core --test data_offline` | Wave 0 |
| DATA-06 | artwork LRU evicts oldest over cap; atomic write; hash stable | unit (tempdir, small cap) | `cargo test -p presto-core data::artwork` | Wave 0 |
| D-17 | sign-out wipes, expired does not, Clear cache wipes | integration | `cargo test -p presto-core --test data_wipe` | Wave 0 |

Artwork network fetch is tested against a local `tokio` TCP stub serving bytes, not the real CDN. Live Apple behavior (sort, shapes) is manual via the probe example, results recorded in a plan SUMMARY.

### Sampling Rate
- **Per task commit:** `cargo test -p presto-core data::`
- **Per wave merge:** full suite command
- **Phase gate:** full suite green plus recorded live probe output

### Wave 0 Gaps
- [ ] `crates/presto-core/tests/data_*.rs` (library, search, errors, offline, wipe)
- [ ] Mock catalog additions and `rate_limited` fault (Open Question 4), schema snapshot and PROTOCOL.md updates
- [ ] 10k-song mock library flag
- [ ] Live probe example for sort, shapes, limits (extend `crates/presto-core/examples/live.rs`)
- [ ] Add rusqlite, reqwest, sha2 to workspace deps

## Sources

### Primary (HIGH confidence)
- Repo code read: `docs/PROTOCOL.md`, `crates/presto-ipc/src/request.rs`, `crates/presto-core/src/{paths,state,supervisor}.rs`, `crates/presto-engine-mock/src/catalog.rs`, `engine/bridge.js`, `tests/common/mod.rs`, `SPIKE-REPORT.md`
- `cargo search` (2026-10-08): rusqlite 0.40.2, reqwest 0.13.5, sha2 0.11.0

### Secondary (MEDIUM confidence)
- https://developer.apple.com/documentation/applemusicapi/get-recently-played-resources.md (endpoint, `next` offset, mixed types)
- https://developer.apple.com/documentation/applemusicapi/get-all-library-albums.md (alphabetical, `next` offset)
- https://developer.apple.com/forums/thread/669720 (recent/played limit cap 10, offset works)
- https://glama.ai/mcp/servers/Cifero74/mcp-apple-music/tools/get_library_songs (limit 1-100, no sort param used)

### Tertiary (LOW confidence, validate by probe)
- Search/hints/recommendations/recent-tracks parameter details and response shapes (Apple doc pages for these returned 404 to fetch; from training knowledge)
- Any `sort` support on library endpoints

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH for rusqlite/sha2/tokio, MEDIUM for reqwest version choice
- Architecture: HIGH for fit with existing core (read directly); MEDIUM for Apple response shapes
- Pitfalls: MEDIUM; sort and limits unverified live

**Research date:** 2026-10-08
**Valid until:** 2026-11-07 (Apple endpoints undocumented-ish, probe results supersede)

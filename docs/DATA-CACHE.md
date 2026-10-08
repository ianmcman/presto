# Data cache

Location: `$XDG_CACHE_HOME/presto/presto.db` (SQLite) and `artwork/`, both 0700. Install id: `$XDG_STATE_HOME/presto/install-id`, 0600.

Cached:
- Raw page JSON per account, request and offset.
- Search history, 10 terms.
- Artwork files named by sha256 of the URL, LRU at 500 MB.

Keying: `storefront:install-id` (D-16). The storefront is re-read on every engine ready or auth restored transition and persisted so offline starts can still key the cache.

Revalidation: pages older than 1 h are refetched on open; Recently Played on every open; Refresh bypasses the TTL. On ready or auth restored, the focused view revalidates and the others are marked stale.

Offline: cached pages and artwork are served and no request is sent. A stale first page shows with an Offline error banner.

Wipe: a mid-session sign-out (same engine run) and Clear cache wipe pages, search history, storefront and artwork. A startup signed_out or an expired session never wipes. Clear cache keeps sign-in and reloads the open view.

Known gap: re-authenticating as a different Apple ID in the same storefront after an expiry shows the previous account's cache until each view revalidates, because Presto holds no account identifier (no IPC change by design).

PKG-02: Apple's terms may bear on persisting API responses and artwork; not yet reviewed.

## Detail and track lists

`ViewKey::Detail` (album, playlist, artist head with inline lists, not paged) and `ViewKey::Tracks` (paged track list) go through the same cache, TTL, offline and retry rules as library views. Paths carry a literal `{sf}` that `DataHandle` replaces with the cached storefront when building the request; with no storefront known the load fails as `Internal`. Tracks page at 100 (unverified live; 05-11 checks).

# Phase 4: Data Layer and Cache - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md.

**Date:** 2026-10-08
**Phase:** 4-Data Layer and Cache
**Areas discussed:** Cache freshness, Library loading, Error states, Cache storage, Search, Home

---

## Cache freshness
Chosen: show cache then refresh in background; 1 hour TTL; per-view Refresh; revalidate on reconnect/re-auth. Alternatives: TTL-gated display, always fetch; 5 min / no TTL; no manual refresh.

## Library loading
Chosen: lazy pages, size 100, alphabetical sort choice, ~10k songs smooth. Conflict between alphabetical sort and lazy paging resolved with server-side sort. Alternatives: background full sync, size 50, Apple order only.

## Error states
Chosen: inline banner over cache, auto-retry with countdown (3 max) plus Retry, full-view error without cache, global offline chip. Alternatives: replace view, manual retry only, generic error, per-view banner.

## Cache storage
Chosen: SQLite, 500 MB artwork LRU, keyed by account + storefront, Clear cache button and wipe on sign-out. Alternatives: JSON files, 200 MB, shared cache.

## Search
Chosen: search-as-you-type 300 ms with hints, all five result groups, catalog/library toggle, last 10 searches. Alternatives: Enter only, fewer groups, catalog only, no history.

## Home
Chosen: Apple's shelves with Recently Played first, ~10 items plus See all, sidebar entry for Recently Played, Recently Played revalidates each open. Alternatives: fixed shelf set, no See all, Home-only recents.

## Claude's Discretion
SQLite schema/location, debounce and cancellation details, hints endpoint choice.

## Deferred Ideas
Full background sync, client-side sort over full library.

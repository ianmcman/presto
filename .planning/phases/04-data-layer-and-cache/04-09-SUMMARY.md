---
phase: 04-data-layer-and-cache
plan: 09
status: done
requirements-completed: [DATA-04, DATA-06]
key-files:
  modified:
    - crates/presto-core/src/data/mod.rs
    - crates/presto-core/tests/data_wipe.rs
    - crates/presto-core/tests/data_offline.rs
    - docs/DATA-CACHE.md
---

# Phase 4 Plan 09: Lifecycle, offline, wipe

`DataHandle` now watches `CoreState`: wipes only on a mid-session sign-out, re-reads the storefront and revalidates the focused view when the engine becomes ready or auth is restored, and exposes `clear_cache()`. 4 wipe tests, 3 offline tests, 2 unit tests; `cargo test --workspace` is green.

## Commits

- a3f46a8: watcher, wipe, clear_cache, data_wipe tests
- 7594b32: data_offline tests and docs/DATA-CACHE.md

## Deviations

- Watcher also fires when `restarts` changes while still usable, so a coalesced restart still revalidates.
- Integration tests cannot use the http artwork stub (the loopback allowance is `cfg(test)` in the lib, and artwork.rs was off limits). Artwork is seeded by writing `file_for(..)` directly, so "stub gone" is trivially true; the fetch path is covered by the artwork unit tests.
- Offline tests age page 0 past the 1 h TTL so run 2 attempts the network and publishes the Offline error; a fresh cached page shows no error.
- startup_signed_out test asserts cached items, not an error, for the same reason.
- Known race (ponytail comment): an in-flight fetch past its gen check may write one page after a wipe.

## Self-Check: PASSED

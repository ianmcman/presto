---
phase: 04-data-layer-and-cache
plan: 03
subsystem: data
tags: [sqlite, rusqlite, cache, paths, errors]
requires: []
provides:
  - presto_core::data::error::UiErrorKind
  - presto_core::data::store::{Store, req_key, now_ms, CachedPage}
  - Paths::{cache, artwork, db, install_id} and Paths::install_id()
  - stub modules data::{client, artwork, models, view, search}
affects: [04-04, 04-05, 04-06, 04-07, 04-08]
tech-stack:
  added: [rusqlite 0.40 (bundled), reqwest 0.12 (rustls), sha2 0.11]
  patterns: [swallow-and-log cache store, disposable db recreated on bad version]
key-files:
  created: [crates/presto-core/src/data/mod.rs, crates/presto-core/src/data/error.rs, crates/presto-core/src/data/store.rs, crates/presto-core/src/data/client.rs, crates/presto-core/src/data/artwork.rs, crates/presto-core/src/data/models.rs, crates/presto-core/src/data/view.rs, crates/presto-core/src/data/search.rs]
  modified: [Cargo.toml, crates/presto-core/Cargo.toml, crates/presto-core/src/lib.rs, crates/presto-core/src/paths.rs]
key-decisions:
  - "Cache dir from XDG_CACHE_HOME via a pure cache_base helper, not env mutation in tests"
metrics:
  completed: 2026-10-08
---

# Phase 4 Plan 03: Data layer foundation Summary

SQLite page cache (one row per account/request/offset, WITHOUT ROWID, user_version 1), UiErrorKind mapping, 0700 cache dirs and a 0600 install id.

## Deviations from Plan

None. One compile fix: `page_count` reads `i64` and casts, since rusqlite has no `FromSql` for `u64`.

## Verification

`cargo test -p presto-core` green (38 lib tests, existing suites unchanged). Clippy shows no warnings in data/ or paths.rs.

## Commits

- fb36bae: deps, paths, install id, data tree, UiErrorKind
- 11e9ebb: SQLite store

## Self-Check: PASSED

---
phase: 04-data-layer-and-cache
plan: 05
subsystem: data
tags: [artwork, cache, reqwest, lru]
requires: [04-03]
provides: [ArtCache, expand, file_for]
key-files:
  modified: [crates/presto-core/src/data/artwork.rs]
requirements-completed: [DATA-06]
completed: 2026-10-08
---

# Phase 4 Plan 05: Artwork Cache Summary

Disk artwork cache (`ArtCache`) with sha256-sharded paths, atomic `.part` writes, deduped in-flight fetches, 60 s failure backoff, and mtime-LRU eviction to 90% of a 500 MB cap.

## Deviations from Plan

None. 9 tests pass against a local TCP stub. No egui or image crates in the tree.

Commit: c9751de

## Self-Check: PASSED

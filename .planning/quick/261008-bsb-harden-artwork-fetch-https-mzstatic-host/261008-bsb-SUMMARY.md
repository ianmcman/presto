---
phase: quick-261008-bsb
plan: 01
subsystem: presto-core/data/artwork
tags: [security, artwork]
key-files:
  modified: [crates/presto-core/src/data/artwork.rs]
decisions:
  - "cfg(test)-only allowance for http://127.0.0.1 keeps stub tests working; compiled out of release"
metrics:
  completed: 2026-10-08
---

# Quick 261008-bsb: Harden artwork fetch

`ArtCache::fetch` now rejects any URL that is not https on `*.mzstatic.com` (no userinfo, no explicit port) before any request. The client never follows redirects and requires a 2xx. Bodies over `MAX_ART_BYTES` (10 MB) are rejected by Content-Length and by streamed byte count, before anything touches disk.

Commit: e882b55

Tests: 15 artwork tests pass (7 new: allowlist, disallowed-url no-hit, redirect, Content-Length cap, streamed cap, exact-cap boundary). Release build clean.

## Deviations from Plan

None.

## Deferred Issues

`cargo clippy -D warnings` fails on pre-existing `collapsible_if` lints (artwork.rs lines 65 and 114 in old code, plus data/mod.rs, search.rs, view.rs). Not touched by this task.

## Self-Check: PASSED

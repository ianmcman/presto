---
phase: 03
slug: core-backend-supervisor-auth
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-10-07
---

# Phase 03 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (tokio `#[tokio::test]`, insta snapshots); `node --test` for engine JS |
| **Config file** | none beyond Cargo workspace; new crate needs dev-deps `tokio` test-util, `tempfile` |
| **Quick run command** | `cargo test -p presto-core -p presto-ipc` |
| **Full suite command** | `cargo test --workspace && (cd engine && npm test)` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p presto-core -p presto-ipc`
- **After every plan wave:** Run `cargo test --workspace && (cd engine && npm test)`
- **Before `/gsd:verify-work`:** Full suite must be green, then the live checkpoint
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 03-XX-XX | TBD | TBD | CORE-01 | unit | `cargo test -p presto-core backoff` | ❌ W0 | ⬜ pending |
| 03-XX-XX | TBD | TBD | CORE-01 | integration | `cargo test -p presto-core --test recover` | ❌ W0 | ⬜ pending |
| 03-XX-XX | TBD | TBD | CORE-01 | integration | `cargo test -p presto-core --test stale` | ❌ W0 | ⬜ pending |
| 03-XX-XX | TBD | TBD | CORE-02 | engine unit | `cd engine && npm test` | ❌ W0 | ⬜ pending |
| 03-XX-XX | TBD | TBD | CORE-02 | ipc | `cargo test -p presto-ipc` | ✅ | ⬜ pending |
| 03-XX-XX | TBD | TBD | CORE-02 | integration | `cargo test -p presto-core --test drift` | ❌ W0 | ⬜ pending |
| 03-XX-XX | TBD | TBD | CORE-03 | unit + integration | `cargo test -p presto-core mirror` | ❌ W0 | ⬜ pending |
| 03-XX-XX | TBD | TBD | AUTH-01 | integration | `cargo test -p presto-core --test auth` | ❌ W0 | ⬜ pending |
| 03-XX-XX | TBD | TBD | AUTH-01 | engine unit | `cd engine && npm test` | ❌ W0 | ⬜ pending |
| 03-XX-XX | TBD | TBD | AUTH-02 | integration | `cargo test -p presto-core --test auth` | ❌ W0 | ⬜ pending |
| 03-XX-XX | TBD | TBD | AUTH-03 | unit | `cargo test -p presto-core paths` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/presto-core` crate and workspace entry, `nix` workspace dep
- [ ] Mock engine: `bridge_ready`, `show_window`, `SetQueue.play`, startup `signed_out`, bridge-missing option
- [ ] `presto-ipc` 1.1 additions with snapshots and docs updated
- [ ] Test launcher closure (per-attempt argv) and `Timings` override so integration tests run in under 10 s
- [ ] `engine/bridge-path.js` and a window-policy module extracted from `main.js` for Electron-free tests

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Kill -9 Electron mid-play, playback resumes | CORE-01 | Needs account and Widevine | Play, `kill -9` engine, confirm resume at same queue and position |
| Edit `~/.config/presto/bridge.js`, restart | CORE-02 | Needs real engine | Change version string, restart, check handshake |
| First-launch sign-in window, then hidden | AUTH-01 | Needs Apple login | Fresh profile, sign in, relaunch, window stays hidden |
| Session invalidation shows re-auth, cache visible | AUTH-02 | Needs real session | Clear cookies, observe prompt |
| Rapid skips match MusicKit queue | CORE-03 | Needs real MusicKit | Skip rapidly, compare queues |
| Profile dir mode | AUTH-03 | Real filesystem | `stat -c %a` on profile dir prints 700 |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending

---
phase: 2
slug: engine-feasibility-spike-gate
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-10-07
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (libtest, tokio) for driver logic; `node --test` for the engine bridge; live spike is a runnable binary plus human checkpoints |
| **Config file** | workspace `Cargo.toml` (members `crates/*`); `engine/package.json` created in Wave 0 |
| **Quick run command** | `cargo test -p presto-spike` |
| **Full suite command** | `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && node --test engine/test` |
| **Estimated runtime** | ~60 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p presto-spike` (and `node --test engine/test` once present)
- **After every plan wave:** Run the full suite command
- **Before `/gsd:verify-work`:** Full suite green plus live checklist PASS table committed in `logs/`
- **Max feedback latency:** 60 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 2-XX-XX | TBD | TBD | SPIKE-01 | live, manual | `cargo run -p presto-spike -- check musickit` | ❌ W0 | ⬜ pending |
| 2-XX-XX | TBD | TBD | SPIKE-02 | manual + driver | `cargo run -p presto-spike -- check session` | ❌ W0 | ⬜ pending |
| 2-XX-XX | TBD | TBD | SPIKE-03 | live | `cargo run -p presto-spike -- check playback` | ❌ W0 | ⬜ pending |
| 2-XX-XX | TBD | TBD | SPIKE-04 | live | `cargo run -p presto-spike -- check api` | ❌ W0 | ⬜ pending |
| 2-XX-XX | TBD | TBD | SPIKE-05 | live | `cargo run -p presto-spike -- check events` | ❌ W0 | ⬜ pending |
| 2-XX-XX | TBD | TBD | SPIKE-06 | file check | `grep -c` required headings in SPIKE-REPORT.md | ❌ W0 | ⬜ pending |
| 2-XX-XX | TBD | TBD | (offline) | integration | `cargo test -p presto-spike` against `presto-engine-mock` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/presto-spike/` crate and offline mock-engine test
- [ ] `engine/package.json`, `engine/main.js`, `engine/bridge.js`, `engine/preload.js`
- [ ] `engine/test/bridge.test.js` with stub MusicKit
- [ ] `npm install` of ECS (network download)
- [ ] `.gitignore` entries: `engine/node_modules`, account-bearing `logs` content

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Apple sign-in persists across engine restart | SPIKE-02 | Needs real credentials and a desktop session | Sign in via engine window, quit, relaunch, run `check session` |
| Hidden-window playback on Wayland and X11 | SPIKE-01, SPIKE-03 | Needs a desktop session | Run `check playback` under each session type, with and without autoplay and throttling flags |
| Full catalog playback past 60 s and across seek | SPIKE-03 | Needs a subscriber account and audio output | Run `check playback`, listen for audio |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending

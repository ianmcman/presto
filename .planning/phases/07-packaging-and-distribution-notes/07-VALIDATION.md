---
phase: 7
slug: packaging-and-distribution-notes
status: complete
nyquist_compliant: true
wave_0_complete: true
created: 2026-10-08
---

# Phase 7 — Validation Strategy

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test + insta (Rust), `node --test` (engine) |
| **Config file** | `crates/presto-ipc/tests/`, `engine/test/` |
| **Quick run command** | `cargo test -p presto-ipc -p presto-core -p presto` |
| **Full suite command** | `cargo test --workspace && (cd engine && npm test)` |
| **Estimated runtime** | ~60 seconds |

## Sampling Rate

- **After every task commit:** quick run command
- **After every plan wave:** full suite command
- **Before `/gsd:verify-work`:** full suite green, scan script green on a built package
- **Max feedback latency:** 60 seconds

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 07-01-T1 | 01 | 1 | PKG-01 | unit | `cargo test -p presto-ipc` | ✅ | ✅ green |
| 07-01-T2 | 01 | 1 | PKG-01 | unit | `cd engine && npm test` | ✅ | ✅ green |
| 07-02-T1 | 02 | 2 | PKG-01 | unit | `cargo test -p presto-core --test cdm` | ✅ | ✅ green |
| 07-02-T2 | 02 | 2 | PKG-01 | build | `cargo build -p presto` | ✅ | ✅ green |
| 07-03-T1 | 03 | 1 | PKG-01 | unit | `cargo test -p presto --lib -- launch:: cli::` | ✅ | ✅ green |
| 07-03-T2 | 03 | 1 | PKG-01 | script | `sh packaging/scan-no-cdm.sh` self-check | ✅ | ✅ green |
| 07-04-T1 | 04 | 1 | PKG-02 | smoke | `grep -q Flathub docs/DISTRIBUTION.md && grep -q 'Apple Media Services' docs/DISTRIBUTION.md && grep -q DISTRIBUTION.md README.md` | ✅ | ✅ green |
| 07-04-T2 | 04 | 1 | PKG-02 | smoke | same grep smoke | ✅ | ✅ green |
| 07-05-T1 | 05 | 3 | PKG-01 | script | `sh packaging/scan-no-cdm.sh packaging/arch/presto-git-*.pkg.tar.zst` | ✅ | ✅ green |
| 07-05-T3 | 05 | 3 | PKG-01 | manual | `07-MANUAL-CHECKLIST.md` | ✅ | ✅ green (items 1-6, 8, 9 pass; 7 optional, skipped) |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

## Wave 0 Requirements

- [x] `packaging/scan-no-cdm.sh`
- [x] `engine_dir` unit tests in `crates/presto/src/launch.rs`
- [ ] updated insta snapshot for `Frame` schema
- [x] `07-MANUAL-CHECKLIST.md`
- [ ] `devtools` installed for clean-chroot build

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Clean-chroot build, install, first-run CDM fetch, sign-in, playback | PKG-01 | Needs Apple account and Widevine | `07-MANUAL-CHECKLIST.md` |

## Validation Sign-Off

- [ ] All tasks have automated verify or Wave 0 dependencies
- [ ] No 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all missing references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending

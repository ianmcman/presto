---
phase: 7
slug: packaging-and-distribution-notes
status: draft
nyquist_compliant: false
wave_0_complete: false
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
| 7-xx-xx | TBD | TBD | PKG-01 | unit | `cargo test -p presto launch::tests::engine_dir` | ❌ W0 | ⬜ pending |
| 7-xx-xx | TBD | TBD | PKG-01 | unit | `cargo test -p presto-ipc` (cdm snapshot, token guard, doc coverage) | ✅ update | ⬜ pending |
| 7-xx-xx | TBD | TBD | PKG-01 | script | `sh packaging/scan-no-cdm.sh presto-git-*.pkg.tar.zst` | ❌ W0 | ⬜ pending |
| 7-xx-xx | TBD | TBD | PKG-02 | smoke | `grep -q Flathub docs/DISTRIBUTION.md && grep -q 'Apple Media Services' docs/DISTRIBUTION.md && grep -q DISTRIBUTION.md README.md` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

## Wave 0 Requirements

- [ ] `packaging/scan-no-cdm.sh`
- [ ] `engine_dir` unit tests in `crates/presto/src/launch.rs`
- [ ] updated insta snapshot for `Frame` schema
- [ ] `07-MANUAL-CHECKLIST.md`
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
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending

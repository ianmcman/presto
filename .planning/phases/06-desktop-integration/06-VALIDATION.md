---
phase: 06
slug: desktop-integration
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-10-08
---

# Phase 06 — Validation Strategy

> Per-phase validation contract. Full test map is in 06-RESEARCH.md "Validation Architecture".

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test, insta snapshots, tempfile |
| **Config file** | workspace `Cargo.toml` |
| **Quick run command** | `cargo test -p presto --lib` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~60 seconds |

## Sampling Rate

- **After every task commit:** `cargo test -p presto --lib`
- **After every plan wave:** `cargo test --workspace`, then `dbus-run-session -- cargo test -p presto --test desktop_mpris`
- **Before `/gsd:verify-work`:** Full suite green
- **Max feedback latency:** 90 seconds

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 06-01-T1 | 01 | 1 | DESK-02 | schema/unit | `cargo test -p presto-ipc` | W0 creates | ⬜ pending |
| 06-01-T2 | 01 | 1 | DESK-02 | unit | `cargo test -p presto --lib cli:: ctl::` | W0 creates | ⬜ pending |
| 06-02-T1 | 02 | 2 | DESK-01, DESK-02 | unit | `cargo test -p presto --lib control::` | W0 creates | ⬜ pending |
| 06-02-T2 | 02 | 2 | DESK-01, DESK-02 | unit | `cargo test -p presto --lib status::` | W0 creates | ⬜ pending |
| 06-03-T1 | 03 | 3 | DESK-02 | integration | `cargo test -p presto --lib ctl:: && cargo test -p presto --test ctl_server` | W0 creates | ⬜ pending |
| 06-03-T2 | 03 | 3 | DESK-02 | integration | `cargo test -p presto --test cli_e2e` | W0 creates | ⬜ pending |
| 06-04-T1 | 04 | 4 | DESK-01 | unit | `cargo test -p presto --lib desktop::` | W0 creates | ⬜ pending |
| 06-04-T2 | 04 | 4 | DESK-01 | integration (private bus) | `dbus-run-session -- cargo test -p presto --test desktop_mpris && cargo test -p presto-core --test engine_switches` | W0 creates | ⬜ pending |
| 06-05-T1 | 05 | 5 | DESK-01, DESK-02 | suite | `cargo test --workspace && dbus-run-session -- cargo test -p presto --test desktop_mpris` | yes | ⬜ pending |
| 06-05-T2 | 05 | 5 | DESK-01, DESK-02 | manual | 06-MANUAL-CHECKLIST.md | n/a | ⬜ pending |

## Wave 0 Requirements

- [ ] `crates/presto/src/status.rs` table tests
- [ ] `crates/presto/tests/ctl_server.rs`
- [ ] `crates/presto/tests/desktop_mpris.rs`
- [ ] insta snapshot for status JSON in presto-ipc

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Media keys on KDE/GNOME/waybar; Raise/Close from minimized window | DESK-01, DESK-02 | needs a real Wayland desktop | checklist in phase VERIFICATION |

## Validation Sign-Off

- [ ] All tasks have automated verify or Wave 0 dependencies
- [ ] No 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending

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
| TBD (filled by planner) | | | DESK-01, DESK-02 | unit/integration | see RESEARCH | ❌ W0 | ⬜ pending |

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

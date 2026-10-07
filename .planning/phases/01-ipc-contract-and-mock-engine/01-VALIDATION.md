---
phase: 01
slug: ipc-contract-and-mock-engine
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-10-07
---

# Phase 01 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (libtest), insta 1.49 snapshots, tokio `#[tokio::test]` |
| **Config file** | none, Wave 0 creates workspace `Cargo.toml` and `rust-toolchain.toml` |
| **Quick run command** | `cargo test -p presto-ipc` |
| **Full suite command** | `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p presto-ipc` (or the touched crate)
- **After every plan wave:** Run `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 60 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 01-01-T2 | 01-01 | 1 | IPC-01 | unit | `cargo test -p presto-ipc --test roundtrip` | ❌ W0 | ⬜ pending |
| 01-01-T3 | 01-01 | 1 | IPC-01 | integration | `cargo test -p presto-ipc --test transport` | ❌ W0 | ⬜ pending |
| 01-02-T1 | 01-02 | 2 | IPC-01 | snapshot | `cargo test -p presto-ipc --test schema snapshot` | ❌ W0 | ⬜ pending |
| 01-02-T1 | 01-02 | 2 | IPC-02 | unit | `cargo test -p presto-ipc --test schema` (no_token, canary, lowercase) | ❌ W0 | ⬜ pending |
| 01-02-T2 | 01-02 | 2 | IPC-01 | unit | `cargo test -p presto-ipc --test doc_covers_variants` | ❌ W0 | ⬜ pending |
| 01-03-T1 | 01-03 | 2 | IPC-03 | unit | `cargo test -p presto-engine-mock --bins` | ❌ W0 | ⬜ pending |
| 01-03-T2 | 01-03 | 2 | IPC-03 | integration | `cargo test -p presto-engine-mock --test protocol` | ❌ W0 | ⬜ pending |
| 01-04-T2 | 01-04 | 3 | IPC-03 | integration | `cargo test -p presto-engine-mock --test faults` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Install Rust toolchain via rustup, pinned 1.98.0 (none on this machine)
- [ ] Workspace `Cargo.toml`, `rust-toolchain.toml`, `.gitignore` (`target/`)
- [ ] `crates/presto-ipc/tests/{roundtrip,schema,doc_covers_variants}.rs`
- [ ] `crates/presto-engine-mock/tests/{protocol,faults}.rs`

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Seam doc exists; egui fork rev `ba6790fe` and fastframe `v0.4.1` resolve | D-17 | One-time, needs network | `cargo fetch` in a throwaway crate outside the workspace, record result in the seam doc |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending

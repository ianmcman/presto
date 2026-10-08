---
phase: 4
slug: data-layer-and-cache
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-10-08
---

# Phase 4 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test, insta (snapshots), tempfile |
| **Config file** | workspace `Cargo.toml` |
| **Quick run command** | `cargo test -p presto-core data::` |
| **Full suite command** | `cargo build -p presto-engine-mock && cargo test --workspace` |
| **Estimated runtime** | ~60 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p presto-core data::`
- **After every plan wave:** Run the full suite command
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 60 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 4-W0 | TBD | 0 | DATA-01 | integration (mock) | `cargo test -p presto-core --test data_library` | ❌ W0 | ⬜ pending |
| 4-W0 | TBD | 0 | DATA-01 | unit (store) | `cargo test -p presto-core data::store` | ❌ W0 | ⬜ pending |
| 4-W0 | TBD | 0 | DATA-02 | unit (paused time) | `cargo test -p presto-core data::search` | ❌ W0 | ⬜ pending |
| 4-W0 | TBD | 0 | DATA-02 | integration (mock) | `cargo test -p presto-core --test data_search` | ❌ W0 | ⬜ pending |
| 4-W0 | TBD | 0 | DATA-03 | unit (fake clock) | `cargo test -p presto-core data::view` | ❌ W0 | ⬜ pending |
| 4-W0 | TBD | 0 | DATA-04 | unit | `cargo test -p presto-core data::client` | ❌ W0 | ⬜ pending |
| 4-W0 | TBD | 0 | DATA-05 | unit + integration | `cargo test -p presto-core --test data_errors` | ❌ W0 | ⬜ pending |
| 4-W0 | TBD | 0 | DATA-06 | integration (mock) | `cargo test -p presto-core --test data_offline` | ❌ W0 | ⬜ pending |
| 4-W0 | TBD | 0 | DATA-06 | unit (tempdir) | `cargo test -p presto-core data::artwork` | ❌ W0 | ⬜ pending |
| 4-W0 | TBD | 0 | D-17 | integration | `cargo test -p presto-core --test data_wipe` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/presto-core/tests/data_*.rs` (library, search, errors, offline, wipe)
- [ ] Mock catalog additions and `rate_limited` fault, with schema snapshot and PROTOCOL.md updates
- [ ] 10k-song mock library flag
- [ ] Live probe example for sort, shapes, limits (extend `crates/presto-core/examples/live.rs`)
- [ ] Add rusqlite, reqwest, sha2 to workspace deps

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Live Apple sort support, response shapes, page limits | DATA-01, DATA-03 | Needs a signed-in Apple account | Run the extended `live` example, record output in the plan SUMMARY |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending

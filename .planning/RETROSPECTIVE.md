# Retrospective

## Milestone: v1.0 Linux Apple Music client

**Shipped:** 2026-10-09
**Phases:** 7 | **Plans:** 56

### What Was Built
A native egui Apple Music client for Linux backed by a hidden castlabs Electron engine, with a mock engine for demo mode, MPRIS and CLI control, and Arch packages.

### What Worked
- The feasibility gate in phase 2 settled the engine choice with measured evidence before the large phases.
- A mock engine with fault injection let phases 3 to 6 be built and tested without an Apple account or Widevine.
- Manual checklists on a real machine found problems that automated tests missed (radio stations, build path leaks, missing license file).

### What Was Inefficient
- STATE.md and the ROADMAP checkboxes drifted behind the real state. Phase 7 was marked complete before verification ran.
- Auto-extracted accomplishments in MILESTONES.md came out as deviation headings and had to be rewritten.
- The agent sandbox cannot start Electron, so every playback check waited for a human.
- Radio stations were never exercised until the first install on a second machine.

### Patterns Established
- Packaging files live under `packaging/`, with a source build (`presto-git`) and a prebuilt one (`presto-bin`).
- The CDM is fetched at runtime and a scan script guards every package against shipping it.

### Key Lessons
- Test every playable item type (songs, playlists, albums, stations) against the real engine early.
- Run phase verification before marking a phase complete.
- Pin third-party CI actions or avoid them.

### Cost Observations
- Model profile: budget (sonnet executors and planners, haiku verifiers).

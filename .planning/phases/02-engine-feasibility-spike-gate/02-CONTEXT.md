# Phase 2: Engine Feasibility Spike (GATE) - Context

**Gathered:** 2026-10-07
**Status:** Ready for planning

<domain>
## Phase Boundary

Decide go/no-go on the hidden-Chromium approach and pick the engine, with measured evidence. Requirements: SPIKE-01 to SPIKE-06. Work stops after this phase for user approval before Phases 3 to 7 are planned in detail. Supervisor recovery, UI, caching and packaging are other phases.

</domain>

<decisions>
## Implementation Decisions

### Candidates and scope
- **D-01:** Spike castlabs ECS first. Spike system Chrome via CDP only if ECS fails a criterion. CEF and WebKitGTK get a desk comparison in the report.
- **D-02:** ECS version: look up the newest supported tag at spike time (v44.1.0 is unconfirmed) and use it. Record the exact tag.
- **D-03:** Test hidden-window behavior (`show:false`) on both Wayland and X11. Playback must continue while hidden.
- **D-04:** Record the date and observed web player build in the report. No resilience work in the spike; that belongs to the Phase 3 bridge design.

### Spike code and protocol
- **D-05:** The spike engine speaks the real Phase 1 `presto-ipc` protocol over the Unix socket (hello, commands, requests, events). No ad-hoc messages.
- **D-06:** A `presto-spike` crate in the workspace is the Rust driver. It listens on the socket, spawns the engine, and runs the checklist (play, seek past 60 s, proxy call, event stream). A REPL mode is not required.
- **D-07:** Spike code is a seed for Phase 3. Keep `engine/` and the spike crate tidy enough to harden rather than rewrite.
- **D-08:** The bridge script is a standalone file (`engine/bridge.js`) read from disk and injected into music.apple.com at runtime, matching CORE-02 from the start.

### Account, terms and setup
- **D-09:** The spike runs on the user's own subscriber account.
- **D-10:** The plan starts with a gate task: the user reads the current Apple Media Services terms and says proceed. No sign-in happens before it.
- **D-11:** The user signs in by hand in the visible engine window, then the window hides. Credentials never touch a script, log or the repo.
- **D-12:** The engine profile lives in a dedicated directory under XDG state (for example `~/.local/state/presto/engine-profile`), mode 0700, never shared with another browser. Phase 3 AUTH-03 keeps this location.

### Go/no-go and report
- **D-13:** GO requires all five roadmap success criteria met. Any miss means NO-GO or a documented alternative.
- **D-14:** RSS (idle and playing) is measured and reported with no pass/fail threshold. The user judges.
- **D-15:** If ECS fails, try Chrome via CDP, then report. If both fail, the report states NO-GO with evidence and options.
- **D-16:** The report is `SPIKE-REPORT.md` under the phase directory, with raw measurement logs saved beside it.

### Claude's Discretion
- Spike crate layout and how the checklist is scripted.
- How measurements are taken (RSS sampling method, duration of playback runs).
- Which test tracks and library endpoint variants to use beyond `/v1/me/library/playlists`.
- Report structure beyond the required items in SPIKE-06.

</decisions>

<specifics>
## Specific Ideas

No specific requirements beyond the roadmap success criteria and the Cider and Sidra precedents already noted in CLAUDE.md.

</specifics>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project
- `.planning/PROJECT.md` : constraints (no Apple tokens in Rust, two-process architecture, engine replaceable, no developer account)
- `.planning/REQUIREMENTS.md` : SPIKE-01 to SPIKE-06; CORE-02 and AUTH-03 for what the spike seeds
- `.planning/ROADMAP.md` : Phase 2 goal and five success criteria
- `.planning/STATE.md` : blockers (read Apple Media Services terms, confirm ECS tag)
- `CLAUDE.md` : engine candidate ranking, Widevine sourcing notes, IPC options, open items for the spike

### Phase 1 contract
- `.planning/phases/01-ipc-contract-and-mock-engine/01-CONTEXT.md` : decisions D-01 to D-17 on transport, handshake, vocabulary, errors
- `docs/PROTOCOL.md` : hand-written protocol the spike engine must speak
- `crates/presto-ipc/` : wire types, version check, timeouts, socket transport
- `crates/presto-engine-mock/` : reference implementation of the protocol and test harness
- `docs/SPOTIFAST-SEAMS.md` : playback and API-client seams the engine feeds

### External
- https://github.com/castlabs/electron-releases and its wiki : ECS tags, supported series, `components` API for CDM download
- https://github.com/wimpysworld/sidra : precedent for ECS plus Apple Music
- https://docs.rs/crate/cef/latest and https://github.com/tauri-apps/cef-rs : desk comparison only

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `presto-ipc` transport and frame types: the spike driver and the Node side both speak these.
- `presto-engine-mock` test harness (`tests/common/mod.rs`): pattern for spawning an engine process and exchanging frames.

### Established Patterns
- Engine connects to a socket that presto listens on; presto spawns the engine as a child and passes socket path and profile dir by argv or env (Phase 1 D-01, D-02).
- Engine output goes to a log file under the state dir (Phase 1 D-03).

### Integration Points
- New `crates/presto-spike` crate joins the workspace.
- New top-level `engine/` directory holds the Electron main process and `bridge.js`.

</code_context>

<deferred>
## Deferred Ideas

- Smoke script to re-detect web player breakage: not in the spike; revisit in Phase 3 bridge design.
- Interactive REPL driver: not needed for the gate.

</deferred>

---

*Phase: 02-engine-feasibility-spike-gate*
*Context gathered: 2026-10-07*

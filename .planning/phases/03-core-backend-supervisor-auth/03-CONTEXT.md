# Phase 3: Core Backend, Supervisor, Auth - Context

**Gathered:** 2026-10-08
**Status:** Ready for planning

<domain>
## Phase Boundary

Presto runs a supervised engine reliably and the user signs in once and stays signed in. Covers engine supervision and recovery, runtime bridge handshake, the Rust queue mirror, sign-in and re-auth. Requirements: CORE-01, CORE-02, CORE-03, AUTH-01, AUTH-02, AUTH-03. Data layer, caches, ported UI views and MPRIS are other phases.

</domain>

<decisions>
## Implementation Decisions

### Recovery
- **D-01:** After a crash or hang restart, restore queue and position. Resume playing only if state was playing at failure; otherwise stay paused at position.
- **D-02:** Backoff 1s, 2s, 4s, doubling, capped at 30s. A failure within 60s of start is a fast failure; after 5 fast failures give up, show "engine failed" with a manual Restart. The counter resets after 2 minutes of stability.
- **D-03:** Stale engine handling: write a pidfile in the state dir. At startup, if the recorded pid is alive and is the engine binary, SIGTERM then SIGKILL it silently and log it. Reap the engine's process group when the main process dies.
- **D-04:** After the 2nd consecutive crash on the same queue, restore the queue paused rather than resuming.
- **D-05:** Hang detection uses the Phase 1 heartbeat (2s x3).

### Sign-in window
- **D-06:** On `signed_out` the engine window shows immediately, and Presto shows a prompt ("Sign in to Apple Music in the window") with a Bring-to-front button.
- **D-07:** Closing the sign-in window hides it (engine keeps running). Presto shows a Sign in button to reopen it.
- **D-08:** After sign-in the window stays hidden on later launches. It shows only for `signed_out` or re-auth. A debug flag may force-show it.
- **D-09:** While signed out, a sign-in panel blocks the full UI. Cached data shows if present.

### Re-auth
- **D-10:** On AuthExpired show a banner with a Re-authenticate button. The window opens on click, never automatically.
- **D-11:** After successful re-auth keep queue and position and resume per the D-01 rule.
- **D-12:** While signed out or expired Rust sends no API requests and serves cached data only. Pending requests fail fast with the signed-out state.
- **D-13:** Session state comes from engine auth events and AuthExpired response errors only. No periodic probe.

### Bridge loading and drift
- **D-14:** bridge.js loads from `$XDG_CONFIG_HOME/presto/bridge.js` if present, else from the install dir copy. Editing and restarting the engine changes behavior with no rebuild.
- **D-15:** The handshake carries bridge version string, capability list and MusicKit build.
- **D-16:** If MusicKit is missing or a required capability is absent, show a blocking error panel ("Apple's web player changed; update bridge.js") with details and log path. This is not counted as a crash and does not enter the restart loop.
- **D-17:** Rust queues commands until `bridge_ready`. After 15s without it, raise the drift error (D-16).

### Claude's Discretion
- Crate layout for the supervisor and queue mirror.
- Queue revision reconciliation details (Phase 1 D-07 snapshots with revision are the base).
- Profile dir creation and 0700 enforcement mechanics (location fixed by Phase 2 D-12).
- Pidfile location and format, log tailing format.

</decisions>

<specifics>
## Specific Ideas

No specific requirements beyond the spike's observed behavior.

</specifics>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project
- `.planning/PROJECT.md` — no Apple tokens in Rust, two-process architecture
- `.planning/REQUIREMENTS.md` — CORE-01..03, AUTH-01..03
- `.planning/ROADMAP.md` — Phase 3 goal and success criteria
- `.planning/STATE.md` — spike gate decision; orphan/stale-lock note
- `CLAUDE.md` — stack and IPC recommendations

### Prior phases
- `.planning/phases/01-ipc-contract-and-mock-engine/01-CONTEXT.md` — transport, handshake, timeouts, error enum, queue snapshots with revision
- `.planning/phases/02-engine-feasibility-spike-gate/02-CONTEXT.md` — profile location, engine spawn pattern
- `.planning/phases/02-engine-feasibility-spike-gate/SPIKE-REPORT.md` — Risks and Phase 3 Inputs (orphan after SIGKILL, bridge-ready gate, origin guard, EPIPE hardening, `--diag` SIGTRAP)
- `docs/PROTOCOL.md` — wire protocol
- `docs/SPOTIFAST-SEAMS.md` — Backend/Command/Event seams

### Code
- `crates/presto-ipc/` — frame, command, event, timeout types
- `crates/presto-engine-mock/` — mock faults used to test recovery
- `crates/presto-spike/` — driver seed for the supervisor
- `engine/main.js`, `engine/bridge.js`, `engine/guard.js`, `engine/preload.js` — engine seed

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `presto-ipc`: transport, Hello with capabilities, Kind timeouts, heartbeat constants.
- `presto-engine-mock`: hang/crash/auth_expired/slow faults to test supervisor and re-auth without Widevine.
- `presto-spike`: socket listen, engine spawn and checklist code to harden into the supervisor.
- `engine/`: Electron main with origin guard, EPIPE hardening, window show on signed_out/hide on signed_in.

### Established Patterns
- presto listens on the socket, spawns the engine as a child, passes socket path and profile dir by argv/env.
- Engine output goes to a log file under the state dir.

### Integration Points
- New native crate (`crates/presto` or a core crate) hosts supervisor, queue mirror and auth state.
- `engine/main.js` gains bridge path resolution and bridge-ready in the handshake.

</code_context>

<deferred>
## Deferred Ideas

None. Discussion stayed within phase scope.

</deferred>

---

*Phase: 03-core-backend-supervisor-auth*
*Context gathered: 2026-10-08*

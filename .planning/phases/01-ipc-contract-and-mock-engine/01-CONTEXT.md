# Phase 1: IPC Contract and Mock Engine - Context

**Gathered:** 2026-10-07
**Status:** Ready for planning

<domain>
## Phase Boundary

A stable, engine-neutral JSON protocol (`presto-ipc` crate plus a protocol doc) and a mock engine that speaks it, so downstream phases can be built and tested without Widevine. Also a short document of the spotifast seams the UI depends on. Requirements: IPC-01, IPC-02, IPC-03. The real engine, UI, supervisor recovery and `--demo` UI are other phases.

</domain>

<decisions>
## Implementation Decisions

### Transport and lifecycle
- **D-01:** Unix domain socket, NDJSON framing. presto listens at `$XDG_RUNTIME_DIR/presto/engine.sock`; the engine connects. Resolves the pending STATE.md todo (stdio is not used).
- **D-02:** presto spawns the engine as a child, passing socket path and profile dir via argv/env, and terminates it on exit. No independent engine daemon.
- **D-03:** Engine stdout/stderr is captured to a log file under the state dir and tailed on failure.
- **D-04:** Hello handshake carries `proto` as major.minor plus a capability list. Minor mismatches are tolerated; features are gated by capabilities; major mismatch is a hard error with a clear message.
- **D-05:** Request IDs, per-kind timeouts and a heartbeat for hang detection are part of the protocol (from research).

### Message vocabulary
- **D-06:** Typed per-action commands: Play, Pause, Seek{ms}, Next, Prev, SetVolume, SetShuffle, SetRepeat, SetQueue{ids,start}, mirroring spotifast's Command enum. No generic MusicKit Invoke passthrough.
- **D-07:** Queue events are full snapshots with a revision: `QueueChanged{rev, items, index}`. Rust discards stale revisions. No deltas.
- **D-08:** Proxied requests take any Apple Music API path: `Request{method, path, query, body}`. The engine adds auth inside the page; Rust never sees a token.
- **D-09:** Closed error enum on responses: Timeout, AuthExpired, RateLimited{retry_after}, NotFound, Unavailable, Upstream{status}, Internal, each with a message. The UI maps kinds to states.

### Mock engine and fault injection
- **D-10:** Mock is a separate `presto-engine-mock` binary, spawned and connected exactly like the real engine.
- **D-11:** Faults are triggered by `--fault` CLI flags at startup and by a `MockControl{fault}` message over the socket for live toggling.
- **D-12:** Fault shapes: hang stops replies and heartbeat but keeps the socket open; crash exits non-zero; auth_expired emits an AuthExpired event and fails requests; slow adds a configurable delay.
- **D-13:** Mock has a small canned catalog (library, search, playlists, albums, artists) and a real playback clock that ticks, seeks and advances the queue, so Phase 5 UI work is meaningful.

### Docs, tests and layout
- **D-14:** PROTOCOL.md is hand-written. The JSON schema (schemars) is snapshot-tested so doc and types cannot drift silently.
- **D-15:** IPC-02 test generates the schema for all IPC types and fails on any property name matching token/auth/bearer/jwt/secret/cookie patterns, with an explicit allowlist.
- **D-16:** Cargo workspace at repo root: `crates/presto-ipc`, `crates/presto-engine-mock`; `crates/presto` and an `engine/` directory for the JS side come later.
- **D-17:** Seam document lists the spotifast Backend/Command/Event and API-client types the UI uses, with file references, and confirms by running cargo that the crmne/egui fork rev `ba6790fe` and the fastframe crates resolve. No full view-by-view port inventory.

### Claude's Discretion
- Exact heartbeat interval and per-kind timeout values.
- Exact event set beyond play state, progress, queue, auth and error events.
- Fault parameter syntax and the slow delay default.
- Crate dependency choices within the stack in CLAUDE.md.

</decisions>

<specifics>
## Specific Ideas

No specific requirements beyond matching spotifast's Command/Event shape where it fits.

</specifics>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project
- `.planning/PROJECT.md` — constraints: Rust holds no Apple tokens, two-process architecture, engine replaceable
- `.planning/REQUIREMENTS.md` — IPC-01, IPC-02, IPC-03 definitions; CORE-03 queue ownership
- `.planning/ROADMAP.md` — Phase 1 goal and success criteria
- `.planning/STATE.md` — pending transport todo (now resolved by D-01)
- `CLAUDE.md` — stack table, IPC options table and recommendation

### External
- https://github.com/crmne/spotifast — Backend/Command/Event and API-client seams to document (`Cargo.toml` for the egui fork patch)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- None. The repo contains only planning artifacts and CLAUDE.md.

### Established Patterns
- None yet. Follow the stack in CLAUDE.md (tokio, serde, tokio-util LinesCodec).

### Integration Points
- `presto-ipc` is shared by the native app, the mock, and later the real engine adapter.

</code_context>

<deferred>
## Deferred Ideas

None. Discussion stayed within phase scope.

</deferred>

---

*Phase: 01-ipc-contract-and-mock-engine*
*Context gathered: 2026-10-07*

# Phase 1: IPC Contract and Mock Engine - Research

**Researched:** 2026-10-07
**Domain:** Rust workspace, serde/schemars protocol crate, tokio Unix-socket NDJSON server/client, mock process with fault injection
**Confidence:** HIGH for stack and seams (versions checked against crates.io, spotifast source read); MEDIUM for numeric defaults (judgment)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Unix domain socket, NDJSON framing. presto listens at `$XDG_RUNTIME_DIR/presto/engine.sock`; the engine connects. Resolves the pending STATE.md todo (stdio is not used).
- **D-02:** presto spawns the engine as a child, passing socket path and profile dir via argv/env, and terminates it on exit. No independent engine daemon.
- **D-03:** Engine stdout/stderr is captured to a log file under the state dir and tailed on failure.
- **D-04:** Hello handshake carries `proto` as major.minor plus a capability list. Minor mismatches are tolerated; features are gated by capabilities; major mismatch is a hard error with a clear message.
- **D-05:** Request IDs, per-kind timeouts and a heartbeat for hang detection are part of the protocol (from research).
- **D-06:** Typed per-action commands: Play, Pause, Seek{ms}, Next, Prev, SetVolume, SetShuffle, SetRepeat, SetQueue{ids,start}, mirroring spotifast's Command enum. No generic MusicKit Invoke passthrough.
- **D-07:** Queue events are full snapshots with a revision: `QueueChanged{rev, items, index}`. Rust discards stale revisions. No deltas.
- **D-08:** Proxied requests take any Apple Music API path: `Request{method, path, query, body}`. The engine adds auth inside the page; Rust never sees a token.
- **D-09:** Closed error enum on responses: Timeout, AuthExpired, RateLimited{retry_after}, NotFound, Unavailable, Upstream{status}, Internal, each with a message. The UI maps kinds to states.
- **D-10:** Mock is a separate `presto-engine-mock` binary, spawned and connected exactly like the real engine.
- **D-11:** Faults are triggered by `--fault` CLI flags at startup and by a `MockControl{fault}` message over the socket for live toggling.
- **D-12:** Fault shapes: hang stops replies and heartbeat but keeps the socket open; crash exits non-zero; auth_expired emits an AuthExpired event and fails requests; slow adds a configurable delay.
- **D-13:** Mock has a small canned catalog (library, search, playlists, albums, artists) and a real playback clock that ticks, seeks and advances the queue, so Phase 5 UI work is meaningful.
- **D-14:** PROTOCOL.md is hand-written. The JSON schema (schemars) is snapshot-tested so doc and types cannot drift silently.
- **D-15:** IPC-02 test generates the schema for all IPC types and fails on any property name matching token/auth/bearer/jwt/secret/cookie patterns, with an explicit allowlist.
- **D-16:** Cargo workspace at repo root: `crates/presto-ipc`, `crates/presto-engine-mock`; `crates/presto` and an `engine/` directory for the JS side come later.
- **D-17:** Seam document lists the spotifast Backend/Command/Event and API-client types the UI uses, with file references, and confirms by running cargo that the crmne/egui fork rev `ba6790fe` and the fastframe crates resolve. No full view-by-view port inventory.

### Claude's Discretion
- Exact heartbeat interval and per-kind timeout values.
- Exact event set beyond play state, progress, queue, auth and error events.
- Fault parameter syntax and the slow delay default.
- Crate dependency choices within the stack in CLAUDE.md.

### Deferred Ideas (OUT OF SCOPE)
None. Discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| IPC-01 | Versioned command, request/response and event messages as JSON, request IDs, per-kind timeouts | Envelope + vocabulary below; `Kind::timeout()` in presto-ipc; hello `proto` major.minor |
| IPC-02 | No IPC type carries an Apple token field, schema test | schemars 1.x `schema_for!` on root `Frame` type plus recursive property-name walk (code example) |
| IPC-03 | Mock engine implements protocol with faults hang/crash/auth_expired/slow | tokio UnixStream client + `Fault` state machine; `--fault` flag + `MockControl` |
</phase_requirements>

## Summary

Phase 1 is plain Rust: one library crate defining serde types, one binary that connects to a Unix socket and speaks NDJSON. No Widevine, no UI. The only external-facing risk is the seam document (D-17). I checked it: the egui fork rev `ba6790fe7cf46e58e8d27ce1524cbfdee745e938` is the head of branch `apps-0.36` on github.com/crmne/egui (public, fetchable), and `crmne/fastframe` tag `v0.4.1` exists (annotated tag, commit `23d87e04`). Full `cargo` resolution is still unrun because **no Rust toolchain is installed on this machine** (`cargo`, `rustc`, `rustup` all absent). Installing one is a Wave 0 task.

The stack is small: serde, serde_json, schemars 1.x, tokio, tokio-util `LinesCodec`, clap, thiserror, tracing, plus insta for the schema snapshot. Keep `presto-ipc` free of tokio so the future core and the mock both consume it; put the framing/transport helper in `presto-ipc` behind a `transport` feature only if both sides need it (they will: presto listens, mock connects, both frame lines). Recommended: put a thin `transport` module in `presto-ipc` (tokio-util codec wrapper, `bind`/`connect`, socket path resolver) so Phase 3 does not rewrite it.

**Primary recommendation:** Define one `Frame` enum (`hello`, `cmd`, `req`, `res`, `evt`, `ping`/`pong`, `mock`) tagged with serde `tag = "t"`, derive `JsonSchema` on it, snapshot that schema with insta, and run the token-name walk on the same schema.

## Standard Stack

### Core (versions verified on crates.io 2026-10-07)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| serde / serde_json | 1.0.229 / 1.0.151 | wire types | CLAUDE.md |
| schemars | 1.2.2 | JSON schema for snapshot + token test | derive `JsonSchema`; 1.x API is `schemars::schema_for!` returning `Schema` (wraps `serde_json::Value`), so the walk is plain `Value` traversal |
| tokio | 1.53.2 (`rt-multi-thread, net, io-util, time, process, macros, sync, signal`) | async runtime | CLAUDE.md |
| tokio-util | 0.7.19 (`codec`) | `Framed` + `LinesCodec::new_with_max_length` | CLAUDE.md |
| futures-util | 0.3 | `SinkExt`/`StreamExt` on `Framed` | needed with tokio-util codec |
| clap | 4.6.7 (`derive`) | mock CLI `--fault` | spotifast uses clap |
| thiserror | 2.0.21 | error types | |
| tracing + tracing-subscriber | 0.1.44 | engine logs to stderr | D-03 captures stderr |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| insta | 1.49.0 (`json` feature) | schema snapshot (D-14) | one `assert_json_snapshot!` on the schema; `cargo insta review` to accept |
| tempfile | 3 | per-test socket dir | integration tests; `XDG_RUNTIME_DIR` may be unset in CI |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| insta | `include_str!` of a checked-in `schema.json` plus string compare | zero deps, but you hand-regenerate. Acceptable if the planner wants fewer deps; insta is the lower-effort path |
| `LinesCodec` | `LengthDelimitedCodec` | CLAUDE.md picks NDJSON; the JS engine side reads lines trivially |

**Installation:**
```bash
# Wave 0: toolchain (not installed here). Match spotifast's pin policy.
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup toolchain install 1.98.0   # spotifast rust-toolchain.toml channel; stable is now 1.99.0
cargo add -p presto-ipc serde --features derive   # etc.; or write Cargo.toml directly
cargo install cargo-insta         # optional, for snapshot review
```
Use `rust-toolchain.toml` with `channel = "1.98.0"`, components rustfmt+clippy, edition 2024, so the later egui phase compiles with the same compiler as spotifast (and the fork). Stable 1.99.0 is fine if 1.98 is unavailable; spotifast pins deliberately.

## Architecture Patterns

### Recommended Project Structure
```
Cargo.toml                       # [workspace] members = ["crates/*"], resolver = "3", shared [workspace.dependencies]
rust-toolchain.toml
docs/PROTOCOL.md                 # hand-written (D-14)
docs/SPOTIFAST-SEAMS.md          # D-17
crates/presto-ipc/
  src/lib.rs                     # re-exports
  src/frame.rs                   # Frame envelope, Hello, ProtoVersion
  src/command.rs                 # Command (D-06)
  src/request.rs                 # ApiRequest{method,path,query,body}, ApiResponse, IpcError (D-08, D-09)
  src/event.rs                   # Event (state, progress, queue, auth, error)
  src/kind.rs                    # Kind enum + timeout()/heartbeat consts
  src/transport.rs               # socket_path(), bind/connect, Framed<UnixStream, LinesCodec>
  tests/schema.rs                # snapshot + token walk
  tests/snapshots/
crates/presto-engine-mock/
  src/main.rs                    # clap, connect, run loop
  src/catalog.rs                 # canned data
  src/player.rs                  # playback clock + queue
  src/fault.rs                   # Fault state
  tests/protocol.rs              # spawn mock, drive over socket
```

### Pattern 1: One envelope enum, internally tagged
Single root type means one schema and one place for the token test. Responses correlate by `id`; commands are fire-and-forget (no id) in the spotifast style, but the wire needs failure feedback, so give commands an `id` and a `res{ok|err}` ack. Recommended:

```rust
// presto-ipc/src/frame.rs
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum Frame {
    Hello(Hello),                       // both directions; engine sends first after connect, presto replies with its own Hello
    Cmd  { id: u64, #[serde(flatten)] cmd: Command },           // presto -> engine
    Req  { id: u64, #[serde(flatten)] req: ApiRequest },        // presto -> engine
    Res  { id: u64, #[serde(flatten)] res: Outcome },           // engine -> presto, Outcome = Ok{data} | Err{error: IpcError}
    Evt  { #[serde(flatten)] evt: Event },                      // engine -> presto
    Ping { seq: u64 }, Pong { seq: u64 },                       // heartbeat (D-05)
    Mock { id: u64, fault: FaultSpec },                         // MockControl (D-11); lives in presto-ipc so one schema covers it
}
```
Caveat: `#[serde(flatten)]` into an internally tagged enum with its own `tag` collides (two `t`). Give the inner enums a different tag name (`"c"` for Command, `"e"` for Event) or nest them as fields (`"cmd": {...}`). Nesting is simpler and schemars-safe; prefer nesting and skip `flatten` entirely. Verify with a roundtrip test.

`Hello { proto: ProtoVersion{major: u16, minor: u16}, role: Presto|Engine, capabilities: Vec<String>, engine: Option<String> /* name+version */ }`. `proto` const lives in presto-ipc (`PROTO: ProtoVersion = {1, 0}`); `Hello::check(&self, ours) -> Result<(), ProtoError>` errors on major mismatch with a message naming both versions.

### Pattern 2: Kind and timeout table in the crate (IPC-01 "per-kind timeout")
Success criterion 1 wants the timeout visible in the crate. One `Kind` enum and a `const fn timeout(kind) -> Duration`, documented in PROTOCOL.md with the same numbers. Recommended defaults (Claude's discretion, tune in the spike):

| Kind | Timeout |
|------|---------|
| Command (play/pause/seek/volume/shuffle/repeat/next/prev) | 5 s |
| SetQueue | 15 s (loads media) |
| Api Request (GET) | 20 s |
| Api Request (writes) | 30 s |
| Hello | 5 s |
| Heartbeat | ping every 2 s, hang after 3 missed (6 s) |

Timeout is enforced by the requester (presto), not sent on the wire; the engine need not know. Expose `Frame::kind()` (or `Cmd::kind()`) so the core looks up its timeout, and `IpcError::Timeout` is synthesized locally by the requester.

### Pattern 3: Event set
Minimum: `PlaybackState{state: Playing|Paused|Stopped|Loading|Ended, seq}`, `Progress{position_ms, duration_ms, at_ms /*engine monotonic*/}`, `TrackChanged{item: QueueItem}`, `QueueChanged{rev, items, index}`, `Volume/Shuffle/Repeat` changes, `Auth{state: SignedIn|SignedOut|SigningIn|Expired}` (this is the AuthExpired event of D-12), `Error{error: IpcError}`, `Capabilities` is in hello. Progress at ~1 Hz plus on seek/state change; the UI interpolates (spotifast's `position_ms + position_at Instant`, `LocalState` in `src/player.rs`).

Queue types: `QueueItem{id, title, artists, album, duration_ms, artwork_url: Option<String>, playable: bool}`. Artwork URLs are Apple's CDN templates; they are not tokens. Caution for the IPC-02 walk: a field named `artwork_url` is fine, but avoid `auth`-containing names for anything else (`author` matches `auth`: do not name a field `author`; use `artist`/`composer`). The allowlist exists for deliberate exceptions, e.g. the `Auth` event variant's payload `state`.

### Pattern 4: Transport (tokio)
```rust
// presto-ipc/src/transport.rs  (Source: tokio-util docs, LinesCodec)
pub fn socket_path() -> PathBuf {
    let base = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join(format!("presto-{}", unsafe { libc_uid() })));
    base.join("presto/engine.sock")
}
pub type Conn = Framed<UnixStream, LinesCodec>;
pub fn framed(s: UnixStream) -> Conn { Framed::new(s, LinesCodec::new_with_max_length(4 * 1024 * 1024)) }
// bind: create_dir_all(parent) with mode 0700; remove stale socket file; UnixListener::bind
```
Skip `libc_uid`: when `XDG_RUNTIME_DIR` is unset just error with a clear message (Linux-first, systemd always sets it); tests pass an explicit path. Child spawn contract (D-02) is Phase 3's job; Phase 1 only fixes the contract: engine argv `--socket <path> --profile <dir>`, env `PRESTO_SOCKET`, `PRESTO_PROFILE`. Put the arg names in PROTOCOL.md so the mock and the real engine take the same ones. Mock should accept both, flags winning.

### Pattern 5: Mock engine structure
One tokio task per connection with `select!` over: inbound frames, a 250 ms playback tick (or 1 s plus on-demand emit), and the heartbeat interval. State: `Player{queue: Vec<QueueItem>, rev, index, state, position_ms, volume, shuffle, repeat}`. Position computed from `Instant` of last state change, not by incrementing (no drift, easy seek). Track end: advance index, bump `rev`, emit `QueueChanged` and `TrackChanged`.

Fault state is `Arc<Mutex<Faults>>` or plain fields in the single task (single task is simpler: no lock). `Faults { hang: bool, auth_expired: bool, slow: Option<Duration>, crash_after: Option<Duration> }`.
- hang: stop writing replies and pongs, keep reading (drain) so the socket stays open; ignore everything except `Mock` control frames? Realistic hang ignores them too; recommend the mock still honours `MockControl{fault: none}` so tests can recover without respawn (document it).
- crash: `std::process::exit(101)` immediately (non-zero, no flush); optionally `crash --after-ms N`.
- auth_expired: emit `Evt::Auth{Expired}` once on activation; every `Req` and `Cmd` replies `Err(AuthExpired)` until cleared.
- slow: `sleep(delay)` before each reply (default 3 s; param `slow=1500`). Progress/pong stay timely, so slow is distinguishable from hang.

Fault syntax: `--fault hang`, `--fault slow=2000`, `--fault crash`, `--fault crash@5000`, `--fault auth_expired`, repeatable; `FaultSpec` is a serde enum shared with `MockControl`, parsed with `FromStr`.

### Anti-Patterns to Avoid
- **Fields named `token`, `auth*`, `bearer`, `jwt`, `secret`, `cookie`, `authorization`, `developer_token`, `music_user_token`** anywhere in `presto-ipc`, including headers in `ApiRequest`. D-08 has no headers field; keep it that way (a `headers` map would invite `Authorization`). Whitelist `query` and `body` only.
- **Debug-printing frames in logs** is fine now but engine adapters later will see real responses; keep the "never log bodies at info" rule in PROTOCOL.md.
- **`serde(flatten)` with internal tags** (see Pattern 1).
- **Unbounded line length**: always set `new_with_max_length`; library responses can be megabytes (4 MB chosen; page limit of 100 items stays below it; document the cap and have the engine paginate).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Line framing | custom buffer/split loop | `tokio_util::codec::LinesCodec` with max length | partial reads, UTF-8, oversize guard |
| JSON schema | hand-written schema | `schemars` derive | the point of D-14/D-15 |
| Snapshot compare | custom diff | `insta` | review workflow |
| CLI parsing | manual argv | `clap` derive | `--fault` repeatable |
| Request/response correlation | bespoke future map | `HashMap<u64, oneshot::Sender<Outcome>>` plus `tokio::time::timeout` (simple; not a library) | fine to write, it is ~30 lines; do not build a generic RPC layer |

**Key insight:** this phase is small. The value is in a precise, stable contract; resist adding generic RPC machinery.

## Runtime State Inventory
Omitted: greenfield phase, no rename or migration.

## Common Pitfalls

### Pitfall 1: Stale socket file
**What goes wrong:** `UnixListener::bind` fails with `AddrInUse` after a crash left `engine.sock`.
**How to avoid:** remove the path before bind (single-instance guarding belongs to a later phase); create parent dir with mode 0700 (`DirBuilder::mode`) and chmod the socket 0600.
**Warning signs:** tests flaky on rerun.

### Pitfall 2: Unix socket path length limit
**What goes wrong:** `sun_path` is about 108 bytes; deep `tempfile` dirs in tests exceed it on some systems.
**How to avoid:** tests use short dirs (`tempfile::Builder::new().prefix("p").tempdir_in("/tmp")`), not nested paths.

### Pitfall 3: Schema test passes vacuously
**What goes wrong:** the walk finds no properties because the schema root uses `$defs`/`oneOf` and the walker only reads top-level `properties`.
**How to avoid:** walk the whole `serde_json::Value` recursively, collecting every key under any `"properties"` object, and also enum variant tags (`const`/`enum` strings in `oneOf`), since a variant named `AuthToken` is also a leak. Add a canary test: a struct with a `bearer: String` field must make the checker fail (guards against vacuous pass). Assert the collected set is non-empty and contains a known name like `position_ms`.
**Note:** schemars 1.x renamed things vs 0.8 (`Schema` wraps a JSON value; `SchemaSettings` changed; default draft is 2020-12 with `$defs`). If a snippet from the web uses `RootSchema`, it is 0.8.

### Pitfall 4: `author`/`authorization` false positives
Substring `auth` hits `author`. Use word-ish matching on snake_case segments: split names on `_`, camelCase boundaries, flag segments in {token, auth, authorization, bearer, jwt, secret, cookie, password, credential}. Then `AuthExpired` as a variant would also trip; the allowlist holds the explicit exceptions (`auth` event, `auth_expired` error kind) with a comment each. D-15 says allowlist; keep it short and exact-match.

### Pitfall 5: Hang detection vs `slow` mock
Heartbeat must be independent of request handling, or `slow` looks like `hang`. Mock sends pongs from the same select loop but never awaits the slow sleep inline: spawn the delayed reply as a task (or keep a delay queue). Test both.

### Pitfall 6: Tick drift and seek races
Compute position from a base `(position_at_start, Instant)`. Seek resets base. Stale `Progress` events can arrive after a seek: include `seq` (incremented on every state-changing user action) on `PlaybackState` and `Progress`, mirroring spotifast's `track_sequence`/`seek_sequence`, so the UI can drop stale ones. Cheap to add now, painful later.

### Pitfall 7: Queue revisions and the mock
`rev` increments on every queue mutation the mock performs, including natural advancement. `SetQueue` replies after the first `QueueChanged` with the new rev. Test: events with rev lower than last applied never appear (and a helper `QueueMirror::apply` in presto-ipc? No: that is Phase 3 core; only document).

### Pitfall 8: Child cleanup in tests
Integration tests that spawn the mock must kill it on drop (`Command::kill_on_drop(true)` with tokio::process) or orphans accumulate.

## Code Examples

### Token-name walk (IPC-02)
```rust
// crates/presto-ipc/tests/schema.rs  (schemars 1.x)
use schemars::schema_for;
use serde_json::Value;

const FORBIDDEN: &[&str] = &["token","auth","authorization","bearer","jwt","secret","cookie","password","credential"];
const ALLOW: &[&str] = &["auth", "auth_expired"]; // exact names, each justified in a comment

fn names(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Object(m) => for (k, v) in m {
            if k == "properties" { if let Value::Object(p) = v { out.extend(p.keys().cloned()); } }
            if k == "const" { if let Value::String(s) = v { out.push(s.clone()); } }
            if k == "enum" { if let Value::Array(a) = v { out.extend(a.iter().filter_map(|x| x.as_str().map(String::from))); } }
            names(v, out);
        },
        Value::Array(a) => a.iter().for_each(|x| names(x, out)),
        _ => {}
    }
}
fn bad(n: &str) -> bool {
    !ALLOW.contains(&n) && n.split(|c: char| c == '_' || c == '-').any(|seg| FORBIDDEN.contains(&seg.to_lowercase().as_str()))
}
#[test]
fn no_token_like_fields() {
    let mut v = Vec::new();
    names(&serde_json::to_value(schema_for!(presto_ipc::Frame)).unwrap(), &mut v);
    assert!(v.iter().any(|n| n == "position_ms"), "walk is vacuous");
    let hits: Vec<_> = v.iter().filter(|n| bad(n)).collect();
    assert!(hits.is_empty(), "token-like names: {hits:?}");
}
#[test]
fn snapshot() {
    insta::assert_json_snapshot!(schema_for!(presto_ipc::Frame));
}
```
CamelCase variant names are not split by this snippet; with `rename_all = "snake_case"` on every enum all wire names are snake_case. Make that a convention and add a test asserting no uppercase letters in collected names, or the walk misses `AuthToken`.

### Mock connect and hello (tokio)
```rust
let stream = UnixStream::connect(&args.socket).await?;
let mut conn = presto_ipc::transport::framed(stream);
conn.send(serde_json::to_string(&Frame::Hello(Hello::engine("presto-engine-mock", env!("CARGO_PKG_VERSION"), caps)))?).await?;
```

### Integration test shape
Test binary binds a listener on a short tempdir socket, spawns `env!("CARGO_BIN_EXE_presto-engine-mock")` with `--socket`, accepts, exchanges hello, then asserts: Play yields `PlaybackState::Playing` then progress ticks; Seek moves position; track end advances queue with `rev+1`; `Req GET /v1/me/library/playlists` returns canned JSON; each fault (hang: no pong within 3 intervals but socket not EOF; crash: child exit status non-success and EOF; auth_expired: event then `Err(AuthExpired)`; slow: reply latency >= delay while pongs stay timely). Use `tokio::time::pause` only for pure logic tests, not process tests.

## Spotifast Seams (input for D-17 doc; verified from source `crmne/spotifast` main, 2026-10-07)

| Seam | Location | Notes |
|------|----------|-------|
| `Backend` struct | `src/backend.rs:876` | fields: `commands: mpsc::UnboundedSender<Command>`, `events: std::sync::mpsc::Receiver<Event>`, `art: ArtLoader`, `offline: bool`, thread handle |
| `Backend::send(&self, Command)` | `src/backend.rs:1004` | when `offline`, only a small allow-list of commands passes (Accent, Shutdown, update commands): this is spotifast's demo mechanism |
| `Backend::poll(&self) -> Vec<Event>` | `src/backend.rs:1222` | `events.try_iter().collect()` |
| `Command` enum | `src/backend.rs:564` | mixed: Spotify-coupled (SignIn, CredentialsRestored, ProxyRestored, AuthorizePlayback, RestartEngine), plus UI-agnostic ones; carries `request: u64` ids |
| `Event` enum | `src/backend.rs:749` | `Auth(AuthStatus)`, `Playback(LocalPlayback)`, `Local(Box<LocalState>)`, `Api(Box<ApiResponse>)`, plus update, proxy, receivers variants |
| `ApiRequest` / `ApiResponse` | `src/backend.rs:108` / `:338` | about 40 variants each, carry `generation: u64` |
| `AuthStatus`, `LocalPlayback`, `RemoteAction` | `src/backend.rs:78`, `:855`, `:88` | |
| `PlayerCommand`, `LocalState` | `src/player.rs:280`, `:165` | playback commands and state (`position_ms`, `position_at`, `track_sequence`, `seek_sequence`) |
| API client | `src/api/` (dir), `src/http.rs`, `src/limiter.rs`, `src/session_reads.rs` | Spotify-coupled; Presto replaces with the proxied `Request` |
| Demo | `src/demo.rs` | injects state; not a mock backend |
| Library crate | `src/lib.rs` (`pub mod api; pub mod app; ...`) | spotifast exposes internals as a lib |

The spotifast repo is a single crate, no `[workspace]`. `ApiRequest` and `Command` are defined in `backend.rs`, not a separate module; the repo's GitHub landing page does not document them, so the seam doc must cite source lines. Line numbers will drift: cite the commit SHA when writing the doc (`git ls-remote https://github.com/crmne/spotifast HEAD`).

**Fork and fastframe verification (partial, git-level):**
- `https://github.com/crmne/egui` branch `apps-0.36` head is `ba6790fe7cf46e58e8d27ce1524cbfdee745e938` (matches spotifast `[patch.crates-io]`). Public.
- Patched crates, all at that rev: ecolor, eframe, egui, egui-wgpu, egui-winit, egui_extras, egui_glow, emath, epaint, epaint_default_fonts. Also patched: crmne/winit `apps-0.30`, plus others spotifast needs (librespot suite, projectm-sys, hyper-proxy2) that Presto does not need. Presto's `[patch]` copies only the egui set (and the winit patch, since egui-winit depends on it; confirm by `cargo tree`).
- `https://github.com/crmne/fastframe` tag `v0.4.1` exists. Crates (spotifast's list): fastframe-text, -fonts, -icons, -theme, -emoji, -i18n, -log, -tray, -shell, -macos, -update, -audio, -scroll, -instance, -now-playing. `fastframe-audio` is referenced via a git fork in spotifast and is not needed by Presto.
- Not yet done: an actual `cargo` resolution. Procedure for the plan: in `scratchpad`-style throwaway dir (not the workspace, so Phase 1 doesn't pull egui into its build), create a crate with `eframe = "0.36"`, `fastframe-now-playing = { git = ".../fastframe", tag = "v0.4.1" }`, `fastframe-shell`, `fastframe-instance`, the `[patch.crates-io]` block copied verbatim, then `cargo fetch` (or `cargo tree`) and record pass/fail in the seam doc. Do not add these to the Phase 1 workspace (slow builds, unnecessary).

## State of the Art

| Old | Current | Impact |
|-----|---------|--------|
| schemars 0.8 (`RootSchema`, `schema_for!` returns struct tree) | schemars 1.x (`Schema` over `serde_json::Value`, draft 2020-12) | walking the schema is plain JSON traversal |
| `thiserror` 1 | 2.0 | derive syntax same for this use |

## Open Questions

1. **Does `Cmd` need its own id and ack?**
   - spotifast commands are fire-and-forget; D-05 says request IDs exist. Recommendation: commands carry `id`, engine answers `Res` (ok or error). It costs one frame per command and lets the UI show failures (e.g. AuthExpired on Play). Decide in plan; document in PROTOCOL.md.
2. **MockControl placement.** It is in the schema, so IPC-02 walks it. It carries no token-like fields. Recommendation: include it in `Frame` under a `mock` capability that real engines never advertise; presto refuses to send it unless the hello lists `mock`.
3. **Rust toolchain 1.98 vs 1.99.** Stable is 1.99.0 (2026-09-28); spotifast pins 1.98.0. Recommendation: pin 1.98.0 until the egui phase confirms a bump is harmless.
4. **Is anything in presto-ipc allowed to depend on tokio?** Recommendation: yes, only the `transport` module, behind a default-on `transport` feature, so the schema test and any future JS-side codegen use no tokio.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | built-in `cargo test` (libtest) + insta 1.49 for snapshot; tokio `#[tokio::test]` |
| Config file | none; Wave 0 creates workspace `Cargo.toml` and `rust-toolchain.toml` |
| Quick run command | `cargo test -p presto-ipc` |
| Full suite command | `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` |

### Phase Requirements to Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| IPC-01 | every Frame variant serde-roundtrips; hello major mismatch errors, minor tolerated; `Kind::timeout` defined for every kind | unit | `cargo test -p presto-ipc --test roundtrip` | Wave 0 |
| IPC-01 | schema snapshot matches (doc/type drift) | snapshot | `cargo test -p presto-ipc --test schema snapshot` | Wave 0 |
| IPC-01 | PROTOCOL.md lists every Command/Event/error variant | unit (cheap): test reads `docs/PROTOCOL.md` and asserts each variant wire name appears | `cargo test -p presto-ipc --test doc_covers_variants` | Wave 0 |
| IPC-02 | no token-like property or variant names; canary proves walker catches `bearer` | unit | `cargo test -p presto-ipc --test schema no_token` | Wave 0 |
| IPC-03 | mock answers full protocol over a real Unix socket | integration | `cargo test -p presto-engine-mock --test protocol` | Wave 0 |
| IPC-03 | each fault: hang, crash, auth_expired, slow (flag and live `MockControl`) | integration | `cargo test -p presto-engine-mock --test faults` | Wave 0 |
| D-17 | seam doc exists, fork and fastframe resolve | manual (one-time, needs network) | `cargo fetch` in throwaway crate, result recorded in doc | n/a |

### Sampling Rate
- Per task commit: `cargo test -p presto-ipc` (or the touched crate)
- Per wave merge: full suite command
- Phase gate: full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] Install Rust toolchain (rustup, 1.98.0), none present on this machine
- [ ] Workspace `Cargo.toml`, `rust-toolchain.toml`, `.gitignore` (`target/`)
- [ ] `crates/presto-ipc/tests/{roundtrip,schema}.rs`, `crates/presto-engine-mock/tests/{protocol,faults}.rs`
- [ ] `cargo install cargo-insta` (optional; `INSTA_UPDATE=always` or `.snap.new` rename works without it)

## Sources

### Primary (HIGH confidence)
- crates.io API, versions checked 2026-10-07: schemars 1.2.2, tokio 1.53.2, tokio-util 0.7.19, serde 1.0.229, serde_json 1.0.151, clap 4.6.7, thiserror 2.0.21, insta 1.49.0, tracing 0.1.44
- `raw.githubusercontent.com/crmne/spotifast/main/Cargo.toml` (patch block, versions, edition 2024, rust 1.98), `rust-toolchain.toml` (1.98.0), `src/backend.rs` (type locations above)
- `git ls-remote` on `crmne/egui` (branch apps-0.36 = ba6790fe) and `crmne/fastframe` (tag v0.4.1)
- static.rust-lang.org channel manifest: stable 1.99.0 (2026-09-28)
- `.planning/research/ARCHITECTURE.md` (prior spotifast study, consistent with source read here)

### Secondary (MEDIUM)
- schemars 1.x API behavior (`Schema` wrapping `Value`, 2020-12 default) from training plus changelog memory; not re-verified with Context7 this session. The planner should have the first task compile the schema test early to confirm.

### Tertiary (LOW)
- Timeout and heartbeat numbers: judgment, tune in Phase 2.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH, versions checked against the registry
- Architecture: MEDIUM-HIGH, standard tokio patterns; serde flatten/tag caveat flagged
- Pitfalls: MEDIUM, from experience; schemars 1.x specifics need a compile check
- Seams: HIGH for locations, partial for cargo resolution (no toolchain here)

**Research date:** 2026-10-07
**Valid until:** 2026-11-07

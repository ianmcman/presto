---
phase: quick-261008-bsb
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/presto-core/src/data/artwork.rs
autonomous: true
requirements: [QUICK-261008-bsb]

must_haves:
  truths:
    - "ArtCache::fetch refuses any URL that is not https on a host ending in .mzstatic.com, with no userinfo and no explicit non-default port"
    - "The reqwest client never follows redirects; a 3xx response is a failure"
    - "Responses over 10 MB are rejected via Content-Length before reading, and via running byte count while streaming; no .part or final file is left behind"
    - "Existing http://127.0.0.1 stub tests still pass via a cfg(test)-only loopback allowance"
  artifacts:
    - path: "crates/presto-core/src/data/artwork.rs"
      provides: "url_allowed guard, no-redirect client, MAX_ART_BYTES streamed cap, tests"
      contains: "MAX_ART_BYTES"
  key_links:
    - from: "ArtCache::fetch"
      to: "url_allowed"
      via: "first statement in fetch, before any network call"
      pattern: "url_allowed"
    - from: "ArtCache::open"
      to: "reqwest::redirect::Policy::none"
      via: "Client::builder().redirect(...)"
      pattern: "Policy::none"
---

<objective>
Harden `ArtCache::fetch` in crates/presto-core/src/data/artwork.rs per the user-approved spec: host/scheme allowlist, no redirects, 10 MB cap enforced on both Content-Length and streamed bytes.

Output: modified artwork.rs with new unit tests. No other files.
</objective>

<execution_context>
@~/.claude/get-shit-done/workflows/execute-plan.md
@~/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@./CLAUDE.md
@crates/presto-core/src/data/artwork.rs

Facts already checked:
- reqwest 0.12, `default-features = false, features = ["rustls-tls"]`. No `stream` feature. Use `Response::chunk()` (no feature needed) and `Response::content_length()`. `reqwest::Url` is the re-exported `url::Url`; `reqwest::redirect::Policy::none()` is available without extra features. Do NOT touch Cargo.toml.
- Only caller outside the file is crates/presto-core/src/data/mod.rs (`ArtCache::open(paths.artwork.clone(), ART_CAP)`); signatures of `open`, `get`, `fetch` stay unchanged. No integration test in crates/presto-core/tests/ calls fetch, so a `#[cfg(test)]` override is sufficient.
- `get()` already records any `fetch` error in `failed`, so a rejected URL shows up as `ArtState::Failed` with no extra code.

PARALLEL WORK WARNING: other agents are editing crates/presto-core/src/data/mod.rs, crates/presto-core/tests/data_library.rs and .planning/config.json in this tree. Stage ONLY crates/presto-core/src/data/artwork.rs (and this quick dir's SUMMARY) by explicit path. Never `git add -A` / `git add .` / `git commit -a`.
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: URL allowlist, no redirects, streamed 10 MB cap, with tests</name>
  <files>crates/presto-core/src/data/artwork.rs</files>
  <behavior>
    - url_allowed("https://is1-ssl.mzstatic.com/image/a/320x320bb.jpg") is true; "https://mzstatic.com/x" false (spec says *.mzstatic.com)
    - false for: "http://is1-ssl.mzstatic.com/x", "https://evilmzstatic.com/x", "https://mzstatic.com.evil.com/x", "https://user:pw@is1.mzstatic.com/x", "https://is1.mzstatic.com@evil.com/x", "https://is1.mzstatic.com:8443/x", "not a url", "file:///etc/passwd"
    - "https://is1.mzstatic.com:443/x" is allowed (url crate normalizes default port to None); assert whichever the guard does is true, it must not be rejected
    - Upper-case host "https://IS1.MZSTATIC.COM/x" allowed (url crate lowercases)
    - fetch of a disallowed URL returns Err without any network hit (stub hit counter stays 0 for an http://localhost:{port} URL, since only 127.0.0.1 is test-allowed) and writes nothing
    - Redirect: stub replying 302 with Location to a 200 path yields Err from fetch, and the stub sees exactly 1 hit
    - Content-Length over cap: stub advertises Content-Length MAX_ART_BYTES+1 (send only a few bytes then close) -> Err, scan(dir) empty, total_bytes()==0
    - Streamed over cap: stub sends no Content-Length (HTTP/1.1 200, Connection: close, body until close) of MAX_ART_BYTES+1 bytes -> Err, scan(dir) empty (no .part, no final file), total_bytes()==0
    - Body exactly MAX_ART_BYTES via no-Content-Length stub succeeds (boundary)
    - All existing tests unchanged and passing
  </behavior>
  <action>
1. Add `pub const MAX_ART_BYTES: u64 = 10 * 1024 * 1024;`.

2. Add a free function:
```rust
/// Artwork may only come from Apple's CDN: https, *.mzstatic.com, no userinfo, default port.
fn url_allowed(url: &str) -> bool {
    let Ok(u) = reqwest::Url::parse(url) else { return false };
    #[cfg(test)]
    if u.scheme() == "http" && u.host_str() == Some("127.0.0.1") {
        return true; // test stubs only; compiled out of real builds
    }
    u.scheme() == "https"
        && u.username().is_empty()
        && u.password().is_none()
        && u.port().is_none()
        && u.host_str().is_some_and(|h| h.ends_with(".mzstatic.com"))
}
```
The leading dot in `.mzstatic.com` is what rejects `evilmzstatic.com`. Do not match on the raw string; always go through `Url::parse` so `user@host` and `host@evil` tricks resolve to the real host.

3. In `ArtCache::open`, add `.redirect(reqwest::redirect::Policy::none())` to the client builder. With redirects off reqwest returns the 3xx as-is; `error_for_status()` does NOT error on 3xx, so in fetch explicitly require `resp.status().is_success()` (replace `error_for_status` with a success check returning `io::Error::other(format!("artwork http {}", status))`).

4. Rewrite the network half of `fetch`:
   - First line: `if !url_allowed(url) { return Err(io::Error::new(io::ErrorKind::PermissionDenied, "artwork url not allowed")); }`
   - Send, check success, then `if resp.content_length().is_some_and(|n| n > MAX_ART_BYTES) { return Err(...too large...) }`.
   - Read with `while let Some(c) = resp.chunk().await.map_err(io::Error::other)? { if buf.len() as u64 + c.len() as u64 > MAX_ART_BYTES { return Err(...) } buf.extend_from_slice(&c); }` into a `Vec<u8>` (pre-size with `content_length().unwrap_or(0).min(MAX_ART_BYTES)`). Returning before the spawn_blocking write means no .part or final file can exist on rejection. Keep the existing spawn_blocking write/rename, total accounting, and eviction exactly as they are, just fed `buf`.
   - ponytail: buffering up to 10 MB in memory is fine for artwork (images are ~100 KB); no need to stream to disk.

5. Tests (inside existing `mod tests`). Extend the stub minimally: add a second helper `raw_stub(resp: Vec<u8>) -> (String, Arc<AtomicUsize>, JoinHandle<()>)` that writes the given raw bytes to each connection then shuts down; the existing `stub(n)` stays as-is. Use it to build: a 302 response (`HTTP/1.1 302 Found\r\nLocation: /ok\r\nContent-Length: 0\r\nConnection: close\r\n\r\n`), a lying Content-Length (`Content-Length: {MAX_ART_BYTES+1}` header + 10 bytes), and no-Content-Length bodies of MAX_ART_BYTES+1 and MAX_ART_BYTES bytes (`HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n` + body). For the 302 test assert hits == 1 after fetch returns. Add a pure `#[test] fn url_allowlist()` covering every case in <behavior>. Add a disallowed-URL fetch test using `http://localhost:{port}` from a stub and assert hits == 0 and Err.
   Keep stub writes ignoring errors (`let _ =`) since the client may hang up early on oversize bodies.
  </action>
  <verify>
    <automated>cd /home/mcmanusiang/presto && cargo test -p presto-core --lib data::artwork && cargo clippy -p presto-core --all-targets -- -D warnings</automated>
  </verify>
  <done>All artwork tests (old and new) pass; clippy clean; Cargo.toml unchanged; `git diff --stat` for this commit lists only crates/presto-core/src/data/artwork.rs.</done>
</task>

</tasks>

<verification>
- `cargo test -p presto-core --lib data::artwork` green.
- `cargo build -p presto-core --release` succeeds (confirms the cfg(test) branch compiles out cleanly with no unused warnings).
- `grep -n "Policy::none\|url_allowed\|MAX_ART_BYTES" crates/presto-core/src/data/artwork.rs` shows the guard, client config, and cap.
</verification>

<success_criteria>
fetch rejects non-https, non-*.mzstatic.com, userinfo, and explicit-port URLs before any request; redirects are not followed; bodies over 10 MB fail via header or stream count with nothing written to disk; existing tests untouched and passing.
</success_criteria>

<output>
After completion, create `.planning/quick/261008-bsb-harden-artwork-fetch-https-mzstatic-host/261008-bsb-SUMMARY.md`. Commit with `git add crates/presto-core/src/data/artwork.rs .planning/quick/261008-bsb-harden-artwork-fetch-https-mzstatic-host/` (explicit paths only).
</output>

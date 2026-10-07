# Domain Pitfalls

**Domain:** Native Linux Apple Music client driving music.apple.com in a hidden Widevine Chromium engine
**Researched:** 2026-10-07
**Overall confidence:** MEDIUM-LOW. Only the Cider/castlabs and CDM-source facts were checked this session (web search). Everything else is from training data and is marked LOW where it matters. Verify in the spike.

## Critical Pitfalls

### 1. Treating the page's MusicKit instance as a stable API
**What goes wrong:** The bridge calls `MusicKit.getInstance()` or reaches into app internals. Apple ships a new web player build, a global is renamed, scoped into a module, or the instance is created lazily/late, and the bridge silently stops working.
**Why:** music.apple.com is a private SPA, not a public SDK surface. Apple has no contract with you. (LOW: exact current global shape unverified; spike must confirm.)
**Prevention:**
- Spike must prove how the instance is reached today (global, Ember/webpack module lookup, or intercepting the constructor/`configure` call) and record the fallback chain.
- Bridge script is runtime-loaded (already decided). Add a versioned capability handshake: bridge reports `{musicKitVersion, features:[...]}` on load, and Rust degrades per-feature instead of failing wholesale.
- Wait for readiness via an explicit event/poll with timeout, never `setTimeout` guesses.
- Pin a "known-good web player" smoke test (script that loads the page, signs in on a test profile, plays a track) and run it on a schedule.
**Detection:** Handshake reports missing features; ready event never fires; `play()` resolves but no `playbackStateDidChange`.
**Phase:** Engine spike (first), then IPC/bridge phase. Ongoing maintenance item.

### 2. Rust ends up needing the developer token (violates the project constraint)
**What goes wrong:** Library/search/recommendations need Apple API calls that carry the developer token plus music-user-token. Proxying "through the page" is fine; the temptation is to read the token from the page and call from Rust for speed.
**Prevention:** Proxy via `fetch` inside the page origin (the page's own MusicKit `api.music(...)` helper or same-origin fetch). Rust sends `{id, method, path, query}` and gets JSON back. Lint the bridge: it must never post a `Authorization`/`Music-User-Token` value over IPC. Add an IPC schema test that rejects any message field matching those headers.
**Detection:** Any code reading `localStorage`/cookies for tokens; IPC payloads containing JWTs (`eyJ`).
**Phase:** IPC protocol phase.

### 3. Widevine not actually accepted (CDM source, version, robustness level)
**What goes wrong:** Page loads, but playback fails with a key-system/license error, or falls back to 30-second previews or an "unsupported browser" wall. Causes: no CDM at all (CEF and plain Chromium builds do not ship it), CDM too old, CDM version mismatched to the Chromium/CEF version, or Chromium without the component-updater wiring.
**Why:** Linux Widevine is L3 (software only). Apple's web player accepts L3 for AAC; it has historically restricted some formats (lossless/Atmos) on browsers without hardware DRM (LOW: verify per current behavior; do not promise lossless). Stock CEF needs `libwidevinecdm.so` supplied and registered with a manifest, with a version compatible with the CEF build. Chrome's CDM is only licensed for use inside Chrome and licensed partners.
**Prevention:**
- Spike matrix: castlabs ECS (what Cider ships; AUR has `electron*-castlab-bin` packages, so it is a proven Linux path), CEF + CDM copied from system Chrome, system Chrome via CDP. Score each on: CDM loads, `requestMediaKeySystemAccess('com.widevine.alpha')` succeeds, full track plays, survives a Chromium upgrade.
- Prefer an engine where the CDM is obtained by a sanctioned mechanism (castlabs bundles/updates it via component updater; system Chrome owns its own).
- Never ship or repack `libwidevinecdm.so` in the repo, AppImage, or Flatpak. Fetch from user's Chrome install or at runtime via the engine's updater.
- Surface a clear "DRM unavailable" state in the UI, not a silent hang.
**Detection:** `chrome://components` Widevine at version 0.0.0.0; EME access promise rejects; track "plays" at 0:00 or stops at 0:30.
**Phase:** Engine spike. This is the go/no-go gate.

### 4. Confusing 30-second previews with full playback (and false spike success)
**What goes wrong:** Spike "passes" because a preview or a non-DRM stream plays. Or the user is not signed in/subscribed, and MusicKit silently serves previews.
**Prevention:** Spike acceptance test: signed-in subscriber account plays a full-length (>3 min) catalog track past 60 s and across a seek, and plays a library-only/uploaded track. Bridge exposes `isAuthorized`, `hasSubscription` (or equivalent) and the engine reports `previewOnly` explicitly. Assert duration from player state vs track metadata.
**Detection:** Playback ends at exactly ~30 s; `playbackDuration` 30.
**Phase:** Engine spike, then playback phase.

### 5. Anti-automation detection from driving the browser (CDP/webdriver)
**What goes wrong:** Apple (or its CDN/fraud layer) flags `navigator.webdriver === true`, headless user agent, missing plugins/codecs, or CDP-attached `Runtime.enable` side effects. Result: login captcha loops, "unusual activity" blocks, degraded playback, or account risk. (LOW: Apple's actual detection posture is unverified; assume it exists for login.)
**Prevention:**
- Do not run headless. Use headed (hidden/offscreen) mode, normal UA matching the engine's real Chromium version, no `--enable-automation`, no `--remote-debugging-port` exposed longer than needed.
- Prefer a embedded-engine API (CEF/Electron preload + native messaging) over attaching CDP, so no `webdriver` flag is set.
- Do the sign-in in a visible window with the real Apple flow and a human; never script credentials or 2FA.
- Keep request rate human-like: the proxy must debounce/limit and not hammer `api.music` endpoints (cap concurrency, cache aggressively).
**Detection:** Login page loops; 403/429 from API; captcha appears only in engine, not in normal Chrome.
**Phase:** Engine spike (choice of attach method), sign-in phase.

### 6. Apple ToS / account risk ignored
**What goes wrong:** Automating music.apple.com and presenting it in a third-party UI likely conflicts with Apple Media Services / Apple Music terms (no unauthorized access, automation, or circumvention of the service's intended client). Possible outcomes range from nothing to account action. (LOW: not re-read this session; read the current Apple Media Services Terms before the spike.)
**Prevention:** Personal use only, stated in README. Stay on the legitimate path: unmodified player, real CDM, no stream capture, no download/offline of DRM content, no token reuse off-origin (already out of scope). Avoid Apple trademarks/logos in the app name and icons. Use a secondary account for development testing if the user is risk-averse.
**Phase:** Before spike (decision recorded), revisit before any public release.

### 7. Session persistence and re-auth treated as solved
**What goes wrong:** Sign-in works once, then the music-user-token/cookies expire, the profile is locked by a stale process, or profile corruption after a crash forces re-login. Cookie storage encryption (Chromium keyring/`os_crypt`) breaks when run without a secret service, making the session unreadable after reboot.
**Prevention:**
- Dedicated engine profile dir under XDG data, single-instance lock, cleanly closed on exit.
- Test: kill -9 the engine mid-play, relaunch, still signed in. Test: reboot with and without gnome-keyring/kwallet running. Choose `--password-store` explicitly.
- Bridge emits `authStateChanged`; Rust UI shows a "Sign in again" action that raises the engine window to Apple's login. Never block the UI on it.
- Handle 2FA/trusted-device prompts by showing the real window.
**Detection:** Logged-out after restart; `SingletonLock` errors; cookie decrypt failures in logs.
**Phase:** Spike (persistence check), sign-in/recovery phase.

### 8. Two sources of truth for the queue
**What goes wrong:** Rust queue and MusicKit queue diverge (autoplay/continuation, radio stations, shuffle/repeat, Apple's own "next up" injection). Skip/prev behave differently than UI shows; MPRIS shows wrong track.
**Prevention:** Decision in planning: make MusicKit the owner for Apple-generated queues (stations, autoplay), and have Rust mirror it from events. Rust sends intent commands (`playItems`, `skipTo(index)`), never maintains a parallel list. Every event carries a monotonically increasing `seq`; Rust drops stale ones. Test: stations, shuffle toggle, end-of-queue autoplay.
**Phase:** IPC/queue design phase, before UI port.

### 9. Hidden engine memory defeats "lightweight"
**What goes wrong:** Chromium plus the Apple SPA idles at several hundred MB (typically 300-800 MB across browser, GPU, renderer; LOW: measure). The app is marketed as lightweight but is heavier than a browser tab.
**Prevention:** Set an honest budget: native UI is light, engine is the cost. Measure in the spike (RSS+PSS across all engine processes, idle and playing). Mitigations: disable unneeded features (`--disable-extensions`, `--disable-background-networking`, no GPU compositing if offscreen where video not needed, small renderer limit, block images/fonts for UI chrome), lazy-start engine only when playback or library fetch is needed, quit it on idle if playback stopped. Do not render the web UI at all: navigate to a minimal page if possible. State the budget in docs.
**Detection:** Idle RSS over budget; engine kept alive after pause for hours.
**Phase:** Spike (measure), packaging/hardening phase.

### 10. Hidden window that Chromium throttles or suspends
**What goes wrong:** Minimized/occluded/offscreen windows get timer throttling, background tab freezing, or audio suspension. Playback stalls, events delay, progress stops, next track fails to load.
**Prevention:** Keep the window shown but offscreen/1x1 or use the engine's offscreen mode; pass `--disable-background-timer-throttling --disable-renderer-backgrounding --disable-backgrounding-occluded-windows`; add `CalculateNativeWinOcclusion` off on Windows later. Test a 2 hour playback with the UI minimized and screen locked, including track transitions (gapless/crossfade behavior).
**Phase:** Spike, playback phase.

## Moderate Pitfalls

### 11. PipeWire / audio quirks
**What goes wrong:** Engine audio appears as a separate "Chromium" stream with a bad name/icon; wrong sample rate resampling; device switching (Bluetooth, USB DAC) leaves the stream on a dead sink; volume desync between app slider and engine stream; crackle with small buffers; PulseAudio vs native PipeWire backend differences.
**Prevention:** Set the stream name/app id via engine flags; control volume through MusicKit `volume` and also expose it, not through PipeWire stream hacks; test hot-unplug and sink switch; check `--enable-features=AudioServiceOutOfProcess` defaults. Apple's web player outputs fixed-rate; do not claim bit-perfect.
**Phase:** Playback phase, audio test checklist.

### 12. Wayland / X11 differences
**What goes wrong:** Hidden window behaves differently: Wayland does not allow positioning offscreen or truly hidden mapped windows (compositors may not render unmapped surfaces, which can stall Chromium frame/visibility). Ozone flag mismatches cause crashes (`--ozone-platform=x11` via XWayland is stable, native Wayland less so). Window raise for sign-in may be denied by focus-stealing prevention. GPU/VA-API flags cause black screen or crashes on some drivers.
**Prevention:** Test both sessions in the spike. Default to a mapped tiny/transparent or XWayland window if hidden-unmapped stalls. For sign-in, show a normal window the user clicks into rather than programmatically raising. Provide `PRESTO_ENGINE_FLAGS` env escape hatch. Disable GPU for the engine if it only needs audio.
**Phase:** Spike, hardening.

### 13. MPRIS fed from the wrong place
**What goes wrong:** MPRIS state drifts from real playback (position, seek, metadata); media keys also reach the web player's own MediaSession handler, causing double handling (two MPRIS players: Chromium registers its own `org.mpris.MediaPlayer2.chromium...`).
**Prevention:** Disable the engine's MediaSession/MPRIS (`--disable-features=MediaSessionService,HardwareMediaKeyHandling`), expose one MPRIS from Rust driven by engine events. Test KDE, GNOME, and waybar/playerctl.
**Phase:** MPRIS phase.

### 14. IPC hang/crash/reload not designed
**What goes wrong:** Page reloads (Apple pushes an update, session expiry, renderer crash) drop the bridge and in-flight requests; Rust waits forever; stale request IDs leak.
**Prevention:** Every proxied request has timeout and ID; on `engine_disconnected` or `bridge_reloaded`, fail all pending with a typed error and re-handshake; supervisor restarts engine with backoff and restores playback state (track, position) from last known events. Version field in handshake; refuse mismatched bridge/engine pairs with a clear message.
**Phase:** IPC protocol phase.

### 15. Library data assumptions
**What goes wrong:** Apple's library API paginates (limit 25/100), has catalog vs library IDs that differ, storefront-specific availability, and rate limits. Large libraries (10k+ songs) load slowly; cached snapshots go stale; lyrics endpoints may not exist or need extra params.
**Prevention:** Paginate with concurrency cap, snapshot with `lastModified`/incremental refresh, normalize catalog/library IDs in one layer, treat lyrics as optional capability reported by the handshake. Cache artwork by URL template + size.
**Phase:** Library/proxy phase.

### 16. Chromium version drift and security
**What goes wrong:** A pinned old CEF/Electron/Chromium that browses a live site with auth cookies accumulates unpatched CVEs; conversely, a bump breaks CDM compatibility.
**Prevention:** Restrict navigation to `*.apple.com`, `*.mzstatic.com`, and known auth/CDN hosts (block all else); no extensions; sandbox on. Track upstream releases; engine is replaceable by design, so keep a CI job that boots it and plays a track.
**Phase:** Hardening/packaging.

## Distribution Blockers (flag, do not solve in v1)

| Target | Blocker | Notes |
|--------|---------|-------|
| AUR | CDM redistribution | Package `presto` and `presto-engine` with the engine depending on an existing AUR engine (e.g. `electron*-castlab-bin` exists) or user's `google-chrome`. Never include the CDM. `-bin` engine packages are acceptable in AUR since the user builds from sources they trust. MEDIUM. |
| AppImage | Same CDM issue, plus bundling a Chromium engine is large (150-300 MB) and sandbox (`chrome-sandbox` SUID) often does not work inside AppImage (needs `--no-sandbox` which is a security regression). LOW-MEDIUM. |
| Flatpak | Chromium sandbox inside Flatpak requires `org.chromium.Chromium.BaseApp`/zypak; Widevine delivery is the hard part (Flathub Chromium handles it via extension; a third-party app cannot redistribute). Widevine CDM licensing and Flathub policy on proprietary blobs are likely blockers. Treat as "probably not feasible in v1", verify with Flathub maintainers before investing. LOW. |
| castlabs ECS | Free builds require acceptance of castlabs' terms; they sign with VMP (EVS) for some services. Whether Apple requires VMP is unverified (Cider shows it works on Linux with castlabs). Check licensing for any redistribution. MEDIUM. |
| Branding | Apple trademarks; keep "Presto" generic. |

## Minor Pitfalls

- **Locale/storefront:** User's storefront affects catalog and language; set in bridge, not hard-coded to `us`.
- **Cookie consent / region banners** can block page readiness; bridge should not depend on UI DOM.
- **Time sync:** token and license failures when system clock is off; show a helpful error.
- **Bridge injection timing:** inject before page scripts (document-start) to capture constructor calls; late injection misses events.
- **Fork drift from spotifast:** shared UI code diverges; keep playback and API-client seams as traits so spotifast upstream changes still merge.
- **Autoplay policy:** Chromium blocks audio without a user gesture; set `--autoplay-policy=no-user-gesture-required`.
- **Demo mode** must exercise the same IPC schema, otherwise it hides protocol bugs.

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|---------------|------------|
| Spike: engine choice | #3 CDM, #4 previews, #5 detection | Matrix test across 3 engines with real subscriber account; go/no-go gate |
| Spike: persistence | #7 session lost on crash/reboot | kill -9 and reboot tests |
| Spike: measurement | #9 RAM, #10 throttling, #12 Wayland | Record numbers in the spike report |
| IPC protocol | #2 tokens leak, #14 crash recovery, #8 queue | Schema tests, seq numbers, supervised restarts |
| UI port | Fork drift, queue desync | Keep seams as traits, UI reads mirrored state only |
| MPRIS | #13 double MPRIS | Disable engine MediaSession |
| Library/proxy | #15 pagination, rate limits | Concurrency cap, snapshot cache |
| Packaging | CDM redistribution, sandbox, Flatpak | AUR first, no CDM in artifacts, defer Flatpak |

## Sources

- Cider uses castlabs Electron (ECS) for Widevine on Linux and Windows: https://en.ubunlog.com/cider-available-linux-windows/ (MEDIUM)
- castlabs Electron builds packaged in AUR: https://aur.archlinux.org/packages/electron41-castlab-bin (MEDIUM)
- CEF Widevine loader requires user-supplied CDM: https://third-party-mirror.googlesource.com/cef/+/refs/heads/master/src/libcef/common/widevine_loader.h (MEDIUM)
- Everything else (Apple detection, ToS wording, MusicKit internals, Flatpak/Flathub policy, memory figures, Wayland behavior): training data, LOW. Validate in the spike.

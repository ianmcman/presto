# Technology Stack

**Project:** Presto (native Linux Apple Music client, hidden Widevine engine)
**Researched:** 2026-10-07

## Recommendation

Engine for the spike: **castlabs Electron (ECS), hidden window, preload bridge, Unix-socket NDJSON to Rust.** Fallback: **system Google Chrome over CDP with a dedicated profile.** Defer: **CEF via cef-rs.**

Why: Sidra (wimpysworld/sidra, updated May 2026) ships exactly this design today (music.apple.com in castlabs Electron, hook script tapping `MusicKit.getInstance()` events, forwarded over Electron IPC, MPRIS on top). Cider also uses castlabs Electron for Widevine on Linux. This is the only path with existing proof for Apple Music on Linux. (MEDIUM: Sidra README via fetch; Cider via search snippets.)

## Recommended Stack

### Native app (`presto`)
| Technology | Version | Purpose | Why | Confidence |
|---|---|---|---|---|
| Rust | stable (spotifast sets 1.98, edition 2024) | language | match reference | MEDIUM |
| eframe / egui / egui_extras | 0.36 (spotifast Cargo.toml), features `accesskit, glow, default_fonts, links, wayland, x11, persistence` | UI | spotifast pins 0.36 | MEDIUM |
| crmne/egui fork, rev `ba6790fe` | git pin | RTL shaping, emoji clusters, Wayland frame pacing | spotifast depends on it via patch. Porting its views needs the same fork, or they will render/pace differently. Start on the fork rev; drop to crates.io 0.36 only if a view ports cleanly without it | MEDIUM |
| tokio | 1.x (multi-thread) | async runtime | same as spotifast | HIGH |
| serde + serde_json | 1.x | IPC payloads | | HIGH |
| tokio-util (`codec::LinesCodec` or `LengthDelimitedCodec`) | 0.7 | framing | NDJSON lines are enough | MEDIUM |
| reqwest | 0.12 (rustls) | artwork/image fetch only | Rust must not call api.music.apple.com; artwork CDN is fine | MEDIUM |
| keyring-core | 1 | not needed for Apple tokens (Rust holds none) | skip unless caching something else | MEDIUM |
| MPRIS | whatever spotifast's fastframe-now-playing uses; fallback `mpris-server` + zbus | media keys | spotifast's fastframe crates are its own framework: check availability in Step 1 | LOW |
| rodio, librespot | NOT USED | | playback is in the engine | HIGH |

fastframe-* crates (audio, i18n, fonts, emoji, tray, now-playing) are spotifast's own; whether they are published or git-only was not verified. Flag for Step 1 study.

### Engine (`presto-engine`), candidates ranked

| Rank | Option | Widevine source | Verdict |
|---|---|---|---|
| 1 | castlabs ECS (npm via GitHub URL, e.g. `github:castlabs/electron-releases#v44.1.0+wvcus`; supported series v43-v45 per wiki, one build/month) | CDM auto-downloaded on first launch via component updater (v16+ `components` API) | Use. Proven by Sidra and Cider. Linux needs no VMP signing; macOS/Windows need free EVS signing for production |
| 2 | System Google Chrome/Chromium + CDP, dedicated `--user-data-dir` | Chrome ships/updates its own CDM; nothing redistributed by us | Best licensing story, thinnest engine. Risks: no truly hidden headful window on Wayland (minimize/offscreen hacks), headless Widevine unreliable (LOW), user-installed browser version drift, automation banner/flags might be detected. Rust side: raw CDP over `--remote-debugging-pipe` or `chromiumoxide`; use `Runtime.addBinding` + `bindingCalled` for events |
| 3 | CEF via cef-rs (`cef` 154.5.0+154.0.34 on docs.rs, tauri-apps, MIT/Apache-2.0) | CEF binary distributions do NOT bundle the CDM. You must supply `libwidevinecdm.so` + manifest in `WidevineCdm/_platform_specific/linux_x64/` and load it (CEF's widevine_loader; on Linux the CDM must load in the zygote at startup, so no hot-add). Component updater only works in Chrome-runtime mode | Defer. Smallest runtime, but you own CDM sourcing, CEF-version-to-CDM compatibility, and a ~300MB CEF dist. Highest effort, no Apple Music precedent found |
| 4 | WebKitGTK/wry | none | Rejected (project Out of Scope) |

Electron host stack for ECS: Node bundled with Electron, `net` module for the Unix socket (no npm deps), `BrowserWindow({show:false})` with a persistent `session.fromPartition('persist:presto')` or `--user-data-dir` under `$XDG_DATA_HOME/presto/engine`. Bridge script loaded from disk at runtime (`fs.readFileSync` then `webContents.executeJavaScript` or a preload that reads the file), per project decision.

### Widevine CDM: sourcing and licensing
- Widevine is proprietary. Google does not allow general redistribution; builds that bundle it (Chrome, Brave, castlabs) have a Google agreement. ungoogled-chromium and Helium do not bundle it because certification is costly (LOW to MEDIUM, secondary sources).
- Linux x64 CDM is delivered via Chrome's component updater to other Chromium browsers (M131+ per Axinom/DoveRunner notices). castlabs ECS runs that updater for you. No Linux aarch64 CDM exists, so ECS aarch64 Linux is not possible (Cider docs).
- Implication: personal use is fine with ECS download-at-first-run or system Chrome. AUR/AppImage must download the CDM at runtime, never ship it. Flatpak: ship ECS binary (castlabs redistributes it) and let it fetch the CDM into the user data dir. Flag the Apple ToS question separately; no research found on enforcement.
- ECS licensing: castlabs disclaims warranty ("AS IS"), EVS signing is free. Verify EVS terms before public distribution (not needed on Linux for personal use).

### IPC options (Rust <-> engine)
| Option | Verdict |
|---|---|
| Unix domain socket at `$XDG_RUNTIME_DIR/presto/engine.sock`, NDJSON, both sides | Recommended. Works for Electron (`net`), CDP-less engines, and a mock engine for `--demo`. Single protocol for all engines. Include `proto` version in a hello frame, request IDs, timeouts, heartbeat for hang detection |
| Child stdio (stdin/stdout NDJSON) | Acceptable alternative. Electron logs pollute stdout; use fd 3 or the socket instead |
| WebSocket on localhost | Avoid. Needs port/auth handling for no gain |
| CDP pipe/websocket directly | Only for the Chrome fallback; wrap it inside an engine adapter so Rust still sees the same NDJSON protocol |
| gRPC/protobuf/D-Bus | Overkill; D-Bus reserved for MPRIS |

Rust spawns and supervises the engine (tokio `Command`, `kill_on_drop`), restarts on crash/heartbeat loss.

## Alternatives Considered
| Category | Recommended | Alternative | Why Not |
|---|---|---|---|
| Engine | castlabs ECS | cef-rs | CDM sourcing burden, no precedent |
| Engine | castlabs ECS | Chrome via CDP | Hidden window and version-drift problems; keep as fallback |
| Engine | castlabs ECS | stock Electron | no Widevine CDM |
| UI | egui (spotifast fork) | Slint/iced | breaks shared code with reference |
| Cider as base | | fork Cider | Closed-ish, shifting to Rust backend/early access; not a library |

## Installation
```bash
# Native
cargo add tokio --features full
cargo add serde --features derive
cargo add serde_json tokio-util --features tokio-util/codec
# egui: git pin to crmne/egui rev ba6790fe via [patch.crates-io], eframe 0.36

# Engine
npm install "https://github.com/castlabs/electron-releases#v44.1.0+wvcus" --save-dev
```

## Open items for the spike
- Confirm the exact current ECS tag (v44.1.0 came from the README example; wiki lists v43-v45 supported).
- Verify Wayland behavior of a `show:false` ECS window and that playback runs while hidden (autoplay policy; may need `--autoplay-policy=no-user-gesture-required`).
- Confirm whether the crmne/egui fork rev is pulled via `[patch]` and whether it is public.
- Confirm fastframe-* crate availability.

## Sources
- spotifast Cargo.toml (raw.githubusercontent.com/crmne/spotifast/main/Cargo.toml), README: https://github.com/crmne/spotifast (MEDIUM, summarized by fetch)
- https://github.com/castlabs/electron-releases and its wiki (MEDIUM)
- https://github.com/wimpysworld/sidra (MEDIUM)
- https://docs.rs/crate/cef/latest, https://github.com/tauri-apps/cef-rs (MEDIUM)
- CEF widevine_loader.cc mirror: https://third-party-mirror.googlesource.com/cef/+/refs/heads/master/src/libcef/common/widevine_loader.cc (MEDIUM)
- Cider search snippets, AUR cider package (LOW to MEDIUM)
- Axinom/DoveRunner CDM announcements, nite07.com Widevine writeup (LOW)

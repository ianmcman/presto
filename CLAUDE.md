<!-- GSD:project-start source:PROJECT.md -->
## Project

**Presto**

A native, lightweight Apple Music desktop client for Linux (primary), with macOS/Windows as later nice-to-haves. Rust + egui UI structured after crmne/spotifast (MIT), with playback and library access delegated to a hidden Widevine-capable Chromium engine that runs Apple's own web player. Personal use first.

**Core Value:** Full-catalog Apple Music playback and library browsing from a fast native Linux UI, without an Apple Developer account.

### Constraints

- **Tech stack**: Rust stable, egui/eframe matching spotifast versions unless justified — shared UI code with the reference
- **Architecture**: Two processes, native `presto` and replaceable `presto-engine`, JSON IPC between them — engine is an implementation detail, never visible except possibly at sign-in
- **Auth**: No developer account, no .p8, no extracted tokens — user constraint
- **Secrets**: None in repo or binary
- **Workflow**: Planning only until the user approves the plan; engine spike is planned first, then stops for go-ahead
- **Distribution**: Personal use first; flag anything blocking Flatpak/AUR (CDM redistribution, ToS)
<!-- GSD:project-end -->

<!-- GSD:stack-start source:research/STACK.md -->
## Technology Stack

## Recommendation
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
### Engine (`presto-engine`), candidates ranked
| Rank | Option | Widevine source | Verdict |
|---|---|---|---|
| 1 | castlabs ECS (npm via GitHub URL, e.g. `github:castlabs/electron-releases#v44.1.0+wvcus`; supported series v43-v45 per wiki, one build/month) | CDM auto-downloaded on first launch via component updater (v16+ `components` API) | Use. Proven by Sidra and Cider. Linux needs no VMP signing; macOS/Windows need free EVS signing for production |
| 2 | System Google Chrome/Chromium + CDP, dedicated `--user-data-dir` | Chrome ships/updates its own CDM; nothing redistributed by us | Best licensing story, thinnest engine. Risks: no truly hidden headful window on Wayland (minimize/offscreen hacks), headless Widevine unreliable (LOW), user-installed browser version drift, automation banner/flags might be detected. Rust side: raw CDP over `--remote-debugging-pipe` or `chromiumoxide`; use `Runtime.addBinding` + `bindingCalled` for events |
| 3 | CEF via cef-rs (`cef` 154.5.0+154.0.34 on docs.rs, tauri-apps, MIT/Apache-2.0) | CEF binary distributions do NOT bundle the CDM. You must supply `libwidevinecdm.so` + manifest in `WidevineCdm/_platform_specific/linux_x64/` and load it (CEF's widevine_loader; on Linux the CDM must load in the zygote at startup, so no hot-add). Component updater only works in Chrome-runtime mode | Defer. Smallest runtime, but you own CDM sourcing, CEF-version-to-CDM compatibility, and a ~300MB CEF dist. Highest effort, no Apple Music precedent found |
| 4 | WebKitGTK/wry | none | Rejected (project Out of Scope) |
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
## Alternatives Considered
| Category | Recommended | Alternative | Why Not |
|---|---|---|---|
| Engine | castlabs ECS | cef-rs | CDM sourcing burden, no precedent |
| Engine | castlabs ECS | Chrome via CDP | Hidden window and version-drift problems; keep as fallback |
| Engine | castlabs ECS | stock Electron | no Widevine CDM |
| UI | egui (spotifast fork) | Slint/iced | breaks shared code with reference |
| Cider as base | | fork Cider | Closed-ish, shifting to Rust backend/early access; not a library |
## Installation
# Native
# egui: git pin to crmne/egui rev ba6790fe via [patch.crates-io], eframe 0.36
# Engine
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
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

Conventions not yet established. Will populate as patterns emerge during development.
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

Architecture not yet mapped. Follow existing patterns found in the codebase.
<!-- GSD:architecture-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd:quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd:debug` for investigation and bug fixing
- `/gsd:execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->



<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd:profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->

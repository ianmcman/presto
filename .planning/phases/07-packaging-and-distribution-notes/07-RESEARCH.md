# Phase 7: Packaging and Distribution Notes - Research

**Researched:** 2026-10-08
**Domain:** Arch PKGBUILD for Rust binary + npm-installed castlabs Electron; CDM runtime-fetch status IPC; distribution doc
**Confidence:** MEDIUM (ldd and code paths verified locally; ECS components API verified in castlabs docs and electron.d.ts; legal sources are secondary summaries)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** The PKGBUILD fetches castlabs ECS (`v44.5.1+wvcus`, from `engine/package.json`) at build time (`npm ci` in `build()`), and installs `engine/` with its Electron under `/usr/lib/presto/engine`. No CDM is fetched or shipped at build time.
- **D-02:** The Electron binary is redistributed in the package archive. Accepted for personal use; recorded in DISTRIBUTION.md.
- **D-03:** The default `engine_dir` resolves relative to the executable (`../lib/presto/engine`), falling back to `./engine` for dev checkouts. `--engine-dir` still overrides.
- **D-04:** `depends=()` is derived from `ldd` of the Electron binary and `presto` (researcher lists the libs).
- **D-05:** One package, `presto-git`, built from GitHub main (`ianmcman/presto`). No tagged-release PKGBUILD.
- **D-06:** PKGBUILD lives at `packaging/arch/PKGBUILD`. It is not published to the AUR in this phase; publishing is a separate user decision tied to the ToS question.
- **D-07:** Ship a minimal `presto.desktop` (Audio/Music categories) and a simple hand-written SVG icon.
- **D-08:** Verification: `makepkg` in a clean chroot, a scripted scan of the package file list asserting no `libwidevinecdm`, then a manual first-run checklist (fresh profile, sign in, play), in the style of the Phase 6 checklist.
- **D-09:** The engine reports CDM state via a new additive IPC event with a proto minor bump. PROTOCOL.md, the schema snapshot and the token-guard test are updated.
- **D-10:** The app shows status text ("Preparing playback components...") in the existing signed-out/loading UI while the CDM downloads.
- **D-11:** On download failure the UI shows an error naming the cause. Retry happens on next launch; no in-app Retry button.
- **D-12:** No CDM version pin or check. Record the observed version (4.10.3112.0 so far) in docs.
- **D-13:** `docs/DISTRIBUTION.md`, linked from README.
- **D-14:** Cites sources for CDM licensing (Widevine, castlabs), Apple Media Services terms and Flathub policy. States known vs unknown; draws no legal conclusion.
- **D-15:** Short feasibility paragraph each for Flatpak and AppImage. No packaging work.
- **D-16:** Includes an end-user section: install, first-run CDM wait, sign-in, data/profile paths, uninstall cleanup.

### Claude's Discretion
- Exact `depends=()` list, PKGBUILD function layout, icon design.
- Name and payload of the CDM IPC event (additive, token-free).
- Chroot tooling (`devtools` vs plain `makepkg`) and scan script form.

### Deferred Ideas (OUT OF SCOPE)
- Publishing to the AUR (needs user go-ahead and ToS review)
- Tagged-release PKGBUILD
- In-app CDM Retry button
- Flatpak and AppImage packaging (PKG-03)
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PKG-01 | AUR package builds with no Widevine CDM in any artifact; CDM fetched at runtime | PKGBUILD pattern, depends list, engine_dir resolution, CDM scan script, `components.whenReady()` runtime fetch (already in `engine/main.js:162`) |
| PKG-02 | Distribution blockers documented | Source list and known/unknown framing for DISTRIBUTION.md |
</phase_requirements>

## Summary

Most plumbing exists. `engine/main.js` already awaits `components.whenReady()` (line 162) before creating the window, so the CDM downloads at first run today with nothing bundled. The engine connects its IPC socket at module top level (line 74) before `app.whenReady`, so a CDM event can be sent during the download. The `components` object emits no events (castlabs docs), so the engine must report state itself: emit `checking` before `whenReady()`, `ready` (with version from `components.status()`) after, `failed` (with message) in the rejection path. Today a rejection is unhandled and the engine would hang or die silently; that needs a try/catch.

The presto binary links only libc, libm, libgcc_s. eframe with `wayland, x11, glow` dlopens libwayland-client, libxkbcommon, libX11 family, libGL/libEGL at runtime, so those must still be in `depends`. All Electron ldd libs map to the Arch packages listed below (computed with `pacman -Qo` on this machine).

Engine path: `Launch::electron` reads `<engine_dir>/node_modules/electron/path.txt` and runs `dist/electron` directly. For a system install that works unchanged if `npm ci` ran in `build()` and `node_modules/electron/dist` is copied intact (path.txt = `electron`). The only code change is the `--engine-dir` default.

**Primary recommendation:** Add `engine_dir` default resolution (exe-relative, then `./engine`), add a `cdm` event (proto 1.2), wrap `whenReady` in try/catch emitting it, write `packaging/arch/PKGBUILD` with `options=(!strip !debug)`, `npm ci` in `build()`, and a shell scan script.

## Standard Stack

### Core
| Tool | Version | Purpose | Why |
|------|---------|---------|-----|
| makepkg / `devtools` (`pkgctl build`, `extra-x86_64-build`) | system | clean chroot build (D-08) | Arch standard; `devtools` not installed here, `makepkg` is |
| npm (nodejs 24 here) | `npm ci` | install ECS from `package-lock.json` | lockfile already committed |
| castlabs ECS `v44.5.1+wvcus` | pinned in `engine/package.json` | Electron with Widevine component updater | locked by spike |
| namcap | optional | lint PKGBUILD/package (not installed here) | recommended, not required |

**Installation note:** the electron dependency is a GitHub URL (`github:castlabs/electron-releases#...`). Existing dev instructions use `npm install --allow-git=root`; `npm ci` in a chroot may need the same flag (`npm ci --allow-git=root`, verify) and needs `git` in makedepends and network in `build()` (makepkg allows network in `build()`; Arch policy discourages it for AUR, acceptable here, record it in DISTRIBUTION.md). Also run `node node_modules/electron/install.js` (the electron postinstall downloads the zip and writes `path.txt` and `dist/`); verify whether `npm ci` already runs it (the castlabs fork does the same as upstream electron postinstall unless `--ignore-scripts` is used). Do not use `--ignore-scripts`.

**Version verification:** `electron` pin read from `engine/package.json` (`v44.5.1+wvcus`); Node 24.21.0 present locally. Chrome version for UA derived at runtime.

## depends=() derivation

`ldd engine/node_modules/electron/dist/electron` resolved to these Arch packages (via `pacman -Qo`, no "not found" entries):

```
alsa-lib at-spi2-core avahi brotli bzip2 cairo dbus expat fontconfig freetype2 fribidi
gdk-pixbuf2 glib2 glibc glycin gmp gnutls graphite gtk3 harfbuzz icu json-glib leancrypto
libcloudproviders libcups libdatrie libdrm libepoxy libffi libgcc libidn2 libp11-kit libpng
libseccomp libstdc++ libtasn1 libthai libunistring libx11 libxau libxcb libxcomposite
libxcursor libxdamage libxdmcp libxext libxfixes libxi libxinerama libxkbcommon libxml2
libxrandr libxrender mesa nettle nspr nss pango pcre2 pixman sqlite systemd-libs tinysparql
util-linux-libs wayland zlib-ng-compat
```

Most are transitive (gtk3 pulls cairo, pango, atk, etc.). Recommended direct `depends` (let pacman resolve the rest; confirm with `namcap` or re-run ldd in the chroot):

```
depends=(gtk3 nss alsa-lib libcups libdrm mesa libxkbcommon libxcomposite libxdamage
         libxrandr libx11 libxcb at-spi2-core dbus glib2 pango cairo wayland libxcursor
         libxi libxinerama libxext libxfixes libxrender systemd-libs)
```

- Runtime extras for presto (egui dlopens): `wayland libxkbcommon libx11 libxcursor libxi libxrandr libxcb mesa libglvnd` (libGL/libEGL via libglvnd; confidence MEDIUM, verify by running presto in the chroot). `ldd target/release/presto` shows only libc/libm/libgcc_s.
- Weak libs from Electron ldd to note: `libglycin-2`, `libtinysparql`, `libcloudproviders` come from gtk3's own deps on current Arch; do not list them directly.
- Optional: `optdepends=('libnotify')`, not needed by anything researched; skip.
- makedepends: `cargo git nodejs npm` (rust pinned by `rust-toolchain.toml`; check its channel and use `rustup` or `rust` accordingly). Include `clang`/`pkgconf` only if the build fails without them.
- `options=(!strip !debug !lto)`: stripping Electron's `electron` binary and `.so`s is unsafe and slow (280 MB dist); `!debug` avoids empty debug package. Strip presto manually with `strip` in package() if desired, or let cargo `profile.release` strip.
- `provides=(presto) conflicts=(presto)`; `pkgver()` using `git describe --long --tags | sed ...` or `printf "r%s.%s" "$(git rev-list --count HEAD)" "$(git rev-parse --short HEAD)"` (no tags guaranteed, so use rev-count form).
- `arch=(x86_64)` only (no aarch64 CDM; per CLAUDE.md).
- `license=(MIT)`? Repo's `package.json` says UNLICENSED and there is no LICENSE file; ask the user or use `custom`. LOW confidence item, see Open Questions.

## Architecture Patterns

### Recommended layout
```
packaging/
  arch/PKGBUILD
  presto.desktop
  presto.svg
  scan-no-cdm.sh          # lists a built .pkg.tar.zst, fails on libwidevinecdm
docs/DISTRIBUTION.md
.planning/phases/07-.../07-MANUAL-CHECKLIST.md
```

### Pattern 1: PKGBUILD (git source, npm in build)
```bash
pkgname=presto-git
pkgver=r0.0
pkgrel=1
arch=(x86_64)
url=https://github.com/ianmcman/presto
makedepends=(cargo git nodejs npm)
options=(!strip !debug)
source=("git+$url.git")
sha256sums=(SKIP)
pkgver() { cd presto; printf 'r%s.%s' "$(git rev-list --count HEAD)" "$(git rev-parse --short HEAD)"; }
build() {
  cd presto
  cargo build --release --locked -p presto
  cd engine && npm ci --allow-git=root   # downloads ECS; no CDM
}
package() {
  cd presto
  install -Dm755 target/release/presto "$pkgdir/usr/bin/presto"
  install -d "$pkgdir/usr/lib/presto"
  cp -a engine "$pkgdir/usr/lib/presto/engine"
  rm -rf "$pkgdir/usr/lib/presto/engine/test"
  install -Dm644 packaging/presto.desktop "$pkgdir/usr/share/applications/presto.desktop"
  install -Dm644 packaging/presto.svg "$pkgdir/usr/share/icons/hicolor/scalable/apps/presto.svg"
}
```
Source: standard Arch VCS PKGBUILD guidelines (wiki.archlinux.org/title/VCS_package_guidelines, wiki.archlinux.org/title/Node.js_package_guidelines). Confidence MEDIUM; the electron-specific handling is from this repo's setup.

Notes:
- `cp -a` preserves the exec bit and any symlinks in `node_modules/.bin`. The `.bin/*` symlinks are relative, so they survive. `npm ci` may leave absolute `$srcdir` paths in `package.json` `_resolved`-style fields or `node_modules/.package-lock.json`; verify with `grep -r "$srcdir" $pkgdir` and fix (makepkg warns on `$srcdir` references via namcap). Consider `npm prune --omit=dev` which would delete electron (it is a devDependency!). Do NOT prune. Cleaner: move electron to `dependencies` in `engine/package.json`, or skip pruning.
- Reduce size: delete `node_modules/electron/dist/LICENSES.chromium.html` NO (license compliance, keep). Keep `LICENSE` files.
- `chrome-sandbox`: not setuid in the npm dist (mode 775). Spike verified that the namespace sandbox works without suid and without `--no-sandbox` (SPIKE-REPORT line 15). Arch kernels allow unprivileged user namespaces (`kernel.unprivileged_userns_clone=1` here). Therefore: install as 755, do NOT setuid (a root-owned suid binary in a user-readable path is extra attack surface). Document that hardened kernels with userns disabled would fail and the fix is a setuid `chrome-sandbox` (user decision), not `--no-sandbox`. Confirm in the chroot first-run check.
- The engine writes into `app.setPath('userData', profile)` (`~/.local/state/presto/engine-profile`), not into `/usr/lib/presto/engine`. Verify no Electron write into the install dir (read-only under /usr) in the manual run.
- `path.txt` contains `electron`; `Launch::electron` works unchanged.
- Bridge resolution: `engine/bridge-path.js` resolves `bridge.js` via env/home/dir (`resolveBridge`); check it finds `__dirname/bridge.js` when installed (engine dir is read-only; no home override). Read `engine/bridge-path.js` while planning.

### Pattern 2: exe-relative engine_dir (D-03)
Current: clap `#[arg(long, env = "PRESTO_ENGINE_DIR", default_value = "engine")]` at `crates/presto/src/cli.rs:16`, test at `cli.rs:131` asserts the default equals `engine`. `launch::real_config(&cli.engine_dir)` at `main.rs:51`.

Plan: keep the clap default as an `Option<PathBuf>` (no default_value), and add a pure function in `launch.rs`:
```rust
/// --engine-dir/env wins; else <exe>/../lib/presto/engine if it has node_modules; else ./engine.
pub fn default_engine_dir(exe: Option<&Path>) -> PathBuf {
    if let Some(p) = exe.and_then(|e| e.parent()).map(|d| d.join("../lib/presto/engine")) {
        if p.join("node_modules/electron/path.txt").exists() { return p; }
    }
    PathBuf::from("engine")
}
```
Use `std::env::current_exe()` (do not canonicalize; `/usr/bin/presto` -> `/usr/bin/../lib/presto/engine` resolves correctly). Unit test with a tempdir tree. Update the `cli_engine_dir_default` test and the `--help` text. Also note `Launch::electron` error text mentions npm install; extend with "or install the package".

Relevant files: `crates/presto/src/cli.rs`, `crates/presto/src/launch.rs`, `crates/presto/src/main.rs`, `crates/presto-core/src/config.rs`.

### Pattern 3: CDM event (proto 1.2)
Files to touch, in order:
1. `crates/presto-ipc/src/event.rs`: add variant to `Event`:
   ```rust
   /// Widevine CDM component state. Engines that do not report it never send it.
   Cdm { state: CdmState, version: Option<String>, message: Option<String> },
   ```
   with `enum CdmState { Checking, Ready, Failed }` (serde snake_case, `JsonSchema`). Wire names `cdm`, `checking`, `ready`, `failed`, `version`, `message`. None of the segments collide with FORBIDDEN (`token auth authorization bearer jwt secret cookie password credential`); `message` already likely exists elsewhere. Do not name anything `*_token` or `auth_*`.
2. `crates/presto-ipc/src/frame.rs:18`: `PROTO` minor 1 -> 2. Add a capability? Unneeded: older presto ignores unknown event? Check serde: `Event` is probably `#[serde(tag = "type")]` and an unknown tag would fail to parse in an old presto. Since presto and engine ship together, gate by capability anyway if the pattern exists (`caps` in frame.rs); simplest: add `caps::CDM = "cdm"` and have only the real engine list it. Planner should check how unknown event types are handled in `transport.rs`.
3. `crates/presto-ipc/tests/snapshots/schema__snapshot.snap`: update with `cargo insta accept` (or `INSTA_UPDATE=always`).
4. `crates/presto-ipc/tests/schema.rs::no_token_like_names`: add `"cdm"` to the collector sanity list optionally; test auto-covers new names.
5. `crates/presto-ipc/tests/doc_covers_variants.rs`: requires `PROTOCOL.md` to contain `` `cdm` ``, `` `checking` ``, `` `ready` ``, `` `failed` ``, `` `version` `` etc. in backticks, and `` `1.2` `` (test `doc_states_version`). Update `docs/PROTOCOL.md` line ~21 ("Protocol version is `1.1`" -> `1.2`, add changelog sentence) and the Events table (line ~100-110).
6. `crates/presto-engine-mock/`: the mock hello/version follows `PROTO`; optionally add a `--fault`-free path that emits `cdm ready` so tests/demo exercise the UI. Check for tests asserting the hello proto version.
7. `crates/presto-core/src/supervisor.rs` `on_event` (line ~296): add `Event::Cdm {..} => self.set(|c| c.cdm = ...)`; the `other => c.player.apply(&other)` arm would otherwise swallow it. Add `cdm: CdmStatus` (or `Option<CdmInfo>`) to the core state struct.
8. `crates/presto/src/ui/status.rs::blocking`: add a branch while engine is Starting (or not yet Ready) and `cdm == Checking` showing `tr("Preparing playback components...")`; on `Failed`, a `panel` with title naming the cause (`message`) and text "Restart Presto to retry." (D-11). Add strings to `i18n.rs`. Note supervisor "Ready only when bridge, auth event seen and restore done" (STATE decisions): the engine does not open the window until `whenReady` resolves, so UI sits in `Starting`; show the CDM text there, and if `Failed` the engine must not exit silently (also consider the supervisor's drift timer of 15s after `bridge_ready`; and the Starting timeout, check `Timings` so a slow first-run download of ~15 to 20 MB does not mark the engine failed. Verify the start timeout in supervisor `Timings::default()`; this is the main integration risk).

### Pattern 4: engine reporting (components API)
`components` emits no events (castlabs docs/api/components.md: "does not currently emit any events"). `whenReady([required])` resolves `ComponentResult[]` and rejects with `ComponentsError` (has `.errors[]`, each `ComponentError` with `.detail` = `{id, status, title, version}`). `status()` returns a record id -> `{status, title, version}`; call after `ready`. `components.WIDEVINE_CDM_ID` is the id constant. Types confirmed in `engine/node_modules/electron/electron.d.ts` (~line 7170-7275).

```js
// engine/main.js, replacing lines 162-163
send({ t: 'evt', evt: { type: 'cdm', state: 'checking', version: null, message: null } });
try {
  await components.whenReady();
  const st = components.status()[components.WIDEVINE_CDM_ID];
  log('cdm', JSON.stringify(components.status()));
  send({ t: 'evt', evt: { type: 'cdm', state: 'ready', version: st?.version ?? null, message: null } });
} catch (e) {
  const msg = (e.errors?.map((x) => `${x.detail?.id}: ${x.detail?.status}`).join('; ')) || e.message;
  log('cdm failed', msg);
  send({ t: 'evt', evt: { type: 'cdm', state: 'failed', version: null, message: msg } });
  return; // keep process alive for the UI; presto restarts it next launch
}
```
Caveats: `send` must be defined before `app.whenReady` callback (it is, line 70-ish; confirm it works pre-hello since the socket connects asynchronously and hello is sent on connect, so queue or check `sock` state). If `whenReady` rejects, the engine must stay alive long enough for presto to render the error (do not `app.quit()`). `components.status()` entry may be missing the `WIDEVINE_CDM_ID` key; guard with `?.`. Download needs network; offline first run yields rejection (confidence MEDIUM, behavior on timeout is unverified: `whenReady` could hang indefinitely with no network; consider `Promise.race` with a 120 s timer emitting `failed: timed out`). Also `engine/test` has node tests; add a test for the message formatter if extracted into a small pure function in a new file (follow `window-policy.js`/`guard.js` style).

CDM storage location (unverified in docs): in Chromium component-updater layout, under the profile (`userData`), e.g. `<engine-profile>/WidevineCdm/<version>/_platform_specific/linux_x64/libwidevinecdm.so`. Verify on this machine: `find ~/.local/state/presto -name 'libwidevinecdm*'` before documenting uninstall cleanup (D-16). Uninstall cleanup is therefore `rm -rf ~/.local/state/presto ~/.cache/presto` (paths from `Paths::from_env`: state `$XDG_STATE_HOME/presto` with `engine-profile`, `logs`; cache `cache_base` -> `presto.db`, `artwork`). Also check `~/.config/presto` (exists, empty here).

### Anti-Patterns
- Running `npm prune --omit=dev`: electron is a devDependency and would be removed.
- Hard-coding `/usr/lib/presto/engine` in Rust; use the exe-relative probe so dev checkouts and chroot builds work.
- Putting `--no-sandbox` in the launch args to dodge chrome-sandbox.
- Naming event fields with `token`/`auth` segments (token guard fails).
- Bumping PROTO major.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| CDM download | own downloader | castlabs `components.whenReady()` | already in main.js; updater handles versioning and signature checks |
| Chroot build | custom container | `devtools` (`pkgctl build` / `extra-x86_64-build`) or `makepkg` in a fresh container | standard |
| Snapshot update | hand-edit .snap | `cargo insta accept` | exact format |
| Package-content scan | custom archive parser | `bsdtar -tf pkg.tar.zst \| grep -i widevine` (also `-xOf` is unnecessary) | list is enough |

## Runtime State Inventory

Not a rename phase. Not applicable. (New state introduced: CDM lives in the engine profile under `~/.local/state/presto/engine-profile`, outside the package; uninstalling the package leaves it, documented per D-16.)

## Common Pitfalls

### Pitfall 1: Start timeout during first-run CDM download
**What goes wrong:** Supervisor marks the engine failed/restarts while the CDM downloads; restart kills the download.
**Why:** `whenReady` blocks window creation, so `bridge_ready`/auth never arrive.
**Avoid:** Read `Timings::default()` and the Starting-phase deadline in `crates/presto-core/src/supervisor.rs`; extend the deadline while the last `cdm` state is `checking` (heartbeat pings are answered by `main.js` independent of the page, so drift logic may be unaffected).
**Warning signs:** Engine restart loop on a fresh profile; log shows repeated `cdm` checking.

### Pitfall 2: Unknown event breaks old/new pairing
**What goes wrong:** Serde fails on the new `type`.
**Avoid:** Engine and presto ship together; still check `transport.rs` unknown-variant handling and add a roundtrip test in `roundtrip.rs`.

### Pitfall 3: npm ci with a GitHub git dependency in chroot
**What goes wrong:** Needs `git`, network, and (npm 11+) `--allow-git`. The electron postinstall must run to produce `dist/` and `path.txt`.
**Avoid:** `makedepends=(git nodejs npm)`, run build in `build()` only, verify `engine/node_modules/electron/path.txt` exists before `package()`.

### Pitfall 4: makepkg strips or compresses Electron
**What goes wrong:** `strip` corrupts or bloats; package is ~300+ MB.
**Avoid:** `options=(!strip !debug)`; accept size. Set `PKGEXT` only locally if build time matters.

### Pitfall 5: `$srcdir` leaks and build-time paths
`package.json`/lock fields or symlinks in `node_modules/.bin` may point into `$srcdir`. Grep the staged `$pkgdir`.

### Pitfall 6: Scan false confidence
The scan of the file list cannot see CDM downloaded at runtime (intended). Also scan the installed tree after `package()` and any `.so` named `*widevine*`; case-insensitive. Also check that the build did not trigger a CDM download (components only run when Electron runs; `npm ci` does not launch it).

### Pitfall 7: Hidden-window first run
Sign-in window must be visible to the user on a fresh profile; covered by Phase 3/6 `show_window`; include in manual checklist.

## Code Examples

### Scan script (`packaging/scan-no-cdm.sh`)
```bash
#!/bin/sh
# Fails if any listed path in the given package contains widevine.
set -eu
for p in "$@"; do
  if bsdtar -tf "$p" | grep -i 'widevine'; then echo "FAIL: CDM in $p" >&2; exit 1; fi
done
echo "ok: no widevine in $*"
```
Note the ECS dist includes `libffmpeg.so` but no CDM; the grep should pass. `LICENSES.chromium.html` text mentions "Widevine"? Grep is on path names only (bsdtar -t), so that is fine.

### Desktop file
```
[Desktop Entry]
Type=Application
Name=Presto
Comment=Apple Music for Linux
Exec=presto
Icon=presto
Categories=AudioVideo;Audio;Music;Player;
Terminal=false
StartupWMClass=presto
```
Check the egui `app_id` set in `crates/presto/src` matches `StartupWMClass`; validate with `desktop-file-validate` if available.

## State of the Art

| Old | Current | Impact |
|-----|---------|--------|
| ECS v15 "Lorry" CDM delivery | ECS v16+ CUS component updater (`components` API) | CDM auto-downloads on first launch, background update check later (castlabs README/wiki) |
| `widevine-ready` event | `components.whenReady()` / `status()` | no events on `components` |

## Distribution doc: sources for DISTRIBUTION.md

| Topic | Source | Known | Unknown |
|-------|--------|-------|---------|
| castlabs ECS terms | https://github.com/castlabs/electron-releases (README: "AS IS, WITHOUT WARRANTY"; updates best-effort), wiki CDM and EVS pages | warranty disclaimer; CDM via CUS in v16+; EVS signing free and needed on macOS/Windows only | explicit redistribution terms for the Electron binary inside a third-party package (D-02); read the repo LICENSE and wiki FAQ in full before writing |
| Widevine CDM | https://www.widevine.com/ and Chrome component updater; secondary (Axinom/DoveRunner notices, per CLAUDE.md) | not redistributable without Google agreement; Linux x64 CDM delivered by component updater | whether runtime fetch by third-party Chromium/ECS on an end-user machine is covered by a license for the end user (LOW) |
| Apple | https://www.apple.com/legal/internet-services/itunes/us/terms.html (Apple Media Services Terms) | personal, noncommercial use only; no software/automated process to "scrape, copy, or perform measurement, analysis, or monitoring of any portion of the Content or Services"; no play-count manipulation; Apple may monitor usage (search snippet; read the live text and quote exactly) | whether a UI driving the official web player in an embedded browser is a breach; enforcement unknown |
| Flathub | https://docs.flathub.org/docs/for-app-authors/requirements | "All content hosted on Flathub must allow legal redistribution"; non-redistributable binaries need `extra-data`; wrapper apps without significant polish/integration are rejected | whether an Electron+remote-site client with native UI would be accepted (the native egui UI is substantial, but the Electron engine is bundled); Apple-brand and ToS review by Flathub reviewers |

Feasibility paragraphs (D-15): Flatpak: ship ECS via `extra-data` (castlabs redistributes it) or bundle if license allows; CDM fetched into the user data dir at runtime; blockers are Apple ToS reviewer risk and the redistribution question. AppImage: possible since CDM is runtime-fetched; size ~300 MB; same legal questions; no store policy. These are paragraphs only.

Mark all legal findings "Known / Unknown", no conclusion (D-14). Fetch exact current wording of Apple terms (the text above comes from a search summary, MEDIUM) when writing.

## Open Questions

1. **License field for PKGBUILD**
   - Known: repo has no LICENSE file; `package.json` says UNLICENSED.
   - Recommendation: use `license=('custom')` or ask user; do not invent MIT.
2. **Does `npm ci` run electron's postinstall and need `--allow-git=root`?**
   - Recommendation: verify in the first chroot build; fall back to explicit `node node_modules/electron/install.js`.
3. **Where is the CDM stored and does `whenReady` time out offline?**
   - Recommendation: check on disk now; wrap in a timeout (120 s) in the engine.
4. **Supervisor start deadline vs download time** (see Pitfall 1).
5. **Unknown-event handling in transport** (see Pitfall 2).
6. **Runtime dlopen libs for egui in a bare chroot** (libglvnd, libxkbcommon, wayland): verify by running presto in the chroot.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Rust | cargo test, insta snapshots, schemars (workspace) |
| Engine | `node --test` in `engine/` (`npm test`) |
| Config | `crates/presto-ipc/tests/`, `engine/test/` |
| Quick run | `cargo test -p presto-ipc -p presto` ; `cd engine && npm test` |
| Full suite | `cargo test --workspace` and `cd engine && npm test` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | Exists? |
|--------|----------|-----------|-------------------|---------|
| PKG-01 | default engine dir resolves exe-relative then `./engine` | unit | `cargo test -p presto launch::tests::engine_dir` | Wave 0 (new test) |
| PKG-01 | clap `--engine-dir` override still wins; help text | unit | `cargo test -p presto cli::` | exists, update `cli_engine_dir_default` |
| PKG-01 | `cdm` event schema snapshot, token guard, doc coverage, version `1.2` | unit | `cargo test -p presto-ipc` | exists, update snapshot and PROTOCOL.md |
| PKG-01 | `cdm` event roundtrip and supervisor state | unit | `cargo test -p presto-ipc roundtrip; cargo test -p presto-core` | Wave 0 additions |
| PKG-01 | engine formats `cdm` failure message | node unit | `cd engine && npm test` | Wave 0 (new test, only if extracted) |
| PKG-01 | package contains no widevine | script | `sh packaging/scan-no-cdm.sh presto-git-*.pkg.tar.zst` | Wave 0 (new script) |
| PKG-01 | builds and runs on clean Arch; first run fetches CDM, plays | manual | `07-MANUAL-CHECKLIST.md` (chroot makepkg, install, fresh profile, sign in, play) | Wave 0 (new doc) |
| PKG-02 | DISTRIBUTION.md exists, lists three blockers, linked from README | smoke | `grep -q 'Flathub' docs/DISTRIBUTION.md && grep -q 'Apple Media Services' docs/DISTRIBUTION.md && grep -q 'DISTRIBUTION.md' README.md` | Wave 0 |

### Sampling Rate
- Per task commit: `cargo test -p presto-ipc -p presto-core -p presto`
- Per wave merge: `cargo test --workspace && (cd engine && npm test)`
- Phase gate: full suite green, scan script green on a real built package, manual checklist filled

### Wave 0 Gaps
- [ ] `packaging/scan-no-cdm.sh`
- [ ] `engine_dir` unit tests in `crates/presto/src/launch.rs`
- [ ] updated insta snapshot for `Frame` schema
- [ ] `07-MANUAL-CHECKLIST.md` (mirror `06-MANUAL-CHECKLIST.md`: numbered sections with Steps / Expected / Result)
- [ ] `devtools` install for clean-chroot build (`sudo pacman -S devtools`, not installed; `makepkg`, `namcap` status: makepkg present, namcap absent)

## Sources

### Primary (HIGH)
- Local: `engine/main.js`, `engine/node_modules/electron/electron.d.ts` (components types), `crates/presto-ipc/{src,tests}`, `crates/presto/src/{cli,launch,main}.rs`, `crates/presto-core/src/{config,supervisor,paths}.rs`, `docs/PROTOCOL.md`; `ldd` and `pacman -Qo` on this machine.
- https://raw.githubusercontent.com/castlabs/electron-releases/master/docs/api/components.md (status, whenReady, no events)
- https://raw.githubusercontent.com/castlabs/electron-releases/master/README.md (first launch install, AS IS)
- https://docs.flathub.org/docs/for-app-authors/requirements

### Secondary (MEDIUM)
- https://github.com/castlabs/electron-releases/wiki/CDM (CUS delivery in v16+; page content partial)
- https://www.apple.com/legal/internet-services/itunes/us/terms.html (via search snippet; quote exact text when writing)
- Arch wiki VCS and Node.js package guidelines (from knowledge; not fetched)

### Tertiary (LOW)
- Widevine redistribution position (secondary notices in CLAUDE.md); CDM disk location; offline `whenReady` behavior.

## Metadata

**Confidence:** stack MEDIUM (PKGBUILD details unverified until first chroot build); architecture HIGH for code paths; pitfalls MEDIUM; legal LOW to MEDIUM.
**Research date:** 2026-10-08
**Valid until:** 2026-11-07 (ECS releases monthly)

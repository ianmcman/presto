# Distribution notes

Last updated 2026-10-08. Presto is a personal project. This doc records what is known and unknown about distributing it. It draws no legal conclusion.

Source checking: the sources below could not be fetched while writing this (no web access in the writing environment). Every claim from a source is marked "(not re-checked 2026-10-08, from research summary)". Read the live pages before relying on any of it.

## What the package ships

- The Rust binaries (`presto`, `presto-engine-mock`).
- castlabs ECS Electron `v44.5.1+wvcus`, redistributed inside the package archive. Accepted for personal use.
- No Widevine CDM in any artifact. `packaging/scan-no-cdm.sh` fails if a package file list contains `widevine`.
- The CDM is downloaded at first run by ECS's component updater into the user's engine profile.

`build()` needs network for `npm ci` and the Electron download. AUR guidelines discourage network access in `build()`.

## Widevine CDM licensing

**Known:**
- The CDM is proprietary. Redistribution needs an agreement with Google (secondary sources, not re-checked 2026-10-08, from research summary).
- Only a Linux x64 CDM is delivered. There is no Linux aarch64 CDM.
- On Linux x64 the CDM reaches non-Chrome Chromium builds through the component updater. ECS runs that updater.
- CDM version observed on the development machine: `4.10.3112.0`. It is not pinned or checked by Presto.

**Unknown:**
- Whether a runtime fetch by ECS on an end-user machine is covered by a license held by the end user or by castlabs.
- Whether Google's terms for the component updater allow use from a third-party application like Presto.

Sources: https://www.widevine.com/ and secondary notices from Axinom and DoveRunner (not re-checked 2026-10-08, from research summary).

## Apple Media Services Terms

**Known:**
- The terms are titled Apple Media Services Terms and Conditions.
- They describe personal, noncommercial use (not re-checked 2026-10-08, from research summary).
- They contain a clause about software or automated processes used to scrape, copy, or measure and monitor the Content or Services, and a clause allowing Apple to monitor usage (not re-checked 2026-10-08, from research summary; quote the live text before citing).
- Presto uses no developer account, no `.p8` key and no extracted tokens. The user signs in on Apple's own web player.

**Unknown:**
- Whether driving the official web player inside an embedded browser from a native UI is permitted.
- Whether Apple enforces against such clients, and how.

Sources: https://www.apple.com/legal/internet-services/itunes/us/terms.html

## Flathub policy

**Known:**
- Flathub requires that hosted content allows legal redistribution. Non-redistributable binaries go through `extra-data` (not re-checked 2026-10-08, from research summary).
- Flathub rejects wrapper or website apps without significant polish or integration (not re-checked 2026-10-08, from research summary).

**Unknown:**
- Whether reviewers accept a bundled Electron engine plus the Apple terms risk.
- Whether a native egui UI over a hidden engine counts as more than a wrapper.

Sources: https://docs.flathub.org/docs/for-app-authors/requirements

## castlabs ECS terms

**Known:**
- The README disclaims warranty ("AS IS") and updates are best effort (not re-checked 2026-10-08, from research summary).
- EVS signing is free and needed only on macOS and Windows. Linux needs none.

**Unknown:**
- Explicit terms for redistributing the ECS binary inside a third-party distro package.

Sources: https://github.com/castlabs/electron-releases and its wiki (CDM and EVS pages).

## Flatpak feasibility

ECS could come in through `extra-data`, or be bundled if its license allows. The CDM would be fetched at runtime into the sandboxed user data dir. The blockers are the redistribution question and Flathub review of the Apple terms risk. No packaging work was done (PKG-03 is deferred).

## AppImage feasibility

Technically possible because the CDM is fetched at runtime. Expect about 300 MB. The legal questions above still apply, but there is no store review. No packaging work was done (PKG-03 is deferred).

## Publishing

Publishing to the AUR or Flathub is not done. It waits on the user's decision about the Apple terms.

## Installing and running (Arch)

Install:

```
git clone https://github.com/ianmcman/presto
cd presto/packaging/arch
makepkg -si
```

The package is `presto-git` (`license=('MIT')`, matching the repo LICENSE, installed to `/usr/share/licenses/presto-git/`), built from GitHub main. A clean-chroot build with `devtools` (`pkgctl build`) is optional.

Installed files: `/usr/bin/presto`, `/usr/bin/presto-engine-mock`, `/usr/lib/presto/engine/`, `/usr/share/applications/presto.desktop`, `/usr/share/icons/hicolor/scalable/apps/presto.svg`.

First run: Presto shows "Preparing playback components…" while the engine downloads the Widevine CDM (about 10 to 20 MB). If the download fails, the screen names the cause. Fix the network and restart Presto. There is no in-app retry.

Sign in: the Apple Music window opens on first run. Sign in there, then return to Presto.

Sandbox: Chromium's namespace sandbox needs unprivileged user namespaces (default on Arch kernels). On kernels with them disabled the engine fails to start. Do not use `--no-sandbox`.

Demo without an account: `presto --demo`.

### Data and profile paths

| Path | Contents |
|---|---|
| `~/.local/state/presto/` (`$XDG_STATE_HOME/presto`) | `engine-profile/` (Electron profile, Apple sign-in cookies), `logs/`, `engine.pid`, `install-id`, `demo/` |
| `~/.local/state/presto/engine-profile/WidevineCdm/<version>/_platform_specific/linux_x64/libwidevinecdm.so` | the downloaded CDM (also `WidevineCdm/latest-component-updated-widevine-cdm`) |
| `~/.cache/presto/` (`$XDG_CACHE_HOME/presto`) | `presto.db`, `artwork/` |
| `~/.config/presto/bridge.js` (`$XDG_CONFIG_HOME/presto`) | optional user bridge override |

### Uninstall

```
sudo pacman -Rns presto-git
rm -rf ~/.local/state/presto ~/.cache/presto ~/.config/presto
```

pacman does not touch the second line's paths. They hold the profile, sign-in cookies, the CDM and the cache.

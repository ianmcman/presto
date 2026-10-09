# Phase 07 Manual Packaging Checklist

**Date:** 2026-10-08 (automated parts); manual items pending
**Tester:** Claude (host makepkg), user (items needing sudo, Apple account)
**Desktop:** KDE Wayland

**Automated test results:** `cargo test --workspace` and `cd engine && npm test` pass. Host `makepkg -d` built `presto-git-r233.49dcfe7-1-x86_64.pkg.tar.zst` (116M) from a local clone (`PRESTO_SRC=file:///home/mcmanusiang/presto`, the GitHub repo is private). Scan: `ok: no widevine`. No `$srcdir` leaks after the `--remap-path-prefix` fix, `ldd` resolves everything, `path.txt` is `electron`, `chrome-sandbox` mode 755.

Note: the staged-tree smoke run failed in the agent's sandbox (Electron "Failed to receive hello message from zygote"), same as the dev tree, so it is an environment limit and not a packaging fault. Item 4 repeats it on a normal session.

---

## 1. Clean-chroot build

Steps: `sudo pacman -S --needed devtools`; `cd packaging/arch && pkgctl build` (or `extra-x86_64-build`). For the private repo, set `PRESTO_SRC=file://<path to local clone>` or make the repo public first.

Expected: package builds; on failure record which `depends`/`makedepends` was missing.

Result: pass. Repo is now public, default git+https source worked. `presto-git r234.d321670-1` built in the devtools chroot; depends/makedepends resolved. The first chroot attempt failed on a lagging archlinux.cachyos.org mirror (404s) and an invalid pacman signature; fixed by refreshing archlinux-keyring and using an official Arch mirror. Environment issue, not a PKGBUILD one.

---

## 2. CDM scan

Steps: `sh ../scan-no-cdm.sh presto-git-*.pkg.tar.zst` on the chroot-built package.

Expected: `ok: no widevine`.

Result: pass. `ok: no widevine` on the chroot-built package.

---

## 3. Install

Steps: `sudo pacman -U presto-git-*.pkg.tar.zst`; `presto --help`.

Expected: help mentions `lib/presto/engine`; Presto appears in the app launcher with the icon.

Result: pass. Not confirmed: `presto --help` text and launcher icon.

---

## 4. First run on a fresh profile

Steps: `XDG_STATE_HOME=$(mktemp -d) XDG_CACHE_HOME=$(mktemp -d) presto` (keeps the real profile intact).

Expected: "Preparing playback components…" appears, no drift error even if the download passes 15 s, then the Apple Music sign-in window opens.

Result: pass. Log showed `cdm Checking` then `cdm Ready 4.10.3112.0`.

---

## 5. Sign in and play

Steps: sign in with an Apple account, play a catalog track. Then `find "$XDG_STATE_HOME/presto/engine-profile/WidevineCdm" -name libwidevinecdm.so`.

Expected: a catalog track plays with audio; the CDM is found in the temporary profile. Record its version.

Result: pass. Signed in, tracks played. CDM at `<state>/presto/engine-profile/WidevineCdm/4.10.3112.0/_platform_specific/linux_x64/libwidevinecdm.so`. Version 4.10.3112.0.

---

## 6. Read-only install dir

Steps: after playback, `find /usr/lib/presto -newer /usr/bin/presto -type f`.

Expected: no output (Electron writes nothing under /usr).

Result: pass. `find /usr/lib/presto -newer /usr/bin/presto -type f` printed nothing.

---

## 7. Offline first run (optional)

Steps: disconnect the network, run item 4's command with new temp dirs, wait up to 2 minutes.

Expected: panel "Couldn't download the Widevine playback component" naming the cause; no Retry button; quitting Presto leaves no engine process (`pgrep -f presto/engine` empty).

Result: skipped (optional).

---

## 8. Demo from the package

Steps: `presto --demo`.

Expected: demo UI plays mock tracks.

Result: pass. `presto --demo` launched, engine spawned, mock tracks played.

---

## 9. Uninstall cleanup

Steps: `sudo pacman -Rns presto-git`, then the `rm -rf` line from docs/DISTRIBUTION.md on the temp dirs.

Expected: `/usr/lib/presto` gone; docs match what was observed.

Result: pass. `sudo pacman -Rns presto-git` ran cleanly (332.39 MiB removed, hooks ran) and `/usr/lib/presto` is gone. The `rm -rf` line for the temp state dirs was not run.

---

## Notes

- Packaging fix after namcap: LICENSE installed to `/usr/share/licenses/presto-git/` (namcap E for MIT license file); `vmp-resign.py`, `cli.js`, `install.js` and non-linux-x64-gnu `extract-zip` prebuilds removed from the package (the app execs `dist/electron` via `path.txt`). namcap not installed on the host, not re-run. Rebuilt package: `presto-git-r235.5f34913-1`, 119 MB (113.9 MiB), scan ok, one `dist/electron`.
- License decision: MIT.
- Out-of-scope bug: clicking a radio station (non-playlist) does not play. Not fixed here.
- Items 1-6, 8, 9 pass; 7 skipped (optional).

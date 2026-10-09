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

Result: partial: makepkg (host) pass. Chroot build not yet run.

---

## 2. CDM scan

Steps: `sh ../scan-no-cdm.sh presto-git-*.pkg.tar.zst` on the chroot-built package.

Expected: `ok: no widevine`.

Result: partial: host-built package passes. Chroot-built package not yet checked.

---

## 3. Install

Steps: `sudo pacman -U presto-git-*.pkg.tar.zst`; `presto --help`.

Expected: help mentions `lib/presto/engine`; Presto appears in the app launcher with the icon.

Result:

---

## 4. First run on a fresh profile

Steps: `XDG_STATE_HOME=$(mktemp -d) XDG_CACHE_HOME=$(mktemp -d) presto` (keeps the real profile intact).

Expected: "Preparing playback components…" appears, no drift error even if the download passes 15 s, then the Apple Music sign-in window opens.

Result:

---

## 5. Sign in and play

Steps: sign in with an Apple account, play a catalog track. Then `find "$XDG_STATE_HOME/presto/engine-profile/WidevineCdm" -name libwidevinecdm.so`.

Expected: a catalog track plays with audio; the CDM is found in the temporary profile. Record its version.

Result:

---

## 6. Read-only install dir

Steps: after playback, `find /usr/lib/presto -newer /usr/bin/presto -type f`.

Expected: no output (Electron writes nothing under /usr).

Result:

---

## 7. Offline first run (optional)

Steps: disconnect the network, run item 4's command with new temp dirs, wait up to 2 minutes.

Expected: panel "Couldn't download the Widevine playback component" naming the cause; no Retry button; quitting Presto leaves no engine process (`pgrep -f presto/engine` empty).

Result:

---

## 8. Demo from the package

Steps: `presto --demo`.

Expected: demo UI plays mock tracks.

Result:

---

## 9. Uninstall cleanup

Steps: `sudo pacman -Rns presto-git`, then the `rm -rf` line from docs/DISTRIBUTION.md on the temp dirs.

Expected: `/usr/lib/presto` gone; docs match what was observed.

Result:

---

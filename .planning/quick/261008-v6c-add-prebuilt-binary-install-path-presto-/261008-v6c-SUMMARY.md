---
phase: quick-261008-v6c
plan: 01
completed: 2026-10-08
key-files:
  created: [.github/workflows/release.yml, packaging/pack-release.sh, packaging/arch-bin/PKGBUILD]
  modified: [.gitignore, README.md, docs/DISTRIBUTION.md]
---

# Quick 261008-v6c: presto-bin install path

Tag-triggered release workflow, shared `pack-release.sh`, and a `presto-bin` PKGBUILD that installs the same layout as `presto-git` without compiling.

## Commits

- 26a6da9 pack script, workflow, .gitignore (`/dist`)
- 98c3720 presto-bin PKGBUILD
- 956ff75 README and DISTRIBUTION docs

## Verification

- `sh -n` on pack script, workflow YAML parse (`tags == ['v*']`), `bash -n` on PKGBUILD, conflicts/makedepends greps: pass.
- No built tree existed, so `cargo build --release --locked` was run (1m11s), then `PACK_SKIP_BUILD=1 sh packaging/pack-release.sh 0.0.0-test`: tarball 138 MB, `sha256sum -c` OK, no widevine or vmp-resign entries. `dist/` removed afterwards.
- Not run: the GitHub workflow, `makepkg` on presto-bin (no release exists), full `npm ci` path.

## Deviations

- Per the caller, the "repo is private" comment and doc note were dropped (repo is now public).
- Added `/packaging/arch-bin/` build-output ignores to `.gitignore`.
- Did not touch crates/, engine/, ROADMAP.md or .planning/config.json.

## Notes

`sha256sums=('SKIP')` in presto-bin until the first release exists.

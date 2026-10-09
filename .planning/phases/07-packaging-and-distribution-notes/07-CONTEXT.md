# Phase 7: Packaging and Distribution Notes - Context

**Gathered:** 2026-10-08
**Status:** Ready for planning

<domain>
## Phase Boundary

Presto installs on Arch from a PKGBUILD with no Widevine CDM in any artifact, first run fetches the CDM and playback works, and `docs/DISTRIBUTION.md` records CDM licensing, Apple ToS and Flathub blockers. Requirements: PKG-01, PKG-02. Flatpak/AppImage packaging (PKG-03) is out of scope.

</domain>

<decisions>
## Implementation Decisions

### Engine install
- **D-01:** The PKGBUILD fetches castlabs ECS (`v44.5.1+wvcus`, from `engine/package.json`) at build time (`npm ci` in `build()`), and installs `engine/` with its Electron under `/usr/lib/presto/engine`. No CDM is fetched or shipped at build time.
- **D-02:** The Electron binary is redistributed in the package archive. Accepted for personal use; recorded in DISTRIBUTION.md.
- **D-03:** The default `engine_dir` resolves relative to the executable (`../lib/presto/engine`), falling back to `./engine` for dev checkouts. `--engine-dir` still overrides.
- **D-04:** `depends=()` is derived from `ldd` of the Electron binary and `presto` (researcher lists the libs).

### Package form
- **D-05:** One package, `presto-git`, built from GitHub main (`ianmcman/presto`). No tagged-release PKGBUILD.
- **D-06:** PKGBUILD lives at `packaging/arch/PKGBUILD`. It is not published to the AUR in this phase; publishing is a separate user decision tied to the ToS question.
- **D-07:** Ship a minimal `presto.desktop` (Audio/Music categories) and a simple hand-written SVG icon.
- **D-08:** Verification: `makepkg` in a clean chroot, a scripted scan of the package file list asserting no `libwidevinecdm`, then a manual first-run checklist (fresh profile, sign in, play), in the style of the Phase 6 checklist.

### First-run CDM
- **D-09:** The engine reports CDM state via a new additive IPC event with a proto minor bump. PROTOCOL.md, the schema snapshot and the token-guard test are updated.
- **D-10:** The app shows status text ("Preparing playback components...") in the existing signed-out/loading UI while the CDM downloads.
- **D-11:** On download failure the UI shows an error naming the cause. Retry happens on next launch; no in-app Retry button.
- **D-12:** No CDM version pin or check. Record the observed version (4.10.3112.0 so far) in docs.

### Distribution doc
- **D-13:** `docs/DISTRIBUTION.md`, linked from README.
- **D-14:** Cites sources for CDM licensing (Widevine, castlabs), Apple Media Services terms and Flathub policy. States known vs unknown; draws no legal conclusion.
- **D-15:** Short feasibility paragraph each for Flatpak and AppImage. No packaging work.
- **D-16:** Includes an end-user section: install, first-run CDM wait, sign-in, data/profile paths, uninstall cleanup.

### Claude's Discretion
- Exact `depends=()` list, PKGBUILD function layout, icon design.
- Name and payload of the CDM IPC event (additive, token-free).
- Chroot tooling (`devtools` vs plain `makepkg`) and scan script form.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

- `.planning/ROADMAP.md` Phase 7 - goal and success criteria
- `.planning/REQUIREMENTS.md` - PKG-01, PKG-02, PKG-03 (deferred)
- `.planning/phases/02-engine-feasibility-spike-gate/SPIKE-REPORT.md` - CDM first-launch download (line ~16, ~107), distribution notes per engine
- `CLAUDE.md` (Technology Stack, "Widevine CDM: sourcing and licensing") - licensing notes and sources
- `docs/PROTOCOL.md` - IPC doc to extend with the CDM event
- `.planning/phases/06-desktop-integration/06-05-PLAN.md` - manual checklist format to mirror
- `engine/package.json` - ECS pin
- `crates/presto-core/src/config.rs` (`Launch::electron`), `crates/presto/src/cli.rs` (`engine_dir` default), `crates/presto/src/launch.rs` - engine path resolution

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `Launch::electron(engine_dir, extra)`: already builds the real-binary path from `node_modules/electron/path.txt`.
- Schema snapshot and token-guard tests in `presto-ipc`: extend for the new event.

### Established Patterns
- `--engine-dir` defaults to relative `engine`; Phase 6 added a `--help`-visible clap CLI.
- Manual verification checklists live beside phase plans.

### Integration Points
- Engine `main.js` emits the CDM event; supervisor/backend forwards it to UI state; signed-out/loading view renders it.
- No `packaging/`, `.desktop` or icon files exist yet.

</code_context>

<specifics>
## Specific Ideas

No specific requirements beyond the decisions above.

</specifics>

<deferred>
## Deferred Ideas

- Publishing to the AUR (needs user go-ahead and ToS review)
- Tagged-release PKGBUILD
- In-app CDM Retry button
- Flatpak and AppImage packaging (PKG-03)

</deferred>

---

*Phase: 07-packaging-and-distribution-notes*
*Context gathered: 2026-10-08*

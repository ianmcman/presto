# Milestones

## v1.0 Linux Apple Music client (Shipped: 2026-10-09)

**Phases completed:** 7 phases, 56 plans
**Timeline:** 2026-10-07 to 2026-10-09, 249 commits, about 17.9k lines of Rust and 0.7k of JavaScript

**Key accomplishments:**

- Versioned, token-free JSON IPC contract (proto 1.2) with a fault-injecting mock engine
- Engine feasibility gate passed: castlabs ECS v44.5.1+wvcus plays full tracks with persistent sign-in
- Supervised engine with crash and hang recovery, runtime-loaded bridge, queue mirror and re-auth
- SQLite page cache, artwork cache, search and offline serving
- egui UI with `presto --demo`, MPRIS, media keys and a CLI control socket
- Arch packages (`presto-git`, prebuilt `presto-bin`) with the Widevine CDM fetched at runtime, `docs/DISTRIBUTION.md`, and a GitHub release workflow (v0.1.0 published)

**Known gaps:**

- No milestone audit was run before archiving.
- Manual checklist item 7 (offline first run) was not run.
- The final PKGBUILD (r235) was not rebuilt on a second machine.
- Apple, Flathub and castlabs terms in `docs/DISTRIBUTION.md` were not re-fetched.
- AppImage and Flatpak packaging are documented as blocked or deferred, not built.

---

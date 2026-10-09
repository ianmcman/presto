# Phase 7: Packaging and Distribution Notes - Discussion Log

> Audit trail only. Decisions are in 07-CONTEXT.md.

**Date:** 2026-10-08
**Areas discussed:** Engine install in PKGBUILD, Package form and layout, First-run CDM experience, Blockers doc scope

All recommended options were selected:
- ECS fetched at build time; engine at /usr/lib/presto/engine via exe-relative lookup; Electron binary redistributed; deps from ldd.
- presto-git only; packaging/arch/PKGBUILD, unpublished; minimal .desktop + SVG; chroot build + widevine scan + manual checklist.
- CDM status via additive IPC event; status text in loading UI; error with retry on next launch; no version pin.
- docs/DISTRIBUTION.md; sourced, no legal conclusion; Flatpak/AppImage paragraphs; end-user section.

Alternatives not chosen: first-run ECS download, separate ECS package, compile-time env var, wrapper script, tagged-release PKGBUILD, AUR publish now, in-app Retry, README-only doc.

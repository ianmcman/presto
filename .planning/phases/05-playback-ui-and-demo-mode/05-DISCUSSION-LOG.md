# Phase 5: Playback UI and Demo Mode - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md.

**Date:** 2026-10-08
**Phase:** 5 - Playback UI and Demo Mode
**Areas discussed:** Port scope and plan split, Player bar and queue, Detail pages and unavailable tracks, Demo mode behavior

---

## Port scope and plan split

| Question | Chosen | Alternatives |
|---|---|---|
| Views ported | Core + everything portable | Core browse only (recommended); Minimal vertical slice |
| Plan split | One phase, wave-split plans | Split 5 / 5.1; You decide |
| Theme / i18n | fastframe theme, English only | Full i18n catalogs; Minimal theme |
| Crate layout | crates/presto, Backend wraps presto-core | UI talks to core directly |

**Notes:** "Everything portable" is bounded in CONTEXT.md D-01 to exclude v2 features.

## Player bar and queue

| Question | Chosen | Alternatives |
|---|---|---|
| Bar layout | Bottom bar, spotifast layout | Apple Music style |
| Queue UI | Right side panel | Full page |
| Queue edits | Play-from-here + remove + Play Next/Add | Play-from-here only; drag reorder |
| Shortcuts | Port spotifast's map as-is | Space+arrows; custom set |

## Detail pages and unavailable tracks

| Question | Chosen | Alternatives |
|---|---|---|
| Album/playlist | Hero + Play/Shuffle + track list | Plus Add to Library (v2) |
| Artist | Top songs + albums + singles shelves | Full discography; plus Similar/Appears On |
| Unavailable look | Greyed row + badge + tooltip | Greyed only; hide (conflicts PLAY-04) |
| Runtime play error | Toast + mark unavailable + auto-skip | Toast and stop; retry once |

## Demo mode behavior

| Question | Chosen | Alternatives |
|---|---|---|
| Start | Spawn presto-engine-mock | In-process mock |
| Visibility | DEMO chip | None |
| Data | Extend mock for every view | Current catalog as-is |
| Faults | CLI flag passthrough | Debug menu; both |

## Claude's Discretion

Module layout, toast/badge styling, which portable extras qualify.

## Deferred Ideas

Drag reorder, in-app fault menu.

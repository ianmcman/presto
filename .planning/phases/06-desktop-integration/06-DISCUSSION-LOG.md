# Phase 6: Desktop Integration - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md.

**Date:** 2026-10-08
**Phase:** 6-Desktop Integration
**Areas discussed:** CLI to running app, Window close vs media keys, MPRIS surface, CLI command set and status --json

## CLI to running app
| Question | Chosen | Alternative |
|---|---|---|
| Transport | Own control socket | MPRIS over D-Bus |
| No instance | Error, exit 1 | Launch app; launch only for play |
| Second `presto` launch | Raise existing window | Allow second instance |
| CLI shape | Clap subcommands, GUI default | Separate prestoctl binary |

## Window close vs media keys
| Question | Chosen | Alternative |
|---|---|---|
| Close window | User asked for hide-and-keep-playing plus tray; tray is TRAY-01 (v2), so deferred. Final: close quits | Hide and keep playing; setting |
| Idle close | N/A after deferral (was: quit if not playing) | Always hidden |
| Media keys | MPRIS only | Also global shortcuts portal |
| Multiple players | Plain well-behaved player | Pause others |

## MPRIS surface
| Question | Chosen | Alternative |
|---|---|---|
| Properties | Full player | Minimal |
| artUrl | file:// cached image | Remote mzstatic URL |
| Not-ready state | Always registered, Stopped | Register when Ready |
| Crate | Claude's discretion | fastframe-now-playing; mpris-server + zbus |

## CLI command set and status --json
| Question | Chosen | Alternative |
|---|---|---|
| Commands | Transport + settings | Transport only; add queue/search |
| status --json | Playback + track + app state | Playback + track only |
| status text | One-line | Multi-line |
| Watch mode | Yes, NDJSON stream | No |

## Claude's Discretion
MPRIS crate, control socket frame schema, exit codes, argument parsing, stale socket cleanup.

## Deferred Ideas
Tray with hide-on-close (TRAY-01), global shortcuts portal, queue/search CLI.

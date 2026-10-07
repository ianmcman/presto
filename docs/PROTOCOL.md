# Presto engine IPC protocol

Source of truth for the types is `crates/presto-ipc`. This document is hand-written and a test fails if a wire name is missing here.

## Overview

Two processes. `presto` (native UI) listens on a Unix socket and spawns the engine as a child; the engine connects. The engine is replaceable: castlabs Electron, Chrome over CDP, or the mock all speak this protocol.

Presto never holds Apple credentials. The engine adds authentication inside the web page. No frame carries a token, cookie or header.

## Transport

- Socket: `$XDG_RUNTIME_DIR/presto/engine.sock`. Parent directory mode 0700, socket mode 0600. A stale socket file is removed on bind.
- Framing: NDJSON. One JSON object per `\n`-terminated UTF-8 line. Maximum line length is 4 MiB, so engines paginate large responses.
- Engine launch contract: argv `--socket <path> --profile <dir>`, or env `PRESTO_SOCKET` / `PRESTO_PROFILE`. argv wins over env.
- Presto spawns the engine as a child and terminates it on exit.
- Engine stdout and stderr are captured by presto into a log file under its state dir and tailed on failure. Never log API response bodies at info level.

## Handshake and versioning

The engine sends `hello` first. Presto replies with its own `hello`.

Protocol version is `1.0`. A minor mismatch is tolerated and new features are gated by capabilities. A major mismatch is a hard error with a message naming both versions; the side that detects it closes the connection (the mock exits with code 2).

| Capability | Meaning |
|---|---|
| `playback` | engine plays audio and accepts transport commands |
| `queue` | engine accepts `set_queue` and emits queue events |
| `api` | engine answers `req` frames |
| `mock` | engine accepts `mock` frames; real engines never list it |

Presto never sends `mock` frames unless the engine hello lists `mock`.

## Frames

Every line is an object with a `t` field.

| `t` | Direction | Fields |
|---|---|---|
| `hello` | both | `proto` (`major`, `minor`), `role` (`presto` or `engine`), `capabilities`, `engine` (optional name) |
| `cmd` | presto to engine | `id`, `cmd` |
| `req` | presto to engine | `id`, `req` |
| `res` | engine to presto | `id`, `outcome` |
| `evt` | engine to presto | `evt` |
| `ping` | presto to engine | `seq` |
| `pong` | engine to presto | `seq` |
| `mock` | presto to engine | `id`, `fault` |

```json
{"t":"hello","proto":{"major":1,"minor":0},"role":"engine","capabilities":["playback","queue","api","mock"],"engine":"mock"}
{"t":"cmd","id":1,"cmd":{"type":"play"}}
{"t":"req","id":2,"req":{"method":"get","path":"/v1/me/library/playlists","query":{"limit":"25"},"body":null}}
{"t":"res","id":2,"outcome":{"status":"ok","data":{"data":[]}}}
{"t":"evt","evt":{"type":"volume","volume":0.5}}
{"t":"ping","seq":7}
{"t":"pong","seq":7}
{"t":"mock","id":3,"fault":{"kind":"slow","delay_ms":3000}}
```

## Commands

Tag `type`.

| `type` | Fields |
|---|---|
| `play` | |
| `pause` | |
| `seek` | `ms` |
| `next` | |
| `prev` | |
| `set_volume` | `volume`, 0.0 to 1.0 |
| `set_shuffle` | `on` |
| `set_repeat` | `mode`: `off`, `one`, `all` |
| `set_queue` | `ids`, `start` |

Every `cmd` gets exactly one `res` with the same `id`. Events caused by a command are sent before its `res`.

## API requests

`req` fields: `method` (`get`, `post`, `put`, `patch`, `delete`), `path`, `query` (string map), `body` (any JSON or null). There is deliberately no headers field; the engine owns authentication.

The `res` `outcome` is tagged by `status`: `ok` with `data`, or `err` with `error`.

## Errors

`error` has `kind` and `message`. `kind` is tagged by `code`. The set is closed and maps to UI states.

| `code` | Fields | Used when |
|---|---|---|
| `timeout` | | the requester's timeout elapsed; synthesized locally, engines may also send it |
| `auth_expired` | | the session is no longer valid and sign-in is needed |
| `rate_limited` | `retry_after_ms` | upstream throttled the request |
| `not_found` | | the item or path does not exist |
| `unavailable` | | the engine or upstream is temporarily unreachable |
| `upstream` | `status` | upstream returned another HTTP error |
| `internal` | | engine bug or unexpected state |

## Events

Tag `type`.

| `type` | Fields |
|---|---|
| `playback_state` | `state`: `stopped`, `loading`, `playing`, `paused`, `ended`; `seq` |
| `progress` | `position_ms`, `duration_ms`, `seq` |
| `track_changed` | `item` (a queue item or null) |
| `queue_changed` | `rev`, `items`, `index` |
| `volume` | `volume` |
| `shuffle` | `on` |
| `repeat` | `mode` |
| `auth` | `state`: `signed_out`, `signing_in`, `signed_in`, `expired` |
| `error` | `error` |

`progress` is sent about every 500 ms while playing and on every state change; receivers interpolate between events.

`queue_changed` is a full snapshot. `rev` bumps on every queue or index change, including natural advance. Receivers drop a `rev` lower than the last applied.

`seq` bumps on each user-initiated change. Receivers drop `progress` with an older `seq`.

Queue item fields: `id`, `title`, `artist`, `album`, `duration_ms`, `artwork_url`, `playable`.

## Timeouts and heartbeat

| Kind | Timeout |
|---|---|
| hello | 5 s |
| command | 5 s |
| set_queue | 15 s |
| API GET request | 20 s |
| API write request | 30 s |
| mock control | 5 s |

Timeouts are enforced by the requester and never appear on the wire.

Presto sends `ping` every 2 s; the engine answers `pong` with the same `seq`. Three unanswered pings (6 s) mean the engine is hung. Pongs must never wait behind slow request handling.

## Mock engine control

The mock engine accepts a `mock` frame with `fault`, tagged by `kind`: `none`, `hang`, `crash` (`after_ms`), `auth_expired`, `slow` (`delay_ms`).

CLI: `--fault`, repeatable, syntax `none|hang|crash|crash@<ms>|auth_expired|slow|slow=<ms>`. `slow` defaults to 3000 ms. Startup faults take effect right after the handshake.

- `hang`: drops every outbound frame (replies, events, pongs) while still reading. The socket stays open. Only a `mock` `none` frame clears it.
- `crash`: exits with status 101, immediately or after `after_ms`.
- `auth_expired`: emits `auth` `expired`. Every `cmd` and `req` then gets `err` `auth_expired` until cleared. `none` then emits `auth` `signed_in`.
- `slow`: delays every `res` by `delay_ms`; `evt` and `pong` stay on time.

`mock` frames are acked with `res` immediately and are never delayed.

## Security

No IPC type may carry an Apple credential field. `crates/presto-ipc/tests/schema.rs` fails on any name with a segment of `token`, `auth`, `authorization`, `bearer`, `jwt`, `secret`, `cookie`, `password` or `credential`. The allowlist is `auth` and `auth_expired`, which are state names.

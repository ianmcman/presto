# Presto

A lightweight native Apple Music client for Linux. The UI is Rust and egui. Playback and library access run in a hidden Widevine-capable Chromium engine (`presto-engine`) that talks to the app over a Unix socket with JSON lines.

Personal project, work in progress. Phase 5 (playback UI and demo mode) is nearly done.

## Try the demo

The demo runs against a mock engine, so no Apple account is needed.

```
cargo build -p presto-engine-mock
cargo run -p presto -- --demo
```

## Layout

- `crates/presto`: egui app
- `crates/presto-core`: engine supervisor, IPC, data cache
- `crates/presto-engine-mock`: mock engine for demo mode and tests
- `docs/PROTOCOL.md`: IPC protocol
- `.planning/`: roadmap and phase plans

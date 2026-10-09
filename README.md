# Presto

A lightweight native Apple Music client for Linux. The UI is Rust and egui. Playback and library access run in a hidden Widevine-capable Chromium engine (`presto-engine`) that talks to the app over a Unix socket with JSON lines.

Personal project, work in progress.

## Try the demo

The demo runs against a mock engine, so no Apple account is needed.

```
cargo build -p presto-engine-mock
cargo run -p presto -- --demo
```

## Install (Arch)

Build with `packaging/arch/PKGBUILD` (`makepkg -si`).
See [docs/DISTRIBUTION.md](docs/DISTRIBUTION.md) for first-run CDM download, data paths, uninstall and distribution blockers.

## Layout

- `crates/presto`: egui app
- `crates/presto-core`: engine supervisor, IPC, data cache
- `crates/presto-engine-mock`: mock engine for demo mode and tests
- `docs/PROTOCOL.md`: IPC protocol
- `docs/DISTRIBUTION.md`: packaging and distribution notes
- `.planning/`: roadmap and phase plans

# Presto

A lightweight native Apple Music client for Linux. The UI is Rust and egui. Playback and library access run in a hidden Widevine-capable Chromium engine (`presto-engine`) that talks to the app over a Unix socket with JSON lines.

Personal project, work in progress.

## Try the demo

The demo runs against a mock engine, so no Apple account is needed.

```
cargo build -p presto-engine-mock
cargo run -p presto -- --demo
```

## Install

Requires x86_64 Linux and an Apple Music subscription. No Apple developer account is needed. Packages target Arch and derivatives (CachyOS, EndeavourOS, Manjaro). Other distros: see [Run from a checkout](#run-from-a-checkout).

### Prebuilt (no compile)

```
git clone https://github.com/ianmcman/presto
cd presto/packaging/arch-bin
makepkg -si
```

This downloads the release tarball (about 138 MB), checks its sha256 and installs `presto` under `/usr`. It replaces `presto-git` if that is installed.

### From source (about 9 minutes)

```
git clone https://github.com/ianmcman/presto
cd presto/packaging/arch
makepkg -si
```

Build dependencies: `rust`, `git`, `nodejs`, `npm`. `makepkg -si` installs them.

### First run

```
presto
```

1. A "Preparing playback components" screen appears while the Widevine CDM downloads into your profile. No CDM ships in any package. This takes a few seconds on a normal connection.
2. The Apple Music sign-in window opens. Sign in as you would on music.apple.com.
3. The library loads. Later launches skip the download and the sign-in.

If the CDM download fails (offline, blocked network), Presto shows the cause. Quit and start it again to retry.

To try it without an account: `presto --demo`.

### Control from a shell

While Presto is running, `presto play`, `pause`, `toggle`, `next`, `prev`, `stop`, `seek`, `volume`, `shuffle`, `repeat`, `status`, `raise` and `quit` control it. MPRIS media keys also work.

### Update

Rebuild from the latest `packaging/arch-bin` or `packaging/arch` checkout and run `makepkg -si` again. Your sign-in and cache survive.

### Uninstall

```
sudo pacman -Rns presto-bin   # or presto-git
rm -rf ~/.local/state/presto ~/.cache/presto ~/.config/presto
```

The second line deletes your sign-in, the downloaded CDM and the cache.

### Run from a checkout

Works on any x86_64 Linux with Rust, Node 22 and the GTK3, Wayland or X11 development libraries:

```
cd engine && npm ci && cd ..
cargo run --release -p presto
```

Add `XDG_STATE_HOME=$(mktemp -d) XDG_CACHE_HOME=$(mktemp -d)` in front to use a throwaway profile.

### Troubleshooting

- Logs are in `~/.local/state/presto/logs/`. Include the newest `engine-*.log` when reporting a problem.
- `presto --help` shows `--engine-dir`, which points Presto at a different engine directory.
- Data paths, the CDM location, licensing and distribution blockers are in [docs/DISTRIBUTION.md](docs/DISTRIBUTION.md).

## Layout

- `crates/presto`: egui app
- `crates/presto-core`: engine supervisor, IPC, data cache
- `crates/presto-engine-mock`: mock engine for demo mode and tests
- `docs/PROTOCOL.md`: IPC protocol
- `docs/DISTRIBUTION.md`: packaging and distribution notes
- `.planning/`: roadmap and phase plans

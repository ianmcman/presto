# Spotifast seams

## Pinned source

crmne/spotifast `995c768dcba4da7f93302bf2107af6b7ec3df02b`, read 2026-10-07. All line numbers below are at this SHA.

## Seams

| Seam | Location | What Presto does with it |
|---|---|---|
| `Backend` struct | `src/backend.rs:876` | Keep the shape (UI holds one handle, sends commands, polls events). Behind it, talk to the engine over presto-ipc instead of librespot and the Spotify API. |
| `Backend::send` | `src/backend.rs:1004` | Same entry point. Spotifast has an offline allow-list that makes `--demo` work without a network; Presto's demo is the mock engine, so the allow-list goes away. |
| `Backend::poll` | `src/backend.rs:1222` | Drain engine `evt` and `res` frames into `Vec<Event>` each frame. |
| `Command` | `src/backend.rs:564` | `Player(..)` maps to `cmd`; `Api(..)` maps to `req`. Spotify-coupled variants are dropped (see below). |
| `Event` | `src/backend.rs:749` | `Local(LocalState)` is rebuilt from `evt`; `Api(ApiResponse)` from `res`; `Auth` from `evt` `auth`. |
| `ApiRequest` / `ApiResponse` | `src/backend.rs:108`, `src/backend.rs:338` | The `generation` field routes responses to the view that asked. Keep it as UI-side bookkeeping keyed by IPC `id`. |
| `AuthStatus` | `src/backend.rs:78` | Map to `AuthState` (`signed_out`, `signing_in`, `signed_in`, `expired`). |
| `LocalPlayback` | `src/backend.rs:855` | Map from `playback_state`. |
| `RemoteAction` | `src/backend.rs:88` | Media-key actions; wire to MPRIS (Phase 6). |
| `PlayerCommand` | `src/player.rs:280` | Source for the `cmd` set: Toggle, Next, Previous, Seek, Volume, Shuffle, Repeat, Load. |
| `LocalState` | `src/player.rs:165` | `position_ms` (168), `position_at` (170), `seek_sequence` (181), `track_sequence` (187). The UI interpolates from `position_at`; Presto does the same from `progress` events. |
| API client | `src/api/` | Replaced by proxied `req` frames. |
| HTTP, limiter, session reads | `src/http.rs`, `src/limiter.rs`, `src/session_reads.rs` | Dropped. The engine makes the calls and reports `rate_limited`. |
| Demo data | `src/demo.rs` | Reference for what the mock engine should serve. |
| Module list | `src/lib.rs:3-54` | Port views from `ui`, `app`, `model`; skip `auth`, `credentials`, `zeroconf`, `milkdrop`. |

Dropped as Spotify-coupled: `SignIn`, `CancelSignIn`, `CredentialsRestored`, `ProxyRestored`, `ApplyProxy`, `AuthorizePlayback`, `RestartEngine`, `Receivers`, `ReceiverActivated`, `Transfer`.

## Mapping to presto-ipc

| Spotifast | presto-ipc |
|---|---|
| `PlayerCommand::Toggle` | `cmd` `play` or `pause` by current state |
| `Next`, `Previous` | `next`, `prev` |
| `Seek(u32)` | `seek` `ms` |
| `Volume(u16)` | `set_volume` `volume` (0.0 to 1.0) |
| `Shuffle(bool)`, `Repeat(mode)` | `set_shuffle`, `set_repeat` |
| `Load(LoadSpec)` | `set_queue` `ids`, `start` |
| `Command::Api(ApiRequest)` | `req` with `method`, `path`, `query`, `body` |
| `Event::Local(LocalState)` | `playback_state`, `progress`, `track_changed`, `queue_changed`, `volume`, `shuffle`, `repeat` |
| `Event::Auth(AuthStatus)` | `auth` |
| `Event::Error(String)` | `error` |
| `track_sequence`, `seek_sequence` | `seq` |

## Fork and fastframe resolution

Throwaway crate outside this repo (edition 2024, `rust-toolchain.toml` channel 1.98.0), deps: `eframe` 0.36 (default features off, `accesskit`, `glow`, `default_fonts`, `wayland`, `x11`) and `fastframe-{text,fonts,icons,theme,shell,instance,now-playing}` at tag `v0.4.1` from `https://github.com/crmne/fastframe`. The `[patch.crates-io]` block was copied verbatim from spotifast's `Cargo.toml` lines 232-267: ecolor, eframe, egui, egui-wgpu, egui-winit, egui_extras, egui_glow, emath, epaint, epaint_default_fonts at `https://github.com/crmne/egui` rev `ba6790fe7cf46e58e8d27ce1524cbfdee745e938`, and winit at `https://github.com/crmne/winit` rev `ed7caa9023f10b397f5b6ec8284a840cbd8a6f65`.

Commands: `cargo fetch`, `cargo tree -i egui --depth 0`, `cargo tree -i winit --depth 0`. No build was run. Toolchain: rustc 1.98.0 (88d9e12ae 2026-08-18), cargo 1.98.0. Date: 2026-10-07.

| Item | Result |
|---|---|
| egui fork rev `ba6790fe` | pass: `egui v0.36.1 (crmne/egui?rev=ba6790fe...)` |
| winit fork | pass: `winit v0.30.13 (crmne/winit?rev=ed7caa90...)` |
| `fastframe-text` v0.4.1 | pass |
| `fastframe-fonts` v0.4.1 | pass |
| `fastframe-icons` v0.4.1 | pass |
| `fastframe-theme` v0.4.1 | pass |
| `fastframe-shell` v0.4.1 | pass |
| `fastframe-instance` v0.4.1 | pass |
| `fastframe-now-playing` v0.4.1 | pass |

`cargo fetch` warned that the `egui_extras` patch was unused, because the throwaway crate does not depend on it. Presto's UI will.

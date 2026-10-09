//! MPRIS media controls and now-playing from a tokio task.

use crate::control::{Control, Position};
use crate::status::mpris_state;
use presto_core::state::CoreState;
use presto_ipc::ctl::CtlOp;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::time::interval;

pub use fastframe_now_playing as np;

/// Map an MPRIS command to a control operation.
/// Returns None if the command should be dropped (not ready state handled by caller).
pub fn command_op(c: &np::Command, s: &CoreState) -> Option<CtlOp> {
    match c {
        np::Command::PlayPause => Some(CtlOp::Toggle),
        np::Command::Play => Some(CtlOp::Play),
        np::Command::Pause => Some(CtlOp::Pause),
        np::Command::Stop => Some(CtlOp::Stop),
        np::Command::Next => Some(CtlOp::Next),
        np::Command::Previous => Some(CtlOp::Prev),
        np::Command::SeekBy(ms) => Some(CtlOp::SeekBy { ms: *ms }),
        np::Command::SetPosition { track_id, position } => {
            // Only seek if the track_id matches the current track
            if s.player.track.as_ref().map(|t| &t.id) == Some(track_id) {
                Some(CtlOp::Seek {
                    ms: position.as_millis() as u64,
                })
            } else {
                None
            }
        }
        np::Command::SetVolume(f) => {
            // Clamp to [0, 1] and convert to percent
            let clamped = f.clamp(0.0, 1.0);
            let pct = (clamped * 100.0).round() as u8;
            Some(CtlOp::Volume { pct })
        }
        np::Command::SetShuffle(on) => Some(CtlOp::Shuffle { on: Some(*on) }),
        np::Command::SetRepeat(r) => {
            use presto_ipc::RepeatMode;
            let mode = match r {
                np::Repeat::Off => RepeatMode::Off,
                np::Repeat::Track => RepeatMode::One,
                np::Repeat::Playlist => RepeatMode::All,
            };
            Some(CtlOp::Repeat { mode: Some(mode) })
        }
        np::Command::Raise => Some(CtlOp::Raise),
        np::Command::Quit => Some(CtlOp::Quit),
        np::Command::OpenUri(_) => None,
    }
}

/// Get the artwork file path for the current track, if available.
pub fn art_for(s: &CoreState, ctl: &dyn Control) -> Option<PathBuf> {
    let url = s.player.track.as_ref()?.artwork_url.as_deref()?;
    ctl.art(url)
}

/// Create an MPRIS app descriptor for the named mode.
pub fn app_for(demo: bool) -> np::App {
    if demo {
        np::App::new("presto-demo", "Presto (Demo)")
    } else {
        np::App::new("presto", "Presto")
    }
}

/// Start the MPRIS service pump. Runs on a tokio task spawned by the backend.
/// The pump wakes on state changes, MPRIS commands, and a 1s interval.
pub fn start(backend: &crate::backend::Backend, ctl: Arc<dyn Control>, app: np::App) {
    let wake = Arc::new(Notify::new());
    let wake_clone = wake.clone();

    let mut np = np::NowPlaying::start(app, move || {
        wake_clone.notify_one();
    });

    // Send the initial state with all controls disabled while waiting for readiness
    np.update(np::State {
        playback: np::Playback::Stopped,
        track: None,
        position: Duration::ZERO,
        volume: None,
        shuffle: None,
        repeat: None,
        controls: np::Controls {
            play: false,
            pause: false,
            next: false,
            previous: false,
            seek: false,
        },
    });

    let mut rx = backend.state();
    let pos = Position::new();

    backend.spawn(async move {
        let mut tick = interval(Duration::from_secs(1));

        loop {
            tokio::select! {
                r = rx.changed() => {
                    if r.is_err() {
                        break;
                    }
                }
                _ = wake.notified() => {}
                _ = tick.tick() => {}
            }

            let s = rx.borrow().clone();

            // Process all pending MPRIS commands
            for c in np.commands() {
                if let Some(op) = command_op(&c, &s) {
                    match crate::control::plan(&op, &s, pos.now(&s)) {
                        Ok(acts) => {
                            // For seek operations, notify MPRIS of the target position
                            for act in &acts {
                                if let crate::control::Act::Cmd(
                                    presto_ipc::Command::Seek { ms },
                                ) = act
                                {
                                    np.seeked(Duration::from_millis(*ms));
                                }
                            }

                            crate::control::apply(&*ctl, acts);
                        }
                        Err(_) => {
                            // Not ready; drop the command silently (D-11)
                        }
                    }
                }
            }

            // Update MPRIS with the current state
            let mpris_st = mpris_state(&s, pos.now(&s), art_for(&s, &*ctl));
            np.update(mpris_st);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use presto_core::state::EngineStatus;
    use presto_ipc::{AuthState, RepeatMode, QueueItem};

    fn mk_state(
        engine: EngineStatus,
        auth: Option<AuthState>,
        player: presto_core::mirror::PlayerMirror,
    ) -> CoreState {
        let mut s = CoreState::default();
        s.engine = engine;
        s.auth = auth;
        s.player = player;
        s
    }

    fn item(id: &str) -> QueueItem {
        QueueItem {
            id: id.into(),
            title: "Title".into(),
            artist: "Artist".into(),
            album: "Album".into(),
            duration_ms: 200_000,
            artwork_url: None,
            playable: true,
        }
    }

    #[test]
    fn command_op_play_pause() {
        let s = mk_state(EngineStatus::Ready, Some(AuthState::SignedIn), Default::default());
        assert_eq!(
            command_op(&np::Command::PlayPause, &s),
            Some(CtlOp::Toggle)
        );
    }

    #[test]
    fn command_op_play() {
        let s = mk_state(EngineStatus::Ready, Some(AuthState::SignedIn), Default::default());
        assert_eq!(command_op(&np::Command::Play, &s), Some(CtlOp::Play));
    }

    #[test]
    fn command_op_pause() {
        let s = mk_state(EngineStatus::Ready, Some(AuthState::SignedIn), Default::default());
        assert_eq!(command_op(&np::Command::Pause, &s), Some(CtlOp::Pause));
    }

    #[test]
    fn command_op_stop() {
        let s = mk_state(EngineStatus::Ready, Some(AuthState::SignedIn), Default::default());
        assert_eq!(command_op(&np::Command::Stop, &s), Some(CtlOp::Stop));
    }

    #[test]
    fn command_op_next() {
        let s = mk_state(EngineStatus::Ready, Some(AuthState::SignedIn), Default::default());
        assert_eq!(command_op(&np::Command::Next, &s), Some(CtlOp::Next));
    }

    #[test]
    fn command_op_previous() {
        let s = mk_state(EngineStatus::Ready, Some(AuthState::SignedIn), Default::default());
        assert_eq!(command_op(&np::Command::Previous, &s), Some(CtlOp::Prev));
    }

    #[test]
    fn command_op_seek_by() {
        let s = mk_state(EngineStatus::Ready, Some(AuthState::SignedIn), Default::default());
        assert_eq!(
            command_op(&np::Command::SeekBy(-5000), &s),
            Some(CtlOp::SeekBy { ms: -5000 })
        );
    }

    #[test]
    fn command_op_set_position_matching_track() {
        let mut player: presto_core::mirror::PlayerMirror = Default::default();
        player.track = Some(item("track-1"));
        let s = mk_state(EngineStatus::Ready, Some(AuthState::SignedIn), player);
        assert_eq!(
            command_op(
                &np::Command::SetPosition {
                    track_id: "track-1".into(),
                    position: Duration::from_secs(90)
                },
                &s
            ),
            Some(CtlOp::Seek { ms: 90_000 })
        );
    }

    #[test]
    fn command_op_set_position_stale_track() {
        let mut player: presto_core::mirror::PlayerMirror = Default::default();
        player.track = Some(item("track-1"));
        let s = mk_state(EngineStatus::Ready, Some(AuthState::SignedIn), player);
        assert_eq!(
            command_op(
                &np::Command::SetPosition {
                    track_id: "track-2".into(),
                    position: Duration::from_secs(90)
                },
                &s
            ),
            None
        );
    }

    #[test]
    fn command_op_set_volume_clamp() {
        let s = mk_state(EngineStatus::Ready, Some(AuthState::SignedIn), Default::default());
        assert_eq!(
            command_op(&np::Command::SetVolume(0.5), &s),
            Some(CtlOp::Volume { pct: 50 })
        );
        assert_eq!(
            command_op(&np::Command::SetVolume(1.7), &s),
            Some(CtlOp::Volume { pct: 100 })
        );
        assert_eq!(
            command_op(&np::Command::SetVolume(-1.0), &s),
            Some(CtlOp::Volume { pct: 0 })
        );
    }

    #[test]
    fn command_op_set_shuffle() {
        let s = mk_state(EngineStatus::Ready, Some(AuthState::SignedIn), Default::default());
        assert_eq!(
            command_op(&np::Command::SetShuffle(true), &s),
            Some(CtlOp::Shuffle { on: Some(true) })
        );
    }

    #[test]
    fn command_op_set_repeat() {
        let s = mk_state(EngineStatus::Ready, Some(AuthState::SignedIn), Default::default());
        assert_eq!(
            command_op(&np::Command::SetRepeat(np::Repeat::Track), &s),
            Some(CtlOp::Repeat {
                mode: Some(RepeatMode::One)
            })
        );
        assert_eq!(
            command_op(&np::Command::SetRepeat(np::Repeat::Playlist), &s),
            Some(CtlOp::Repeat {
                mode: Some(RepeatMode::All)
            })
        );
        assert_eq!(
            command_op(&np::Command::SetRepeat(np::Repeat::Off), &s),
            Some(CtlOp::Repeat {
                mode: Some(RepeatMode::Off)
            })
        );
    }

    #[test]
    fn command_op_raise() {
        let s = mk_state(EngineStatus::Ready, Some(AuthState::SignedIn), Default::default());
        assert_eq!(command_op(&np::Command::Raise, &s), Some(CtlOp::Raise));
    }

    #[test]
    fn command_op_quit() {
        let s = mk_state(EngineStatus::Ready, Some(AuthState::SignedIn), Default::default());
        assert_eq!(command_op(&np::Command::Quit, &s), Some(CtlOp::Quit));
    }

    #[test]
    fn command_op_open_uri() {
        let s = mk_state(EngineStatus::Ready, Some(AuthState::SignedIn), Default::default());
        assert_eq!(
            command_op(&np::Command::OpenUri("spotify:...".into()), &s),
            None
        );
    }

    #[test]
    fn app_for_demo() {
        let app = app_for(true);
        assert_eq!(app.bus_name, "presto-demo");
        assert_eq!(app.identity, "Presto (Demo)");
    }

    #[test]
    fn app_for_real() {
        let app = app_for(false);
        assert_eq!(app.bus_name, "presto");
        assert_eq!(app.identity, "Presto");
    }
}

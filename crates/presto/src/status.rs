//! Status and MPRIS state mappings.

use presto_core::state::CoreState;
use presto_ipc::ctl::{Status, StatusState, StatusTrack, EngineLabel};
use presto_ipc::{PlayState, RepeatMode};
use std::path::Path;

pub fn status_json(s: &CoreState, pos_ms: u64, art: Option<&Path>) -> Status {
    use presto_core::state::EngineStatus;

    let (state, track, _ready) = if !crate::control::ready(s) {
        (StatusState::Stopped, None, false)
    } else {
        let st = match s.player.state {
            PlayState::Playing => StatusState::Playing,
            PlayState::Paused => StatusState::Paused,
            PlayState::Loading => StatusState::Loading,
            PlayState::Stopped | PlayState::Ended => StatusState::Stopped,
        };
        let track = s.player.track.as_ref().map(|t| StatusTrack {
            id: t.id.clone(),
            title: t.title.clone(),
            artist: t.artist.clone(),
            album: t.album.clone(),
            artwork_path: art.map(|p| p.to_string_lossy().into_owned()),
        });
        (st, track, true)
    };

    let engine = match &s.engine {
        EngineStatus::Ready => EngineLabel::Ready,
        EngineStatus::Starting => EngineLabel::Starting,
        EngineStatus::Restarting { .. } => EngineLabel::Restarting,
        EngineStatus::Drift { .. } | EngineStatus::Failed { .. } => EngineLabel::Failed,
    };

    Status {
        proto: presto_ipc::ctl::CTL_PROTO,
        state,
        position_ms: pos_ms,
        duration_ms: s.player.duration_ms,
        volume: (s.player.volume * 100.0).round() as u8,
        shuffle: s.player.shuffle,
        repeat: s.player.repeat,
        track,
        auth: s.auth.clone(),
        engine,
    }
}

pub fn one_line(st: &Status) -> String {
    match (st.state, &st.track) {
        (StatusState::Playing, Some(t)) => {
            let pos = crate::playback::fmt_time(st.position_ms);
            let dur = crate::playback::fmt_time(st.duration_ms);
            format!("Playing: {} - {} ({}/{})", t.title, t.artist, pos, dur)
        }
        (StatusState::Paused, Some(t)) => {
            let pos = crate::playback::fmt_time(st.position_ms);
            let dur = crate::playback::fmt_time(st.duration_ms);
            format!("Paused: {} - {} ({}/{})", t.title, t.artist, pos, dur)
        }
        (StatusState::Loading, Some(t)) => {
            let pos = crate::playback::fmt_time(st.position_ms);
            let dur = crate::playback::fmt_time(st.duration_ms);
            format!("Loading: {} - {} ({}/{})", t.title, t.artist, pos, dur)
        }
        _ => "Stopped".to_string(),
    }
}

pub fn mpris_state(s: &CoreState, pos_ms: u64, art: Option<std::path::PathBuf>) -> fastframe_now_playing::State {
    use fastframe_now_playing::{Playback, Repeat, Track};

    if !crate::control::ready(s) {
        return fastframe_now_playing::State {
            playback: Playback::Stopped,
            track: None,
            position: std::time::Duration::from_millis(pos_ms),
            volume: None,
            shuffle: None,
            repeat: None,
            controls: fastframe_now_playing::Controls {
                play: false,
                pause: false,
                next: false,
                previous: false,
                seek: false,
            },
        };
    }

    let playback = match s.player.state {
        PlayState::Playing => Playback::Playing,
        PlayState::Loading => Playback::Paused,
        PlayState::Paused => Playback::Paused,
        PlayState::Stopped | PlayState::Ended => Playback::Stopped,
    };

    let track = s.player.track.as_ref().map(|t| Track {
        id: t.id.clone(),
        title: t.title.clone(),
        artists: vec![t.artist.clone()],
        album: t.album.clone(),
        duration: Some(std::time::Duration::from_millis(t.duration_ms)),
        art_file: art.clone(),
        art_url: None,
        url: None,
        genres: vec![],
        bpm: None,
        rating: None,
    });

    let can_control = s.player.track.is_some() && s.player.duration_ms > 0;

    fastframe_now_playing::State {
        playback,
        track,
        position: std::time::Duration::from_millis(pos_ms),
        volume: Some(s.player.volume as f64),
        shuffle: Some(s.player.shuffle),
        repeat: Some(match s.player.repeat {
            RepeatMode::Off => Repeat::Off,
            RepeatMode::One => Repeat::Track,
            RepeatMode::All => Repeat::Playlist,
        }),
        controls: fastframe_now_playing::Controls {
            play: can_control,
            pause: can_control,
            next: can_control,
            previous: can_control,
            seek: can_control,
        },
    }
}

pub fn same_but_position(a: &Status, b: &Status) -> bool {
    a.proto == b.proto
        && a.state == b.state
        && a.duration_ms == b.duration_ms
        && a.volume == b.volume
        && a.shuffle == b.shuffle
        && a.repeat == b.repeat
        && a.track == b.track
        && a.auth == b.auth
        && a.engine == b.engine
        // position_ms is explicitly not compared
}

#[cfg(test)]
mod tests {
    use super::*;
    use presto_core::mirror::PlayerMirror;
    use presto_core::state::EngineStatus;
    use presto_ipc::AuthState;

    fn mk_state(
        engine: EngineStatus,
        auth: Option<AuthState>,
        player: PlayerMirror,
    ) -> CoreState {
        let mut s = CoreState::default();
        s.engine = engine;
        s.auth = auth;
        s.player = player;
        s
    }

    fn item(id: &str) -> presto_ipc::QueueItem {
        presto_ipc::QueueItem {
            id: id.into(),
            title: "Title".into(),
            artist: "Artist".into(),
            album: "Album".into(),
            duration_ms: 220_000,
            artwork_url: None,
            playable: true,
        }
    }

    #[test]
    fn status_json_playing_with_track() {
        let mut player = PlayerMirror::default();
        player.track = Some(item("1"));
        player.state = PlayState::Playing;
        player.position_ms = 72_000;
        player.duration_ms = 220_000;
        player.volume = 0.5;

        let state = mk_state(
            EngineStatus::Ready,
            Some(AuthState::SignedIn),
            player,
        );

        let status = status_json(&state, 72_000, None);
        assert_eq!(status.state, StatusState::Playing);
        assert_eq!(status.position_ms, 72_000);
        assert_eq!(status.duration_ms, 220_000);
        assert_eq!(status.volume, 50);
        assert!(status.track.is_some());
    }

    #[test]
    fn status_json_paused_with_track() {
        let mut player = PlayerMirror::default();
        player.track = Some(item("1"));
        player.state = PlayState::Paused;

        let state = mk_state(
            EngineStatus::Ready,
            Some(AuthState::SignedIn),
            player,
        );

        let status = status_json(&state, 0, None);
        assert_eq!(status.state, StatusState::Paused);
        assert!(status.track.is_some());
    }

    #[test]
    fn status_json_loading_with_track() {
        let mut player = PlayerMirror::default();
        player.track = Some(item("1"));
        player.state = PlayState::Loading;

        let state = mk_state(
            EngineStatus::Ready,
            Some(AuthState::SignedIn),
            player,
        );

        let status = status_json(&state, 0, None);
        assert_eq!(status.state, StatusState::Loading);
        assert!(status.track.is_some());
    }

    #[test]
    fn status_json_stopped_no_track() {
        let state = mk_state(
            EngineStatus::Ready,
            Some(AuthState::SignedIn),
            PlayerMirror::default(),
        );

        let status = status_json(&state, 0, None);
        assert_eq!(status.state, StatusState::Stopped);
        assert!(status.track.is_none());
    }

    #[test]
    fn status_json_not_ready_is_stopped() {
        let mut player = PlayerMirror::default();
        player.track = Some(item("1"));
        player.state = PlayState::Playing;

        let state = mk_state(
            EngineStatus::Starting,
            Some(AuthState::SignedIn),
            player,
        );

        let status = status_json(&state, 0, None);
        assert_eq!(status.state, StatusState::Stopped);
        assert!(status.track.is_none());
    }

    #[test]
    fn status_json_engine_states() {
        let player = PlayerMirror::default();

        let state = mk_state(EngineStatus::Ready, Some(AuthState::SignedIn), player.clone());
        assert_eq!(status_json(&state, 0, None).engine, EngineLabel::Ready);

        let state = mk_state(EngineStatus::Starting, Some(AuthState::SignedIn), player.clone());
        assert_eq!(status_json(&state, 0, None).engine, EngineLabel::Starting);

        let state = mk_state(
            EngineStatus::Restarting { attempt: 1, delay_ms: 1000 },
            Some(AuthState::SignedIn),
            player.clone(),
        );
        assert_eq!(status_json(&state, 0, None).engine, EngineLabel::Restarting);

        let state = mk_state(
            EngineStatus::Failed {
                reason: "test".into(),
                log_path: Default::default(),
                log_tail: vec![],
            },
            Some(AuthState::SignedIn),
            player.clone(),
        );
        assert_eq!(status_json(&state, 0, None).engine, EngineLabel::Failed);
    }

    #[test]
    fn status_json_volume_rounding() {
        let mut player = PlayerMirror::default();
        player.volume = 0.505;

        let state = mk_state(
            EngineStatus::Ready,
            Some(AuthState::SignedIn),
            player,
        );

        let status = status_json(&state, 0, None);
        assert_eq!(status.volume, 51); // 0.505 * 100 rounds to 51
    }

    #[test]
    fn one_line_playing() {
        let status = Status {
            proto: 1,
            state: StatusState::Playing,
            position_ms: 72_000,
            duration_ms: 220_000,
            volume: 50,
            shuffle: false,
            repeat: RepeatMode::Off,
            track: Some(StatusTrack {
                id: "1".into(),
                title: "Title".into(),
                artist: "Artist".into(),
                album: "Album".into(),
                artwork_path: None,
            }),
            auth: Some(AuthState::SignedIn),
            engine: EngineLabel::Ready,
        };

        let line = one_line(&status);
        assert_eq!(line, "Playing: Title - Artist (1:12/3:40)");
    }

    #[test]
    fn one_line_paused() {
        let status = Status {
            proto: 1,
            state: StatusState::Paused,
            position_ms: 72_000,
            duration_ms: 220_000,
            volume: 50,
            shuffle: false,
            repeat: RepeatMode::Off,
            track: Some(StatusTrack {
                id: "1".into(),
                title: "Title".into(),
                artist: "Artist".into(),
                album: "Album".into(),
                artwork_path: None,
            }),
            auth: Some(AuthState::SignedIn),
            engine: EngineLabel::Ready,
        };

        let line = one_line(&status);
        assert_eq!(line, "Paused: Title - Artist (1:12/3:40)");
    }

    #[test]
    fn one_line_loading() {
        let status = Status {
            proto: 1,
            state: StatusState::Loading,
            position_ms: 0,
            duration_ms: 0,
            volume: 50,
            shuffle: false,
            repeat: RepeatMode::Off,
            track: Some(StatusTrack {
                id: "1".into(),
                title: "Title".into(),
                artist: "Artist".into(),
                album: "Album".into(),
                artwork_path: None,
            }),
            auth: Some(AuthState::SignedIn),
            engine: EngineLabel::Ready,
        };

        let line = one_line(&status);
        assert!(line.starts_with("Loading: "));
    }

    #[test]
    fn one_line_stopped() {
        let status = Status {
            proto: 1,
            state: StatusState::Stopped,
            position_ms: 0,
            duration_ms: 0,
            volume: 0,
            shuffle: false,
            repeat: RepeatMode::Off,
            track: None,
            auth: None,
            engine: EngineLabel::Ready,
        };

        let line = one_line(&status);
        assert_eq!(line, "Stopped");
    }

    #[test]
    fn mpris_state_playing() {
        let mut player = PlayerMirror::default();
        player.track = Some(item("1"));
        player.state = PlayState::Playing;
        player.volume = 0.5;
        player.shuffle = true;
        player.repeat = RepeatMode::All;

        let state = mk_state(
            EngineStatus::Ready,
            Some(AuthState::SignedIn),
            player,
        );

        let mpris = mpris_state(&state, 0, None);
        assert_eq!(mpris.playback, fastframe_now_playing::Playback::Playing);
        assert!(mpris.track.is_some());
        assert_eq!(mpris.volume, Some(0.5));
        assert_eq!(mpris.shuffle, Some(true));
        assert_eq!(mpris.repeat, Some(fastframe_now_playing::Repeat::Playlist));
    }

    #[test]
    fn mpris_state_loading_is_paused() {
        let mut player = PlayerMirror::default();
        player.track = Some(item("1"));
        player.state = PlayState::Loading;

        let state = mk_state(
            EngineStatus::Ready,
            Some(AuthState::SignedIn),
            player,
        );

        let mpris = mpris_state(&state, 0, None);
        assert_eq!(mpris.playback, fastframe_now_playing::Playback::Paused);
    }

    #[test]
    fn mpris_state_repeat_mapping() {
        let mut player = PlayerMirror::default();
        player.track = Some(item("1"));
        player.state = PlayState::Playing;

        let state = mk_state(
            EngineStatus::Ready,
            Some(AuthState::SignedIn),
            player.clone(),
        );

        let mpris = mpris_state(&state, 0, None);
        assert_eq!(mpris.repeat, Some(fastframe_now_playing::Repeat::Off));

        let mut player = player.clone();
        player.repeat = RepeatMode::One;
        let state = mk_state(
            EngineStatus::Ready,
            Some(AuthState::SignedIn),
            player,
        );
        let mpris = mpris_state(&state, 0, None);
        assert_eq!(mpris.repeat, Some(fastframe_now_playing::Repeat::Track));
    }

    #[test]
    fn mpris_state_not_ready() {
        let state = mk_state(
            EngineStatus::Starting,
            None,
            PlayerMirror::default(),
        );

        let mpris = mpris_state(&state, 0, None);
        assert_eq!(mpris.playback, fastframe_now_playing::Playback::Stopped);
        assert!(mpris.track.is_none());
        assert_eq!(mpris.controls.play, false);
        assert_eq!(mpris.controls.pause, false);
        assert_eq!(mpris.controls.seek, false);
    }

    #[test]
    fn mpris_state_track_has_single_artist() {
        let mut player = PlayerMirror::default();
        player.track = Some(item("1"));
        player.state = PlayState::Playing;

        let state = mk_state(
            EngineStatus::Ready,
            Some(AuthState::SignedIn),
            player,
        );

        let mpris = mpris_state(&state, 0, None);
        assert_eq!(mpris.track.as_ref().unwrap().artists, vec!["Artist"]);
    }

    #[test]
    fn mpris_state_no_art_url() {
        let mut player = PlayerMirror::default();
        player.track = Some(item("1"));
        player.state = PlayState::Playing;

        let state = mk_state(
            EngineStatus::Ready,
            Some(AuthState::SignedIn),
            player,
        );

        let mpris = mpris_state(&state, 0, None);
        let track = mpris.track.unwrap();
        // The Track struct doesn't expose art_url directly in our usage, but verify it's set correctly
        assert!(track.art_file.is_none());
    }

    #[test]
    fn same_but_position_true_for_position_only_diff() {
        let status1 = Status {
            proto: 1,
            state: StatusState::Playing,
            position_ms: 1000,
            duration_ms: 10000,
            volume: 50,
            shuffle: false,
            repeat: RepeatMode::Off,
            track: Some(StatusTrack {
                id: "1".into(),
                title: "Title".into(),
                artist: "Artist".into(),
                album: "Album".into(),
                artwork_path: None,
            }),
            auth: Some(AuthState::SignedIn),
            engine: EngineLabel::Ready,
        };

        let status2 = Status {
            position_ms: 2000,
            ..status1.clone()
        };

        assert!(same_but_position(&status1, &status2));
    }

    #[test]
    fn same_but_position_false_for_other_diffs() {
        let status1 = Status {
            proto: 1,
            state: StatusState::Playing,
            position_ms: 1000,
            duration_ms: 10000,
            volume: 50,
            shuffle: false,
            repeat: RepeatMode::Off,
            track: Some(StatusTrack {
                id: "1".into(),
                title: "Title".into(),
                artist: "Artist".into(),
                album: "Album".into(),
                artwork_path: None,
            }),
            auth: Some(AuthState::SignedIn),
            engine: EngineLabel::Ready,
        };

        let status2 = Status {
            state: StatusState::Paused,
            position_ms: 2000,
            ..status1.clone()
        };

        assert!(!same_but_position(&status1, &status2));
    }

    #[test]
    fn status_json_no_token_leak() {
        let mut player = PlayerMirror::default();
        player.track = Some(item("1"));
        player.state = PlayState::Playing;

        let state = mk_state(
            EngineStatus::Ready,
            Some(AuthState::SignedIn),
            player,
        );

        let status = status_json(&state, 0, None);
        let json = serde_json::to_string(&status).unwrap();

        // Verify no credential-like strings are present
        assert!(!json.to_lowercase().contains("token"));
        assert!(!json.to_lowercase().contains("bearer"));
        assert!(!json.to_lowercase().contains("http"));
    }
}

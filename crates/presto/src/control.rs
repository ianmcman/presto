//! Control operations: mapping CtlOp to Commands, with guard traits and state helpers.

use presto_core::state::CoreState;
use presto_ipc::{AuthState, Command, PlayState, RepeatMode};
use crate::playback::{toggle_cmd, seek_by, volume_by, next_repeat, MIN_SEEK_MS, Clock};

pub trait Control: Send + Sync {
    fn send(&self, c: Command);
    fn raise(&self);
    fn quit(&self);
    fn art(&self, url: &str) -> Option<std::path::PathBuf>;
}

#[derive(Debug, PartialEq)]
pub enum Act {
    Cmd(Command),
    Raise,
    Quit,
}

#[derive(Debug, PartialEq)]
pub enum Reject {
    NotReady,
    NothingPlaying,
}

impl Reject {
    pub fn message(&self) -> &'static str {
        match self {
            Reject::NotReady => "engine not ready",
            Reject::NothingPlaying => "nothing is playing",
        }
    }
}

pub fn ready(s: &CoreState) -> bool {
    use presto_core::state::EngineStatus;
    matches!(s.engine, EngineStatus::Ready) &&
    s.auth.as_ref().map_or(false, |a| matches!(a, AuthState::SignedIn { .. }))
}

pub fn plan(op: &presto_ipc::ctl::CtlOp, s: &CoreState, pos_ms: u64) -> Result<Vec<Act>, Reject> {
    use presto_ipc::ctl::CtlOp;

    // Operations that work anytime
    match op {
        CtlOp::Raise => return Ok(vec![Act::Raise]),
        CtlOp::Quit => return Ok(vec![Act::Quit]),
        CtlOp::Status => return Ok(vec![]),
        CtlOp::Subscribe => return Ok(vec![]),
        _ => {}
    }

    // All other operations require ready state
    if !ready(s) {
        return Err(Reject::NotReady);
    }

    // Track-dependent operations
    match op {
        CtlOp::Play => {
            if s.player.track.is_none() {
                return Err(Reject::NothingPlaying);
            }
            Ok(vec![Act::Cmd(Command::Play)])
        }
        CtlOp::Pause => {
            if s.player.track.is_none() {
                return Err(Reject::NothingPlaying);
            }
            Ok(vec![Act::Cmd(Command::Pause)])
        }
        CtlOp::Toggle => {
            if s.player.track.is_none() {
                return Err(Reject::NothingPlaying);
            }
            Ok(vec![Act::Cmd(toggle_cmd(s.player.state))])
        }
        CtlOp::Stop => {
            if s.player.track.is_none() {
                return Err(Reject::NothingPlaying);
            }
            Ok(vec![Act::Cmd(Command::Pause)])
        }
        CtlOp::Next => {
            if s.player.track.is_none() {
                return Err(Reject::NothingPlaying);
            }
            Ok(vec![Act::Cmd(Command::Next)])
        }
        CtlOp::Prev => {
            if s.player.track.is_none() {
                return Err(Reject::NothingPlaying);
            }
            Ok(vec![Act::Cmd(Command::Prev)])
        }
        CtlOp::Seek { ms } => {
            if s.player.track.is_none() {
                return Err(Reject::NothingPlaying);
            }
            let target = seek_to(*ms, s.player.duration_ms);
            Ok(vec![Act::Cmd(Command::Seek { ms: target })])
        }
        CtlOp::SeekBy { ms } => {
            if s.player.track.is_none() {
                return Err(Reject::NothingPlaying);
            }
            let new_pos = seek_by(pos_ms, *ms, s.player.duration_ms);
            let target = seek_to(new_pos, s.player.duration_ms);
            Ok(vec![Act::Cmd(Command::Seek { ms: target })])
        }
        CtlOp::Volume { pct } => {
            let v = *pct as f32 / 100.0;
            Ok(vec![Act::Cmd(Command::SetVolume { volume: v })])
        }
        CtlOp::VolumeBy { pct } => {
            let new_vol = volume_by(s.player.volume, *pct);
            Ok(vec![Act::Cmd(Command::SetVolume { volume: new_vol })])
        }
        CtlOp::Shuffle { on } => {
            let should_shuffle = on.unwrap_or(!s.player.shuffle);
            Ok(vec![Act::Cmd(Command::SetShuffle { on: should_shuffle })])
        }
        CtlOp::Repeat { mode } => {
            let new_mode = mode.unwrap_or_else(|| next_repeat(s.player.repeat));
            Ok(vec![Act::Cmd(Command::SetRepeat { mode: new_mode })])
        }
        _ => unreachable!(),
    }
}

fn seek_to(ms: u64, dur: u64) -> u64 {
    // ponytail: clamp seeks below MIN_SEEK_MS to MIN_SEEK_MS; engines don't answer small seeks.
    // Upgrade path: if seeking becomes reliable for small values, remove the MIN_SEEK_MS constraint.
    ms.min(dur).max(MIN_SEEK_MS.min(dur))
}

pub fn apply(ctl: &dyn Control, acts: Vec<Act>) {
    for act in acts {
        match act {
            Act::Cmd(c) => ctl.send(c),
            Act::Raise => ctl.raise(),
            Act::Quit => ctl.quit(),
        }
    }
}

pub struct Position(std::sync::Mutex<Clock>);

impl Position {
    pub fn new() -> Self {
        Position(std::sync::Mutex::new(Clock::default()))
    }

    pub fn now(&self, s: &CoreState) -> u64 {
        let now = std::time::Instant::now();
        let mut clock = self.0.lock().unwrap();
        clock.observe(&s.player, now);
        clock.position(now)
    }
}

impl Default for Position {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use presto_core::mirror::PlayerMirror;
    use presto_ipc::ctl::CtlOp;

    fn mk_state(
        engine: presto_core::state::EngineStatus,
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
            duration_ms: 200_000,
            artwork_url: None,
            playable: true,
        }
    }

    #[test]
    fn play_with_ready_and_track() {
        let mut player = PlayerMirror::default();
        player.track = Some(item("1"));
        let state = mk_state(
            presto_core::state::EngineStatus::Ready,
            Some(AuthState::SignedIn),
            player,
        );
        let result = plan(&CtlOp::Play, &state, 0);
        assert_eq!(result, Ok(vec![Act::Cmd(Command::Play)]));
    }

    #[test]
    fn toggle_playing_to_pause() {
        let mut player = PlayerMirror::default();
        player.track = Some(item("1"));
        player.state = PlayState::Playing;
        let state = mk_state(
            presto_core::state::EngineStatus::Ready,
            Some(AuthState::SignedIn),
            player,
        );
        let result = plan(&CtlOp::Toggle, &state, 0);
        assert_eq!(result, Ok(vec![Act::Cmd(Command::Pause)]));
    }

    #[test]
    fn toggle_paused_to_play() {
        let mut player = PlayerMirror::default();
        player.track = Some(item("1"));
        player.state = PlayState::Paused;
        let state = mk_state(
            presto_core::state::EngineStatus::Ready,
            Some(AuthState::SignedIn),
            player,
        );
        let result = plan(&CtlOp::Toggle, &state, 0);
        assert_eq!(result, Ok(vec![Act::Cmd(Command::Play)]));
    }

    #[test]
    fn stop_becomes_pause() {
        let mut player = PlayerMirror::default();
        player.track = Some(item("1"));
        let state = mk_state(
            presto_core::state::EngineStatus::Ready,
            Some(AuthState::SignedIn),
            player,
        );
        let result = plan(&CtlOp::Stop, &state, 0);
        assert_eq!(result, Ok(vec![Act::Cmd(Command::Pause)]));
    }

    #[test]
    fn engine_not_ready_rejects() {
        let mut player = PlayerMirror::default();
        player.track = Some(item("1"));
        let state = mk_state(
            presto_core::state::EngineStatus::Starting,
            Some(AuthState::SignedIn),
            player,
        );
        assert_eq!(plan(&CtlOp::Play, &state, 0), Err(Reject::NotReady));
        assert_eq!(plan(&CtlOp::Next, &state, 0), Err(Reject::NotReady));
    }

    #[test]
    fn auth_not_signed_in_rejects() {
        let mut player = PlayerMirror::default();
        player.track = Some(item("1"));
        let state = mk_state(
            presto_core::state::EngineStatus::Ready,
            None,
            player,
        );
        assert_eq!(plan(&CtlOp::Play, &state, 0), Err(Reject::NotReady));
    }

    #[test]
    fn no_track_rejects_playback_ops() {
        let state = mk_state(
            presto_core::state::EngineStatus::Ready,
            Some(AuthState::SignedIn),
            PlayerMirror::default(),
        );
        assert_eq!(plan(&CtlOp::Play, &state, 0), Err(Reject::NothingPlaying));
        assert_eq!(plan(&CtlOp::Pause, &state, 0), Err(Reject::NothingPlaying));
        assert_eq!(plan(&CtlOp::Seek { ms: 5000 }, &state, 0), Err(Reject::NothingPlaying));
    }

    #[test]
    fn seek_below_min_is_clamped_up() {
        let mut player = PlayerMirror::default();
        player.track = Some(item("1"));
        player.duration_ms = 10_000;
        let state = mk_state(
            presto_core::state::EngineStatus::Ready,
            Some(AuthState::SignedIn),
            player,
        );
        let result = plan(&CtlOp::Seek { ms: 500 }, &state, 0);
        assert_eq!(result, Ok(vec![Act::Cmd(Command::Seek { ms: 3_000 })]));
    }

    #[test]
    fn seek_by_upward() {
        let mut player = PlayerMirror::default();
        player.track = Some(item("1"));
        player.duration_ms = 200_000;
        let state = mk_state(
            presto_core::state::EngineStatus::Ready,
            Some(AuthState::SignedIn),
            player,
        );
        let result = plan(&CtlOp::SeekBy { ms: 10_000 }, &state, 60_000);
        assert_eq!(result, Ok(vec![Act::Cmd(Command::Seek { ms: 70_000 })]));
    }

    #[test]
    fn seek_by_downward_clamps_to_min() {
        let mut player = PlayerMirror::default();
        player.track = Some(item("1"));
        player.duration_ms = 200_000;
        let state = mk_state(
            presto_core::state::EngineStatus::Ready,
            Some(AuthState::SignedIn),
            player,
        );
        let result = plan(&CtlOp::SeekBy { ms: -5_000 }, &state, 1_000);
        assert_eq!(result, Ok(vec![Act::Cmd(Command::Seek { ms: 3_000 })]));
    }

    #[test]
    fn seek_past_duration_clamps_to_dur() {
        let mut player = PlayerMirror::default();
        player.track = Some(item("1"));
        player.duration_ms = 10_000;
        let state = mk_state(
            presto_core::state::EngineStatus::Ready,
            Some(AuthState::SignedIn),
            player,
        );
        // Seek to duration means clamping to dur, but duration is 10k which is > MIN_SEEK_MS
        let result = plan(&CtlOp::Seek { ms: 2_000 }, &state, 0);
        // 2000 -> clamped to 3000 (MIN_SEEK_MS)
        assert_eq!(result, Ok(vec![Act::Cmd(Command::Seek { ms: 3_000 })]));
    }

    #[test]
    fn volume_absolute() {
        let state = mk_state(
            presto_core::state::EngineStatus::Ready,
            Some(AuthState::SignedIn),
            PlayerMirror::default(),
        );
        let result = plan(&CtlOp::Volume { pct: 50 }, &state, 0);
        assert_eq!(result, Ok(vec![Act::Cmd(Command::SetVolume { volume: 0.5 })]));
    }

    #[test]
    fn volume_by_down_clamps_to_zero() {
        let mut player = PlayerMirror::default();
        player.volume = 0.05;
        let state = mk_state(
            presto_core::state::EngineStatus::Ready,
            Some(AuthState::SignedIn),
            player,
        );
        let result = plan(&CtlOp::VolumeBy { pct: -10 }, &state, 0);
        assert_eq!(result, Ok(vec![Act::Cmd(Command::SetVolume { volume: 0.0 })]));
    }

    #[test]
    fn shuffle_with_none_toggles() {
        let mut player = PlayerMirror::default();
        player.shuffle = false;
        let state = mk_state(
            presto_core::state::EngineStatus::Ready,
            Some(AuthState::SignedIn),
            player,
        );
        let result = plan(&CtlOp::Shuffle { on: None }, &state, 0);
        assert_eq!(result, Ok(vec![Act::Cmd(Command::SetShuffle { on: true })]));
    }

    #[test]
    fn repeat_with_none_advances() {
        let mut player = PlayerMirror::default();
        player.repeat = RepeatMode::Off;
        let state = mk_state(
            presto_core::state::EngineStatus::Ready,
            Some(AuthState::SignedIn),
            player,
        );
        let result = plan(&CtlOp::Repeat { mode: None }, &state, 0);
        assert_eq!(result, Ok(vec![Act::Cmd(Command::SetRepeat { mode: RepeatMode::All })]));
    }

    #[test]
    fn raise_always_works() {
        let state = mk_state(
            presto_core::state::EngineStatus::Starting,
            None,
            PlayerMirror::default(),
        );
        let result = plan(&CtlOp::Raise, &state, 0);
        assert_eq!(result, Ok(vec![Act::Raise]));
    }

    #[test]
    fn quit_always_works() {
        let state = mk_state(
            presto_core::state::EngineStatus::Failed { reason: "x".into(), log_path: Default::default(), log_tail: vec![] },
            None,
            PlayerMirror::default(),
        );
        let result = plan(&CtlOp::Quit, &state, 0);
        assert_eq!(result, Ok(vec![Act::Quit]));
    }

    #[test]
    fn status_always_works() {
        let state = mk_state(
            presto_core::state::EngineStatus::Starting,
            None,
            PlayerMirror::default(),
        );
        let result = plan(&CtlOp::Status, &state, 0);
        assert_eq!(result, Ok(vec![]));
    }
}

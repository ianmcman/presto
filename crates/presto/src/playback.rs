//! Playback state and controls shown by the UI. Pure logic: callers pass `Instant`s in.
use presto_core::mirror::PlayerMirror;
use presto_ipc::{Command, ErrorKind, PlayState, QueueItem, RepeatMode};
use std::collections::HashSet;
use std::time::{Duration, Instant};

pub const SEEK_HOLD: Duration = Duration::from_secs(1);
pub const VOLUME_THROTTLE: Duration = Duration::from_millis(50);
/// Live MusicKit never answers seeks to targets under about 3 s (Phase 3 finding).
pub const MIN_SEEK_MS: u64 = 3000;

pub fn toggle_cmd(state: PlayState) -> Command {
    match state {
        PlayState::Playing | PlayState::Loading => Command::Pause,
        _ => Command::Play,
    }
}

pub fn next_repeat(m: RepeatMode) -> RepeatMode {
    match m {
        RepeatMode::Off => RepeatMode::All,
        RepeatMode::All => RepeatMode::One,
        RepeatMode::One => RepeatMode::Off,
    }
}

/// Interpolates position between engine progress events.
#[derive(Default)]
pub struct Clock {
    key: Option<(u64, u64, PlayState)>,
    anchor_ms: u64,
    anchor_at: Option<Instant>,
    playing: bool,
    duration_ms: u64,
}

impl Clock {
    pub fn observe(&mut self, p: &PlayerMirror, now: Instant) {
        self.duration_ms = p.duration_ms;
        let key = (p.seq, p.position_ms, p.state);
        if self.key != Some(key) {
            self.key = Some(key);
            self.anchor_ms = p.position_ms;
            self.anchor_at = Some(now);
            self.playing = p.state == PlayState::Playing;
        }
    }

    pub fn position(&self, now: Instant) -> u64 {
        let mut pos = self.anchor_ms;
        if let (true, Some(at)) = (self.playing, self.anchor_at) {
            pos += now.saturating_duration_since(at).as_millis() as u64;
        }
        pos.min(self.duration_ms)
    }
}

#[derive(Default)]
pub struct SeekBar {
    drag: Option<u64>,
    held: Option<(u64, u64, String, Instant)>,
}

impl SeekBar {
    pub fn drag(&mut self, ms: u64) {
        self.drag = Some(ms);
    }

    pub fn shown(&self, engine_ms: u64, seq: u64, track_id: &str, now: Instant) -> u64 {
        if let Some(d) = self.drag {
            return d;
        }
        match &self.held {
            Some((ms, s, id, at)) if seq <= *s && id == track_id && now.saturating_duration_since(*at) < SEEK_HOLD => *ms,
            _ => engine_ms,
        }
    }

    pub fn release(&mut self, now: Instant, seq: u64, track_id: &str) -> Option<Command> {
        let ms = self.drag.take()?;
        self.held = Some((ms, seq, track_id.to_string(), now));
        Some(Command::Seek { ms })
    }
}

pub fn seek_by(pos: u64, delta_ms: i64, dur: u64) -> u64 {
    (pos as i64).saturating_add(delta_ms).clamp(0, dur as i64) as u64
}

#[derive(Default)]
pub struct Volume {
    remembered: Option<f32>,
    last_sent: Option<Instant>,
    pending: Option<f32>,
}

impl Volume {
    pub fn toggle_mute(&mut self, current: f32) -> f32 {
        if current > 0.0 {
            self.remembered = Some(current);
            0.0
        } else {
            self.remembered.unwrap_or(0.5)
        }
    }

    pub fn drag(&mut self, v: f32, now: Instant) -> Option<f32> {
        if self.last_sent.is_none_or(|t| now.saturating_duration_since(t) >= VOLUME_THROTTLE) {
            self.last_sent = Some(now);
            self.pending = None;
            Some(v)
        } else {
            self.pending = Some(v);
            None
        }
    }

    pub fn release(&mut self) -> Option<f32> {
        self.last_sent = None;
        self.pending.take()
    }
}

pub fn volume_by(v: f32, pct: i32) -> f32 {
    (v + pct as f32 / 100.0).clamp(0.0, 1.0)
}

pub fn fmt_time(ms: u64) -> String {
    let s = ms / 1000;
    let (h, m, s) = (s / 3600, s / 60 % 60, s % 60);
    if h > 0 { format!("{h}:{m:02}:{s:02}") } else { format!("{m}:{s:02}") }
}

pub fn kind_label(k: &ErrorKind) -> String {
    match k {
        ErrorKind::Unavailable => "unavailable".into(),
        ErrorKind::Upstream { status } => format!("HTTP {status}"),
        ErrorKind::Timeout => "timeout".into(),
        ErrorKind::Internal => "engine error".into(),
        ErrorKind::NotFound => "not found".into(),
        ErrorKind::AuthExpired => "session expired".into(),
        ErrorKind::RateLimited { .. } => "rate limited".into(),
    }
}

pub struct GuardInput<'a> {
    pub track: Option<&'a QueueItem>,
    pub state: PlayState,
    pub position_ms: u64,
    pub queue_rev: u64,
    pub queue_index: Option<u32>,
    pub queue_len: usize,
    pub engine_errors: u64,
    pub last_error: Option<&'a ErrorKind>,
}

#[derive(Debug, PartialEq)]
pub enum GuardAction {
    SkipUnavailable { title: String },
    SkipFailed { title: String, kind: String },
    AllFailed,
}

/// Skips unplayable or failing tracks once each (D-11) and gives up when the whole queue fails (D-12).
pub struct Guard {
    bad: HashSet<String>,
    seen_errors: u64,
    last_skip: Option<(String, u64, Option<u32>)>,
    consecutive: usize,
}

impl Guard {
    pub fn new(engine_errors_now: u64) -> Guard {
        Guard { bad: HashSet::new(), seen_errors: engine_errors_now, last_skip: None, consecutive: 0 }
    }

    pub fn unavailable(&self, id: &str, playable: bool) -> bool {
        !playable || self.bad.contains(id)
    }

    pub fn observe(&mut self, i: &GuardInput) -> Option<GuardAction> {
        let new_err = i.engine_errors > self.seen_errors;
        self.seen_errors = i.engine_errors;
        let t = i.track?;
        if i.state == PlayState::Playing && i.position_ms > 0 && !self.unavailable(&t.id, t.playable) {
            self.consecutive = 0;
            return None;
        }
        let failing = new_err && i.state != PlayState::Playing;
        if failing {
            self.bad.insert(t.id.clone());
        }
        if !self.unavailable(&t.id, t.playable) {
            return None;
        }
        let key = (t.id.clone(), i.queue_rev, i.queue_index);
        if self.last_skip.as_ref() == Some(&key) {
            return None;
        }
        self.last_skip = Some(key);
        self.consecutive += 1;
        let title = t.title.clone();
        Some(if self.consecutive >= i.queue_len.max(1) {
            self.consecutive = 0;
            GuardAction::AllFailed
        } else if failing {
            GuardAction::SkipFailed { title, kind: kind_label(i.last_error.unwrap_or(&ErrorKind::Internal)) }
        } else {
            GuardAction::SkipUnavailable { title }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, playable: bool) -> QueueItem {
        QueueItem { id: id.into(), title: format!("T{id}"), artist: String::new(), album: String::new(), duration_ms: 1000, artwork_url: None, playable }
    }

    fn player(state: PlayState, seq: u64, pos: u64, dur: u64) -> PlayerMirror {
        PlayerMirror { state, seq, position_ms: pos, duration_ms: dur, ..Default::default() }
    }

    #[test]
    fn toggle_and_repeat() {
        for s in [PlayState::Paused, PlayState::Stopped, PlayState::Ended] {
            assert_eq!(toggle_cmd(s), Command::Play);
        }
        for s in [PlayState::Playing, PlayState::Loading] {
            assert_eq!(toggle_cmd(s), Command::Pause);
        }
        assert_eq!(next_repeat(RepeatMode::Off), RepeatMode::All);
        assert_eq!(next_repeat(RepeatMode::All), RepeatMode::One);
        assert_eq!(next_repeat(RepeatMode::One), RepeatMode::Off);
    }

    #[test]
    fn clock_interpolates_and_freezes() {
        let t0 = Instant::now();
        let mut c = Clock::default();
        c.observe(&player(PlayState::Playing, 1, 10_000, 200_000), t0);
        assert_eq!(c.position(t0 + Duration::from_millis(1500)), 11_500);
        c.observe(&player(PlayState::Paused, 2, 10_000, 200_000), t0);
        assert_eq!(c.position(t0 + Duration::from_secs(9)), 10_000);
    }

    #[test]
    fn clock_clamps_and_keeps_anchor_when_unchanged() {
        let t0 = Instant::now();
        let mut c = Clock::default();
        let p = player(PlayState::Playing, 1, 9_000, 10_000);
        c.observe(&p, t0);
        c.observe(&p, t0 + Duration::from_secs(1));
        assert_eq!(c.position(t0 + Duration::from_millis(1500)), 10_000);
        assert_eq!(c.position(t0 + Duration::from_millis(500)), 9_500);
    }

    #[test]
    fn seek_single_command_and_hold() {
        let t0 = Instant::now();
        let mut s = SeekBar::default();
        assert_eq!(s.release(t0, 1, "a"), None);
        s.drag(5_000);
        s.drag(6_000);
        assert_eq!(s.shown(1_000, 1, "a", t0), 6_000);
        assert_eq!(s.release(t0, 1, "a"), Some(Command::Seek { ms: 6_000 }));
        assert_eq!(s.release(t0, 1, "a"), None);
        assert_eq!(s.shown(1_000, 1, "a", t0 + Duration::from_millis(500)), 6_000);
    }

    #[test]
    fn seek_hold_ends() {
        let t0 = Instant::now();
        let mk = || {
            let mut s = SeekBar::default();
            s.drag(6_000);
            s.release(t0, 1, "a");
            s
        };
        assert_eq!(mk().shown(1_000, 2, "a", t0), 1_000, "newer seq");
        assert_eq!(mk().shown(1_000, 1, "b", t0), 1_000, "track change");
        assert_eq!(mk().shown(1_000, 1, "a", t0 + SEEK_HOLD), 1_000, "timeout");
    }

    #[test]
    fn seek_by_clamps() {
        assert_eq!(seek_by(5_000, -10_000, 100_000), 0);
        assert_eq!(seek_by(95_000, 10_000, 100_000), 100_000);
        assert_eq!(seek_by(5_000, 5_000, 100_000), 10_000);
    }

    #[test]
    fn mute_remembers() {
        let mut v = Volume::default();
        assert_eq!(v.toggle_mute(0.0), 0.5);
        assert_eq!(v.toggle_mute(0.8), 0.0);
        assert_eq!(v.toggle_mute(0.0), 0.8);
    }

    #[test]
    fn volume_throttle() {
        let t0 = Instant::now();
        let mut v = Volume::default();
        assert_eq!(v.drag(0.1, t0), Some(0.1));
        assert_eq!(v.drag(0.2, t0 + Duration::from_millis(10)), None);
        assert_eq!(v.drag(0.3, t0 + Duration::from_millis(20)), None);
        assert_eq!(v.release(), Some(0.3));
        assert_eq!(v.release(), None);
        assert_eq!(v.drag(0.4, t0 + Duration::from_millis(30)), Some(0.4));
        assert_eq!(v.drag(0.5, t0 + Duration::from_millis(80)), Some(0.5));
    }

    #[test]
    fn volume_by_clamps() {
        assert_eq!(volume_by(0.98, 5), 1.0);
        assert_eq!(volume_by(0.02, -5), 0.0);
        assert!((volume_by(0.5, 5) - 0.55).abs() < 1e-6);
    }

    #[test]
    fn time_format() {
        assert_eq!(fmt_time(0), "0:00");
        assert_eq!(fmt_time(65_000), "1:05");
        assert_eq!(fmt_time(3_725_000), "1:02:05");
    }

    #[test]
    fn labels() {
        assert_eq!(kind_label(&ErrorKind::Unavailable), "unavailable");
        assert_eq!(kind_label(&ErrorKind::Upstream { status: 503 }), "HTTP 503");
        assert_eq!(kind_label(&ErrorKind::Timeout), "timeout");
        assert_eq!(kind_label(&ErrorKind::Internal), "engine error");
        assert_eq!(kind_label(&ErrorKind::NotFound), "not found");
        assert_eq!(kind_label(&ErrorKind::AuthExpired), "session expired");
        assert_eq!(kind_label(&ErrorKind::RateLimited { retry_after_ms: None }), "rate limited");
    }

    fn inp<'a>(t: Option<&'a QueueItem>, state: PlayState, pos: u64, idx: u32, len: usize, errs: u64, le: Option<&'a ErrorKind>) -> GuardInput<'a> {
        GuardInput { track: t, state, position_ms: pos, queue_rev: 1, queue_index: Some(idx), queue_len: len, engine_errors: errs, last_error: le }
    }

    #[test]
    fn guard_skips_unplayable_once() {
        let mut g = Guard::new(0);
        let t = item("a", false);
        let i = inp(Some(&t), PlayState::Loading, 0, 0, 3, 0, None);
        assert_eq!(g.observe(&i), Some(GuardAction::SkipUnavailable { title: "Ta".into() }));
        assert_eq!(g.observe(&i), None);
    }

    #[test]
    fn guard_failed_track_marked_bad() {
        let mut g = Guard::new(2);
        let t = item("a", true);
        let e = ErrorKind::Upstream { status: 503 };
        assert_eq!(g.observe(&inp(Some(&t), PlayState::Loading, 0, 0, 3, 2, None)), None, "old errors ignored");
        assert_eq!(
            g.observe(&inp(Some(&t), PlayState::Loading, 0, 0, 3, 3, Some(&e))),
            Some(GuardAction::SkipFailed { title: "Ta".into(), kind: "HTTP 503".into() })
        );
        assert!(g.unavailable("a", true));
        // later TrackChanged into the bad id skips as unavailable
        let mut i = inp(Some(&t), PlayState::Loading, 0, 2, 3, 3, None);
        i.queue_rev = 2;
        assert_eq!(g.observe(&i), Some(GuardAction::SkipUnavailable { title: "Ta".into() }));
    }

    #[test]
    fn guard_single_item_all_failed() {
        let mut g = Guard::new(0);
        let t = item("a", false);
        assert_eq!(g.observe(&inp(Some(&t), PlayState::Loading, 0, 0, 1, 0, None)), Some(GuardAction::AllFailed));
    }

    #[test]
    fn guard_repeat_all_three_bad_stops() {
        let mut g = Guard::new(0);
        let ts = [item("a", false), item("b", false), item("c", false)];
        let mut out = vec![];
        for (n, t) in ts.iter().enumerate() {
            out.push(g.observe(&inp(Some(t), PlayState::Loading, 0, n as u32, 3, 0, None)));
        }
        assert!(matches!(out[0], Some(GuardAction::SkipUnavailable { .. })));
        assert!(matches!(out[1], Some(GuardAction::SkipUnavailable { .. })));
        assert_eq!(out[2], Some(GuardAction::AllFailed));
    }

    #[test]
    fn guard_playing_resets_counter() {
        let mut g = Guard::new(0);
        let bad = item("a", false);
        let ok = item("b", true);
        assert!(g.observe(&inp(Some(&bad), PlayState::Loading, 0, 0, 2, 0, None)).is_some());
        assert_eq!(g.observe(&inp(Some(&ok), PlayState::Playing, 500, 1, 2, 0, None)), None);
        let bad2 = item("c", false);
        assert!(matches!(g.observe(&inp(Some(&bad2), PlayState::Loading, 0, 0, 2, 0, None)), Some(GuardAction::SkipUnavailable { .. })));
        assert_eq!(g.observe(&inp(None, PlayState::Stopped, 0, 0, 2, 0, None)), None);
    }
}

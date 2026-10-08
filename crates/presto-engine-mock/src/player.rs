//! Playback clock and queue. Time is injected; position is derived from
//! `(base_ms, base_at)` and never incremented.

use crate::catalog;
use presto_ipc::{Command, ErrorKind, Event, IpcError, PlayState, QueueItem, RepeatMode};
use std::time::{Duration, Instant};

/// Runtime failure derived from the track itself (no fault kind needed).
fn failure(item: &QueueItem) -> Option<IpcError> {
    if !item.playable {
        Some(IpcError::new(
            ErrorKind::Unavailable,
            "mock: track is not available",
        ))
    } else if item.id == "s8" {
        Some(IpcError::new(
            ErrorKind::Upstream { status: 503 },
            "mock: stream failed",
        ))
    } else {
        None
    }
}

pub struct Player {
    queue: Vec<QueueItem>,
    rev: u64,
    index: Option<usize>,
    state: PlayState,
    base_ms: u64,
    base_at: Instant,
    volume: f32,
    shuffle: bool,
    repeat: RepeatMode,
    seq: u64,
    /// Mimics MusicKit on a fresh load: state is loading, Play/Pause are dropped while loading, the first seek is dropped, and playback autostarts 800 ms after load and 400 ms after any seek (03-06, 03-08 live gaps).
    pub restore_quirks: bool,
    loading: bool,
    autoplay_at: Option<Instant>,
}

impl Player {
    pub fn new(now: Instant) -> Self {
        Self {
            queue: vec![],
            rev: 0,
            index: None,
            state: PlayState::Stopped,
            base_ms: 0,
            base_at: now,
            volume: 1.0,
            shuffle: false,
            repeat: RepeatMode::Off,
            seq: 0,
            restore_quirks: false,
            loading: false,
            autoplay_at: None,
        }
    }

    pub fn audible(&self) -> bool {
        self.state == PlayState::Playing && self.volume > 0.0
    }

    fn duration(&self) -> u64 {
        self.index.map_or(0, |i| self.queue[i].duration_ms)
    }

    pub fn position(&self, now: Instant) -> u64 {
        if self.state == PlayState::Playing {
            let elapsed = now.saturating_duration_since(self.base_at).as_millis() as u64;
            (self.base_ms + elapsed).min(self.duration())
        } else {
            self.base_ms
        }
    }

    fn progress(&self, now: Instant) -> Event {
        Event::Progress {
            position_ms: self.position(now),
            duration_ms: self.duration(),
            seq: self.seq,
        }
    }

    fn state_evt(&self) -> Event {
        Event::PlaybackState {
            state: self.state,
            seq: self.seq,
        }
    }

    fn queue_evt(&self) -> Event {
        Event::QueueChanged {
            rev: self.rev,
            items: self.queue.clone(),
            index: self.index.map(|i| i as u32),
        }
    }

    fn track_evt(&self) -> Event {
        Event::TrackChanged {
            item: self.index.map(|i| self.queue[i].clone()),
        }
    }

    fn require_queue(&self) -> Result<(), IpcError> {
        if self.queue.is_empty() {
            Err(IpcError::new(ErrorKind::Unavailable, "queue is empty"))
        } else {
            Ok(())
        }
    }

    fn restart(&mut self, now: Instant) {
        self.base_ms = 0;
        self.base_at = now;
        self.seq += 1;
    }

    /// Stops with an error event when the current track cannot play.
    fn fail_current(&mut self, ev: &mut Vec<Event>, now: Instant) {
        let Some(error) = self.index.and_then(|i| failure(&self.queue[i])) else {
            return;
        };
        if !matches!(self.state, PlayState::Playing | PlayState::Loading) {
            return;
        }
        self.state = PlayState::Stopped;
        self.base_ms = 0;
        self.base_at = now;
        self.autoplay_at = None;
        self.seq += 1;
        ev.push(Event::Error { error });
        ev.push(self.state_evt());
    }

    fn move_to(&mut self, idx: usize, now: Instant) -> Vec<Event> {
        let was = self.state;
        self.index = Some(idx);
        self.rev += 1;
        self.restart(now);
        let mut ev = vec![self.queue_evt(), self.track_evt()];
        if matches!(was, PlayState::Ended | PlayState::Stopped) {
            self.state = PlayState::Playing;
            ev.push(self.state_evt());
        }
        ev.push(self.progress(now));
        self.fail_current(&mut ev, now);
        ev
    }

    /// Shared by `next` and natural track end.
    fn advance(&mut self, now: Instant) -> Vec<Event> {
        let i = self.index.unwrap_or(0);
        if i + 1 < self.queue.len() {
            self.move_to(i + 1, now)
        } else if self.repeat == RepeatMode::All {
            self.move_to(0, now)
        } else {
            self.state = PlayState::Ended;
            self.base_ms = self.duration();
            self.seq += 1;
            vec![self.state_evt()]
        }
    }

    pub fn apply(&mut self, cmd: &Command, now: Instant) -> Result<Vec<Event>, IpcError> {
        match cmd {
            Command::SetQueue { ids, start, play } => {
                let items = ids
                    .iter()
                    .map(|id| {
                        catalog::song(id).ok_or_else(|| {
                            IpcError::new(ErrorKind::NotFound, format!("unknown song id {id}"))
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                if items.is_empty() || *start as usize >= items.len() {
                    return Err(IpcError::new(
                        ErrorKind::Internal,
                        "start is outside the queue",
                    ));
                }
                self.queue = items;
                self.rev += 1;
                self.index = Some(*start as usize);
                self.state = if *play {
                    PlayState::Playing
                } else {
                    PlayState::Paused
                };
                self.restart(now);
                self.loading = self.restore_quirks;
                if self.restore_quirks {
                    self.state = PlayState::Loading;
                    self.autoplay_at = Some(now + Duration::from_millis(800));
                }
                let mut ev = vec![
                    self.queue_evt(),
                    self.track_evt(),
                    self.state_evt(),
                    self.progress(now),
                ];
                if *play {
                    self.fail_current(&mut ev, now);
                }
                Ok(ev)
            }
            Command::ShowWindow { .. } => Ok(vec![]),
            Command::Play => {
                self.require_queue()?;
                if self.restore_quirks && self.state == PlayState::Loading {
                    return Ok(vec![]);
                }
                let pos = if self.state == PlayState::Ended {
                    0
                } else {
                    self.position(now)
                };
                self.base_ms = pos;
                self.base_at = now;
                self.state = PlayState::Playing;
                self.seq += 1;
                let mut ev = vec![self.state_evt(), self.progress(now)];
                self.fail_current(&mut ev, now);
                Ok(ev)
            }
            Command::Pause => {
                self.require_queue()?;
                if self.restore_quirks && self.state == PlayState::Loading {
                    return Ok(vec![]);
                }
                self.base_ms = self.position(now);
                self.base_at = now;
                self.state = PlayState::Paused;
                self.seq += 1;
                Ok(vec![self.state_evt(), self.progress(now)])
            }
            Command::Seek { ms } => {
                self.require_queue()?;
                if self.loading {
                    self.loading = false;
                    return Ok(vec![]);
                }
                self.base_ms = (*ms).min(self.duration());
                self.base_at = now;
                self.seq += 1;
                if self.restore_quirks {
                    self.state = PlayState::Loading;
                    self.autoplay_at = Some(now + Duration::from_millis(400));
                    return Ok(vec![self.state_evt(), self.progress(now)]);
                }
                Ok(vec![self.progress(now)])
            }
            Command::Next => {
                self.require_queue()?;
                Ok(self.advance(now))
            }
            Command::Prev => {
                self.require_queue()?;
                let i = self.index.unwrap_or(0);
                if self.position(now) > 3000 || i == 0 {
                    self.restart(now);
                    Ok(vec![self.progress(now)])
                } else {
                    Ok(self.move_to(i - 1, now))
                }
            }
            Command::SetVolume { volume } => {
                self.volume = volume.clamp(0.0, 1.0);
                Ok(vec![Event::Volume {
                    volume: self.volume,
                }])
            }
            Command::SetShuffle { on } => {
                // ponytail: shuffle does not reorder the mock queue; reorder if a UI test needs it
                self.shuffle = *on;
                Ok(vec![Event::Shuffle { on: self.shuffle }])
            }
            Command::SetRepeat { mode } => {
                self.repeat = *mode;
                Ok(vec![Event::Repeat { mode: self.repeat }])
            }
        }
    }

    pub fn tick(&mut self, now: Instant) -> Vec<Event> {
        if self.autoplay_at.is_some_and(|t| now >= t) {
            self.autoplay_at = None;
            self.base_at = now;
            self.state = PlayState::Playing;
            self.seq += 1;
            return vec![self.state_evt(), self.progress(now)];
        }
        if self.state != PlayState::Playing {
            return vec![];
        }
        if self.position(now) < self.duration() {
            return vec![self.progress(now)];
        }
        if self.repeat == RepeatMode::One {
            self.restart(now);
            vec![self.progress(now)]
        } else {
            self.advance(now)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn q(ids: &[&str], start: u32) -> Command {
        Command::SetQueue {
            ids: ids.iter().map(|s| s.to_string()).collect(),
            start,
            play: true,
        }
    }
    fn secs(t0: Instant, s: u64) -> Instant {
        t0 + Duration::from_secs(s)
    }
    fn started() -> (Player, Instant) {
        let t0 = Instant::now();
        let mut p = Player::new(t0);
        p.apply(&q(&["s1", "s2"], 0), t0).unwrap();
        (p, t0)
    }

    #[test]
    fn set_queue_events() {
        let t0 = Instant::now();
        let mut p = Player::new(t0);
        let ev = p.apply(&q(&["s1", "s2"], 0), t0).unwrap();
        assert!(matches!(
            &ev[0],
            Event::QueueChanged {
                rev: 1,
                index: Some(0),
                ..
            }
        ));
        assert!(matches!(&ev[1], Event::TrackChanged { item: Some(i) } if i.id == "s1"));
        assert!(matches!(
            ev[2],
            Event::PlaybackState {
                state: PlayState::Playing,
                ..
            }
        ));
        assert!(matches!(ev[3], Event::Progress { position_ms: 0, .. }));
    }

    #[test]
    fn set_queue_paused() {
        let t0 = Instant::now();
        let mut p = Player::new(t0);
        let cmd = Command::SetQueue {
            ids: vec!["s1".into()],
            start: 0,
            play: false,
        };
        let ev = p.apply(&cmd, t0).unwrap();
        assert!(matches!(
            ev[2],
            Event::PlaybackState {
                state: PlayState::Paused,
                ..
            }
        ));
        assert_eq!(p.position(secs(t0, 5)), 0);
    }

    #[test]
    fn clock_pause_seek() {
        let (mut p, t0) = started();
        assert_eq!(p.position(secs(t0, 2)), 2000);
        p.apply(&Command::Pause, secs(t0, 2)).unwrap();
        assert_eq!(p.position(secs(t0, 10)), 2000);
        let seq = p.seq;
        p.apply(&Command::Seek { ms: 60000 }, secs(t0, 10)).unwrap();
        assert_eq!(p.base_ms, 60000);
        assert_eq!(p.seq, seq + 1);
    }

    #[test]
    fn natural_advance_and_end() {
        let (mut p, t0) = started();
        let ev = p.tick(secs(t0, 184));
        assert!(matches!(
            &ev[0],
            Event::QueueChanged {
                rev: 2,
                index: Some(1),
                ..
            }
        ));
        assert!(matches!(&ev[1], Event::TrackChanged { item: Some(i) } if i.id == "s2"));
        let ev = p.tick(secs(t0, 184 + 205));
        assert!(matches!(
            ev[0],
            Event::PlaybackState {
                state: PlayState::Ended,
                ..
            }
        ));
        assert!(p.tick(secs(t0, 1000)).is_empty());
    }

    #[test]
    fn repeat_all_wraps_and_one_restarts() {
        let (mut p, t0) = started();
        p.apply(
            &Command::SetRepeat {
                mode: RepeatMode::All,
            },
            t0,
        )
        .unwrap();
        p.apply(&Command::Next, t0).unwrap();
        let ev = p.apply(&Command::Next, t0).unwrap();
        assert!(matches!(&ev[0], Event::QueueChanged { index: Some(0), .. }));

        let (mut p, t0) = started();
        p.apply(
            &Command::SetRepeat {
                mode: RepeatMode::One,
            },
            t0,
        )
        .unwrap();
        let ev = p.tick(secs(t0, 184));
        assert!(matches!(
            ev.as_slice(),
            [Event::Progress { position_ms: 0, .. }]
        ));
        assert_eq!(p.index, Some(0));
    }

    #[test]
    fn next_prev() {
        let (mut p, t0) = started();
        p.apply(&Command::Next, t0).unwrap();
        let ev = p.apply(&Command::Next, t0).unwrap();
        assert!(matches!(
            ev[0],
            Event::PlaybackState {
                state: PlayState::Ended,
                ..
            }
        ));
        // prev from Ended at position == duration restarts? position > 3000 -> restart current
        p.apply(&Command::Prev, t0).unwrap();
        assert_eq!(p.index, Some(1));
        // within 3 s on index 1 moves back
        let ev = p.apply(&Command::Prev, t0).unwrap();
        assert!(matches!(&ev[0], Event::QueueChanged { index: Some(0), .. }));
        // index 0 restarts
        let ev = p.apply(&Command::Prev, t0).unwrap();
        assert!(matches!(
            ev.as_slice(),
            [Event::Progress { position_ms: 0, .. }]
        ));
    }

    #[test]
    fn errors_and_volume() {
        let t0 = Instant::now();
        let mut p = Player::new(t0);
        let e = p.apply(&Command::Play, t0).unwrap_err();
        assert_eq!(e.kind, ErrorKind::Unavailable);
        assert_eq!(
            p.apply(&q(&["zz"], 0), t0).unwrap_err().kind,
            ErrorKind::NotFound
        );
        assert_eq!(
            p.apply(&q(&["s1"], 1), t0).unwrap_err().kind,
            ErrorKind::Internal
        );
        let ev = p.apply(&Command::SetVolume { volume: 1.7 }, t0).unwrap();
        assert_eq!(ev, vec![Event::Volume { volume: 1.0 }]);
    }

    #[test]
    fn prev_after_3s_restarts_and_play_after_end_restarts() {
        let (mut p, t0) = started();
        p.apply(&Command::Next, t0).unwrap();
        let ev = p.apply(&Command::Prev, secs(t0, 5)).unwrap();
        assert!(matches!(
            ev.as_slice(),
            [Event::Progress { position_ms: 0, .. }]
        ));
        assert_eq!(p.index, Some(1));
        p.tick(secs(t0, 5 + 205));
        assert_eq!(p.state, PlayState::Ended);
        p.apply(&Command::Play, secs(t0, 300)).unwrap();
        assert_eq!(p.position(secs(t0, 300)), 0);
    }

    fn has_err(ev: &[Event], kind: ErrorKind) -> bool {
        ev.iter()
            .any(|e| matches!(e, Event::Error { error } if error.kind == kind))
    }

    #[test]
    fn fail_unavailable_then_next_plays() {
        let t0 = Instant::now();
        let mut p = Player::new(t0);
        let ev = p.apply(&q(&["s7", "s1"], 0), t0).unwrap();
        assert!(matches!(&ev[0], Event::QueueChanged { items, .. } if !items[0].playable));
        assert!(has_err(&ev, ErrorKind::Unavailable));
        assert_eq!(p.state, PlayState::Stopped);
        let ev = p.apply(&Command::Next, t0).unwrap();
        assert!(matches!(&ev[1], Event::TrackChanged { item: Some(i) } if i.id == "s1"));
        assert_eq!(p.state, PlayState::Playing);
    }

    #[test]
    fn fail_runtime_and_advance_into_failure() {
        let t0 = Instant::now();
        let mut p = Player::new(t0);
        let ev = p.apply(&q(&["s8"], 0), t0).unwrap();
        assert!(has_err(&ev, ErrorKind::Upstream { status: 503 }));
        assert_eq!(p.state, PlayState::Stopped);
        let mut p = Player::new(t0);
        p.apply(&q(&["s1", "s8"], 0), t0).unwrap();
        let ev = p.apply(&Command::Next, t0).unwrap();
        assert!(has_err(&ev, ErrorKind::Upstream { status: 503 }));
        assert_eq!((p.state, p.index), (PlayState::Stopped, Some(1)));
    }

    #[test]
    fn paused_load_no_error_until_play() {
        let t0 = Instant::now();
        let mut p = Player::new(t0);
        let cmd = Command::SetQueue {
            ids: vec!["s8".into()],
            start: 0,
            play: false,
        };
        let ev = p.apply(&cmd, t0).unwrap();
        assert!(!ev.iter().any(|e| matches!(e, Event::Error { .. })));
        let ev = p.apply(&Command::Play, t0).unwrap();
        assert!(has_err(&ev, ErrorKind::Upstream { status: 503 }));
        assert_eq!(p.state, PlayState::Stopped);
    }

    #[test]
    fn generated_ids_queue() {
        let t0 = Instant::now();
        let mut p = Player::new(t0);
        let ev = p.apply(&q(&["i.00001", "i.00002"], 0), t0).unwrap();
        assert!(matches!(&ev[0], Event::QueueChanged { items, .. }
            if items[0].title == "Song 00001" && items[1].title == "Song 00002"));
    }

    #[test]
    fn audible_needs_playing_and_volume() {
        let (mut p, t0) = started();
        assert!(p.audible());
        p.apply(&Command::SetVolume { volume: 0.0 }, t0).unwrap();
        assert!(!p.audible());
        p.apply(&Command::SetVolume { volume: 1.0 }, t0).unwrap();
        p.apply(&Command::Pause, t0).unwrap();
        assert!(!p.audible());
    }

    #[test]
    fn restore_quirks_model_musickit_load() {
        let t0 = Instant::now();
        let ms = |n| t0 + Duration::from_millis(n);
        let mut p = Player::new(t0);
        p.restore_quirks = true;
        let cmd = Command::SetQueue {
            ids: vec!["s1".into()],
            start: 0,
            play: false,
        };
        p.apply(&cmd, t0).unwrap();
        assert_eq!(p.state, PlayState::Loading);
        assert!(p.apply(&Command::Pause, t0).unwrap().is_empty());
        assert_eq!(p.state, PlayState::Loading);
        p.apply(&Command::Seek { ms: 60000 }, t0).unwrap();
        assert_eq!((p.position(t0), p.state), (0, PlayState::Loading));
        p.tick(ms(900));
        assert_eq!((p.position(ms(900)), p.state), (0, PlayState::Playing));
        p.apply(&Command::Seek { ms: 60000 }, ms(900)).unwrap();
        assert_eq!((p.position(ms(900)), p.state), (60000, PlayState::Loading));
        assert!(p.apply(&Command::Pause, ms(900)).unwrap().is_empty());
        p.tick(ms(1400));
        assert_eq!(p.state, PlayState::Playing);
        p.apply(&Command::Pause, ms(1400)).unwrap();
        assert_eq!(p.state, PlayState::Paused);
        assert!((60000..60600).contains(&p.position(ms(1400))));
    }
}

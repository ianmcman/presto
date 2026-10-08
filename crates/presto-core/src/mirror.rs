//! Read-only mirrors of the engine's queue and player. Rust never edits them; it only applies events.
use presto_ipc::{Event, PlayState, QueueItem, RepeatMode};
use serde::Serialize;
use std::hash::{DefaultHasher, Hash, Hasher};

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct QueueMirror {
    pub generation: u64,
    pub rev: u64,
    pub items: Vec<QueueItem>,
    pub index: Option<u32>,
}

impl QueueMirror {
    /// Applies only if `rev` is newer. Returns whether it applied.
    pub fn apply(&mut self, rev: u64, items: Vec<QueueItem>, index: Option<u32>) -> bool {
        if rev <= self.rev {
            return false;
        }
        (self.rev, self.items, self.index) = (rev, items, index);
        true
    }

    /// New page bridge: the engine restarts its revision count. Items stay for display.
    pub fn new_generation(&mut self) {
        self.generation += 1;
        self.rev = 0;
    }

    pub fn ids(&self) -> Vec<String> {
        self.items.iter().map(|i| i.id.clone()).collect()
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct PlayerMirror {
    pub state: PlayState,
    pub seq: u64,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub volume: f32,
    pub shuffle: bool,
    pub repeat: RepeatMode,
    pub track: Option<QueueItem>,
}

impl Default for PlayerMirror {
    fn default() -> Self {
        Self {
            state: PlayState::Stopped,
            seq: 0,
            position_ms: 0,
            duration_ms: 0,
            volume: 1.0,
            shuffle: false,
            repeat: RepeatMode::Off,
            track: None,
        }
    }
}

impl PlayerMirror {
    /// PlaybackState and Progress older than the newest seq are dropped.
    pub fn apply(&mut self, evt: &Event) {
        match evt {
            Event::PlaybackState { state, seq } if *seq >= self.seq => {
                self.state = *state;
                self.seq = *seq;
            }
            Event::Progress { position_ms, duration_ms, seq } if *seq >= self.seq => {
                self.position_ms = *position_ms;
                self.duration_ms = *duration_ms;
                self.seq = *seq;
            }
            Event::TrackChanged { item } => self.track = item.clone(),
            Event::Volume { volume } => self.volume = *volume,
            Event::Shuffle { on } => self.shuffle = *on,
            Event::Repeat { mode } => self.repeat = *mode,
            _ => {}
        }
    }

    pub fn new_generation(&mut self) {
        self.seq = 0;
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Snapshot {
    pub ids: Vec<String>,
    pub index: u32,
    pub position_ms: u64,
    pub was_playing: bool,
    pub volume: f32,
    pub shuffle: bool,
    pub repeat: RepeatMode,
}

/// None when the queue is empty.
pub fn snapshot(q: &QueueMirror, p: &PlayerMirror) -> Option<Snapshot> {
    if q.items.is_empty() {
        return None;
    }
    Some(Snapshot {
        ids: q.ids(),
        index: q.index.unwrap_or(0),
        position_ms: p.position_ms,
        was_playing: p.state == PlayState::Playing,
        volume: p.volume,
        shuffle: p.shuffle,
        repeat: p.repeat,
    })
}

/// Hash of the id list, for the same-queue crash counter.
pub fn queue_key(ids: &[String]) -> u64 {
    let mut h = DefaultHasher::new();
    ids.hash(&mut h);
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str) -> QueueItem {
        QueueItem { id: id.into(), title: id.into(), artist: String::new(), album: String::new(), duration_ms: 1000, artwork_url: None, playable: true }
    }
    fn items(ids: &[&str]) -> Vec<QueueItem> {
        ids.iter().map(|i| item(i)).collect()
    }

    #[test]
    fn mirror_drops_equal_and_lower_rev() {
        let mut q = QueueMirror::default();
        assert!(q.apply(3, items(&["s1"]), Some(0)));
        assert!(!q.apply(3, items(&["s2"]), Some(0)));
        assert!(!q.apply(2, items(&["s3"]), Some(0)));
        assert_eq!(q.ids(), ["s1"]);
    }

    #[test]
    fn mirror_new_generation_accepts_rev_1() {
        let mut q = QueueMirror::default();
        q.apply(9, items(&["s1"]), Some(0));
        q.new_generation();
        assert_eq!((q.generation, q.ids()), (1, vec!["s1".to_string()]));
        assert!(q.apply(1, items(&["s2"]), Some(0)));
        assert_eq!(q.ids(), ["s2"]);
    }

    #[test]
    fn mirror_progress_older_seq_dropped() {
        let mut p = PlayerMirror::default();
        p.apply(&Event::PlaybackState { state: PlayState::Playing, seq: 5 });
        p.apply(&Event::Progress { position_ms: 99, duration_ms: 1000, seq: 4 });
        assert_eq!(p.position_ms, 0);
        p.apply(&Event::Progress { position_ms: 99, duration_ms: 1000, seq: 5 });
        assert_eq!(p.position_ms, 99);
    }

    #[test]
    fn mirror_snapshot() {
        let mut q = QueueMirror::default();
        let mut p = PlayerMirror::default();
        assert!(snapshot(&q, &p).is_none());
        q.apply(1, items(&["s1", "s2", "s3"]), Some(1));
        p.apply(&Event::PlaybackState { state: PlayState::Playing, seq: 1 });
        p.apply(&Event::Progress { position_ms: 30000, duration_ms: 1000, seq: 1 });
        let s = snapshot(&q, &p).unwrap();
        assert_eq!((s.ids.len(), s.index, s.position_ms, s.was_playing), (3, 1, 30000, true));
    }

    #[test]
    fn mirror_queue_key_stable() {
        let a = ["s1".to_string(), "s2".to_string()];
        let b = ["s2".to_string(), "s1".to_string()];
        assert_eq!(queue_key(&a), queue_key(&a.clone()));
        assert_ne!(queue_key(&a), queue_key(&b));
    }
}

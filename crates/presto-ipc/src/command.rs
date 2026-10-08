use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RepeatMode {
    Off,
    One,
    All,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    Play,
    Pause,
    Seek {
        ms: u64,
    },
    Next,
    Prev,
    /// 0.0..=1.0
    SetVolume {
        volume: f32,
    },
    SetShuffle {
        on: bool,
    },
    SetRepeat {
        mode: RepeatMode,
    },
    SetQueue {
        ids: Vec<String>,
        start: u32,
        /// false: load paused (restore after a restart). Omitted = true.
        #[serde(default = "yes")]
        play: bool,
    },
    /// Handled by the engine host, never by the page; works before bridge_ready.
    ShowWindow {
        show: bool,
    },
}

fn yes() -> bool {
    true
}

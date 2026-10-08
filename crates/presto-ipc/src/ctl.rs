use crate::{AuthState, RepeatMode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const CTL_PROTO: u32 = 1;
pub const MAX_LINE: usize = 4096;

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct CtlRequest {
    pub proto: u32,
    pub id: u64,
    #[serde(flatten)]
    pub op: CtlOp,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum CtlOp {
    Play,
    Pause,
    Toggle,
    Next,
    Prev,
    Stop,
    Seek { ms: u64 },
    SeekBy { ms: i64 },
    Volume { pct: u8 },
    VolumeBy { pct: i32 },
    Shuffle { on: Option<bool> },
    Repeat { mode: Option<RepeatMode> },
    Status,
    Subscribe,
    Raise,
    Quit,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct CtlReply {
    pub proto: u32,
    pub id: u64,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<Status>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct Status {
    pub proto: u32,
    pub state: StatusState,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub volume: u8,
    pub shuffle: bool,
    pub repeat: RepeatMode,
    pub track: Option<StatusTrack>,
    pub auth: Option<AuthState>,
    pub engine: EngineLabel,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct StatusTrack {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub artwork_path: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StatusState {
    Playing,
    Paused,
    Stopped,
    Loading,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EngineLabel {
    Ready,
    Starting,
    Restarting,
    Failed,
}

impl CtlReply {
    pub fn ok(id: u64) -> Self {
        Self {
            proto: CTL_PROTO,
            id,
            ok: true,
            error: None,
            status: None,
        }
    }

    pub fn status(id: u64, status: Status) -> Self {
        Self {
            proto: CTL_PROTO,
            id,
            ok: true,
            error: None,
            status: Some(status),
        }
    }

    pub fn err(id: u64, msg: impl Into<String>) -> Self {
        Self {
            proto: CTL_PROTO,
            id,
            ok: false,
            error: Some(msg.into()),
            status: None,
        }
    }
}

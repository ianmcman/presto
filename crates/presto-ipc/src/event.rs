use crate::{IpcError, RepeatMode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlayState {
    Stopped,
    Loading,
    Playing,
    Paused,
    Ended,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuthState {
    SignedOut,
    SigningIn,
    SignedIn,
    Expired,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq, Eq)]
pub struct QueueItem {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
    pub artwork_url: Option<String>,
    pub playable: bool,
}

/// Widevine CDM component state (1.2). Engines without a CDM never send it.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CdmState {
    Checking,
    Ready,
    Failed,
}

/// `seq` increments on every user-initiated state change (play, pause, seek,
/// skip, set_queue) so receivers drop stale Progress.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    PlaybackState {
        state: PlayState,
        seq: u64,
    },
    Progress {
        position_ms: u64,
        duration_ms: u64,
        seq: u64,
    },
    TrackChanged {
        item: Option<QueueItem>,
    },
    QueueChanged {
        rev: u64,
        items: Vec<QueueItem>,
        index: Option<u32>,
    },
    Volume {
        volume: f32,
    },
    Shuffle {
        on: bool,
    },
    Repeat {
        mode: RepeatMode,
    },
    Auth {
        state: AuthState,
    },
    Error {
        error: IpcError,
    },
    /// Sent each time the page bridge installs (every page load). Engines reset queue rev and seq at this point.
    BridgeReady {
        version: String,
        capabilities: Vec<String>,
        musickit_build: Option<String>,
    },
    /// Sent before the engine waits for the CDM, then once with the outcome.
    Cdm {
        state: CdmState,
        version: Option<String>,
        message: Option<String>,
    },
}

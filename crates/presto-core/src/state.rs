//! State published by the supervisor over a `watch` channel.
use crate::mirror::{PlayerMirror, QueueMirror};
use presto_ipc::AuthState;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub enum EngineStatus {
    #[default]
    Starting,
    Ready,
    Restarting { attempt: u32, delay_ms: u64 },
    Drift { reason: String, log_path: PathBuf },
    Failed { reason: String, log_path: PathBuf, log_tail: Vec<String> },
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct BridgeInfo {
    pub version: String,
    pub capabilities: Vec<String>,
    pub musickit_build: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct CoreState {
    pub engine: EngineStatus,
    pub bridge: Option<BridgeInfo>,
    pub restarts: u32,
    pub log_path: Option<PathBuf>,
    pub auth: Option<AuthState>,
    pub queue: QueueMirror,
    pub player: PlayerMirror,
}

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

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct CdmInfo {
    pub state: presto_ipc::CdmState,
    pub version: Option<String>,
    pub message: Option<String>,
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
    /// Runtime engine error events seen so far (D-12).
    pub engine_errors: u64,
    pub last_engine_error: Option<presto_ipc::IpcError>,
    /// Last Widevine CDM report this attempt; None for engines that never send one.
    pub cdm: Option<CdmInfo>,
}

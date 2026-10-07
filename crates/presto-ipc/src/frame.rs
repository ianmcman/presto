use crate::{ApiRequest, Command, Event, FaultSpec, Outcome};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtoVersion {
    pub major: u16,
    pub minor: u16,
}

impl fmt::Display for ProtoVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

pub const PROTO: ProtoVersion = ProtoVersion { major: 1, minor: 0 };

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Presto,
    Engine,
}

pub mod caps {
    pub const PLAYBACK: &str = "playback";
    pub const QUEUE: &str = "queue";
    pub const API: &str = "api";
    pub const MOCK: &str = "mock";
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct Hello {
    pub proto: ProtoVersion,
    pub role: Role,
    pub capabilities: Vec<String>,
    pub engine: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ProtoError {
    #[error(
        "IPC protocol major version mismatch: this side speaks {ours}, peer speaks {theirs}. Update presto or the engine so both use major version {ours_major}."
    )]
    MajorMismatch {
        ours: ProtoVersion,
        theirs: ProtoVersion,
        ours_major: u16,
    },
}

impl Hello {
    pub fn new(role: Role, capabilities: &[&str], engine: Option<String>) -> Self {
        Self {
            proto: PROTO,
            role,
            capabilities: capabilities.iter().map(|c| c.to_string()).collect(),
            engine,
        }
    }

    pub fn has(&self, cap: &str) -> bool {
        self.capabilities.iter().any(|c| c == cap)
    }

    /// Err only when the major version differs (D-04).
    pub fn check(&self, ours: ProtoVersion) -> Result<(), ProtoError> {
        if self.proto.major == ours.major {
            Ok(())
        } else {
            Err(ProtoError::MajorMismatch {
                ours,
                theirs: self.proto,
                ours_major: ours.major,
            })
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum Frame {
    Hello(Hello),
    Cmd { id: u64, cmd: Command },
    Req { id: u64, req: ApiRequest },
    Res { id: u64, outcome: Outcome },
    Evt { evt: Event },
    Ping { seq: u64 },
    Pong { seq: u64 },
    Mock { id: u64, fault: FaultSpec },
}

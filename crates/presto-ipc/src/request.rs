use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct ApiRequest {
    pub method: HttpMethod,
    /// e.g. "/v1/me/library/playlists"
    pub path: String,
    #[serde(default)]
    pub query: BTreeMap<String, String>,
    #[serde(default)]
    pub body: Option<serde_json::Value>,
}

impl ApiRequest {
    pub fn get(path: impl Into<String>) -> Self {
        Self {
            method: HttpMethod::Get,
            path: path.into(),
            query: BTreeMap::new(),
            body: None,
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Outcome {
    Ok {
        #[serde(default)]
        data: serde_json::Value,
    },
    Err {
        error: IpcError,
    },
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct IpcError {
    pub kind: ErrorKind,
    pub message: String,
}

impl IpcError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

/// `Timeout` is synthesized locally by the requester when `Kind::timeout` elapses;
/// engines may also send it.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum ErrorKind {
    Timeout,
    AuthExpired,
    RateLimited { retry_after_ms: Option<u64> },
    NotFound,
    Unavailable,
    Upstream { status: u16 },
    Internal,
}

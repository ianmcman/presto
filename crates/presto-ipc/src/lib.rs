//! Presto engine IPC protocol. See docs/PROTOCOL.md.

mod command;
mod event;
mod fault;
mod frame;
mod kind;
mod request;
pub mod transport;

pub use command::{Command, RepeatMode};
pub use event::{AuthState, Event, PlayState, QueueItem};
pub use fault::{DEFAULT_SLOW_MS, FaultSpec};
pub use frame::{Frame, Hello, PROTO, ProtoError, ProtoVersion, Role, caps};
pub use kind::{HEARTBEAT_INTERVAL, HEARTBEAT_MISSES, Kind};
pub use request::{ApiRequest, ErrorKind, HttpMethod, IpcError, Outcome};

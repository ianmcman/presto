use crate::{Command, Frame, HttpMethod};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Hello,
    Command,
    SetQueue,
    ApiRead,
    ApiWrite,
    MockControl,
}

impl Kind {
    /// Enforced by the requester; never sent on the wire.
    pub const fn timeout(self) -> Duration {
        Duration::from_secs(match self {
            Kind::Hello | Kind::Command | Kind::MockControl => 5,
            Kind::SetQueue => 15,
            Kind::ApiRead => 20,
            Kind::ApiWrite => 30,
        })
    }
}

/// Presto sends Ping this often.
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(2);
/// Hang is declared after this many unanswered pings (6 s).
pub const HEARTBEAT_MISSES: u32 = 3;

impl Frame {
    pub fn kind(&self) -> Option<Kind> {
        match self {
            Frame::Cmd {
                cmd: Command::SetQueue { .. },
                ..
            } => Some(Kind::SetQueue),
            Frame::Cmd { .. } => Some(Kind::Command),
            Frame::Req { req, .. } if req.method == HttpMethod::Get => Some(Kind::ApiRead),
            Frame::Req { .. } => Some(Kind::ApiWrite),
            Frame::Hello(_) => Some(Kind::Hello),
            Frame::Mock { .. } => Some(Kind::MockControl),
            _ => None,
        }
    }
}

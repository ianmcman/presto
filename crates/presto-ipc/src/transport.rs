use crate::Frame;
use futures_util::{Sink, SinkExt, Stream, StreamExt};
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};
use tokio::net::{UnixListener, UnixStream};
use tokio_util::codec::{Framed, LinesCodec, LinesCodecError};

/// Engines paginate to stay under this.
pub const MAX_LINE: usize = 4 * 1024 * 1024;
/// Engine contract (D-02): argv `--socket <path> --profile <dir>`, or these env vars; argv wins.
pub const ENV_SOCKET: &str = "PRESTO_SOCKET";
pub const ENV_PROFILE: &str = "PRESTO_PROFILE";

pub type Conn = Framed<UnixStream, LinesCodec>;

#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("XDG_RUNTIME_DIR is not set; pass an explicit socket path")]
    NoRuntimeDir,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Codec(#[from] LinesCodecError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// `$XDG_RUNTIME_DIR/presto/engine.sock`
pub fn socket_path() -> Result<PathBuf, TransportError> {
    let dir = std::env::var_os("XDG_RUNTIME_DIR").ok_or(TransportError::NoRuntimeDir)?;
    Ok(PathBuf::from(dir).join("presto").join("engine.sock"))
}

/// Creates the parent dir (0700), removes a stale socket file, binds, sets mode 0600.
pub fn bind(path: &Path) -> Result<UnixListener, TransportError> {
    if let Some(parent) = path.parent() {
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(parent)?;
    }
    match std::fs::remove_file(path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e.into()),
        _ => {}
    }
    let l = UnixListener::bind(path)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(l)
}

pub async fn connect(path: &Path) -> Result<Conn, TransportError> {
    Ok(framed(UnixStream::connect(path).await?))
}

pub fn framed(s: UnixStream) -> Conn {
    Framed::new(s, LinesCodec::new_with_max_length(MAX_LINE))
}

pub async fn send<S>(sink: &mut S, f: &Frame) -> Result<(), TransportError>
where
    S: Sink<String, Error = LinesCodecError> + Unpin,
{
    Ok(sink.send(serde_json::to_string(f)?).await?)
}

/// `Ok(None)` on EOF. Cancel-safe: only awaits `StreamExt::next`, so it may be
/// used inside `tokio::select!`.
pub async fn recv<S>(stream: &mut S) -> Result<Option<Frame>, TransportError>
where
    S: Stream<Item = Result<String, LinesCodecError>> + Unpin,
{
    match stream.next().await {
        None => Ok(None),
        Some(line) => Ok(Some(serde_json::from_str(&line?)?)),
    }
}

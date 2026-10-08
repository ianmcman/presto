pub mod parse;
pub mod server;
pub mod client;

pub use server::serve;

use std::fs::File;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use nix::fcntl::{FlockArg, Flock};
use presto_core::paths::ensure_private_dir;
use tokio::net::UnixListener;

#[derive(Debug)]
pub struct CtlPaths {
    pub dir: PathBuf,
    pub sock: PathBuf,
    pub lock: PathBuf,
}

impl CtlPaths {
    pub fn new(runtime_dir: &Path, demo: bool) -> Self {
        let dir = if demo {
            runtime_dir.join("presto-demo")
        } else {
            runtime_dir.join("presto")
        };
        let sock_name = if demo { "ctl-demo.sock" } else { "ctl.sock" };
        let lock_name = if demo { "ctl-demo.lock" } else { "ctl.lock" };
        CtlPaths {
            sock: dir.join(sock_name),
            lock: dir.join(lock_name),
            dir,
        }
    }
}

#[derive(Debug)]
pub enum LockError {
    Held,
    Io(io::Error),
}

impl std::fmt::Display for LockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LockError::Held => write!(f, "another instance is running"),
            LockError::Io(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for LockError {}

pub struct CtlGuard {
    _lock: nix::fcntl::Flock<File>,
    sock: PathBuf,
}

impl Drop for CtlGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.sock);
    }
}

pub fn lock(p: &CtlPaths) -> Result<CtlGuard, LockError> {
    ensure_private_dir(&p.dir).map_err(LockError::Io)?;
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&p.lock)
        .map_err(LockError::Io)?;

    let flock = Flock::lock(f, FlockArg::LockExclusiveNonblock)
        .map_err(|(_f, e)| {
            if e == nix::errno::Errno::EWOULDBLOCK {
                LockError::Held
            } else {
                LockError::Io(io::Error::from(e))
            }
        })?;

    Ok(CtlGuard {
        _lock: flock,
        sock: p.sock.clone(),
    })
}

pub fn bind(guard: &CtlGuard) -> io::Result<UnixListener> {
    let _ = std::fs::remove_file(&guard.sock);
    let listener = UnixListener::bind(&guard.sock)?;
    std::fs::set_permissions(&guard.sock, std::fs::Permissions::from_mode(0o600))?;
    Ok(listener)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctl_paths_real() {
        let p = CtlPaths::new(Path::new("/run/user/1000"), false);
        assert_eq!(p.dir, PathBuf::from("/run/user/1000/presto"));
        assert_eq!(p.sock, PathBuf::from("/run/user/1000/presto/ctl.sock"));
        assert_eq!(p.lock, PathBuf::from("/run/user/1000/presto/ctl.lock"));
    }

    #[test]
    fn ctl_paths_demo() {
        let p = CtlPaths::new(Path::new("/run/user/1000"), true);
        assert_eq!(p.dir, PathBuf::from("/run/user/1000/presto-demo"));
        assert_eq!(p.sock, PathBuf::from("/run/user/1000/presto-demo/ctl-demo.sock"));
        assert_eq!(p.lock, PathBuf::from("/run/user/1000/presto-demo/ctl-demo.lock"));
    }
}

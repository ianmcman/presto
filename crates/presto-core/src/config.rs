//! Core configuration: how to launch the engine and the supervisor timings.
use crate::paths::Paths;
use presto_ipc::{HEARTBEAT_INTERVAL, HEARTBEAT_MISSES};
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

pub const REQUIRED_BRIDGE_CAPS: &[&str] = &["playback", "queue", "api"];

#[derive(Clone, Debug)]
pub struct Launch {
    pub program: PathBuf,
    pub args: Vec<OsString>,
}

impl Launch {
    /// `<engine_dir>/node_modules/electron/dist/<path.txt>`, args = [engine_dir, extra...].
    /// Spawns the real binary: killing the `.bin/electron` Node wrapper would orphan it.
    pub fn electron(engine_dir: &Path, extra: &[&str]) -> io::Result<Launch> {
        let pt = engine_dir.join("node_modules/electron/path.txt");
        let rel = std::fs::read_to_string(&pt).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!(
                    "cannot read {}: {e}; in a checkout run npm install --allow-git=root and node node_modules/electron/install.js in engine/, or pass --engine-dir",
                    pt.display()
                ),
            )
        })?;
        let mut args: Vec<OsString> = vec![engine_dir.into()];
        args.extend(extra.iter().map(OsString::from));
        Ok(Launch { program: engine_dir.join("node_modules/electron/dist").join(rel.trim()), args })
    }
}

/// Called with the 0-based attempt number before each spawn.
pub type Launcher = Arc<dyn Fn(u32) -> Launch + Send + Sync>;

#[derive(Clone, Debug)]
pub struct Timings {
    pub heartbeat: Duration,
    pub misses: u32,
    pub connect: Duration,
    pub drift: Duration,
    pub backoff_base: Duration,
    pub backoff_cap: Duration,
    pub fast: Duration,
    pub stable: Duration,
    pub kill_grace: Duration,
    pub quit_wait: Duration,
}

impl Default for Timings {
    fn default() -> Self {
        let s = Duration::from_secs;
        Timings {
            heartbeat: HEARTBEAT_INTERVAL,
            misses: HEARTBEAT_MISSES,
            connect: s(30),
            drift: s(15),
            backoff_base: s(1),
            backoff_cap: s(30),
            fast: s(60),
            stable: s(120),
            kill_grace: s(3),
            quit_wait: s(10),
        }
    }
}

pub struct CoreConfig {
    pub launcher: Launcher,
    pub socket: PathBuf,
    pub paths: Paths,
    pub timings: Timings,
}

/// D-16: the page must expose MusicKit and every required capability.
pub fn check_bridge(caps: &[String], musickit_build: Option<&str>) -> Result<(), String> {
    const P: &str = "Apple's web player changed; update bridge.js: ";
    if musickit_build.is_none() {
        return Err(format!("{P}MusicKit not found on the page"));
    }
    match REQUIRED_BRIDGE_CAPS.iter().find(|c| !caps.iter().any(|x| x == **c)) {
        Some(c) => Err(format!("{P}bridge lacks capability {c}")),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caps(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn config_check_bridge_ok() {
        assert!(check_bridge(&caps(&["playback", "queue", "api", "x"]), Some("1.0")).is_ok());
    }

    #[test]
    fn config_check_bridge_missing_cap() {
        let e = check_bridge(&caps(&["playback", "queue"]), Some("1.0")).unwrap_err();
        assert!(e.contains("capability api"), "{e}");
    }

    #[test]
    fn config_check_bridge_no_build() {
        let e = check_bridge(&caps(&["playback", "queue", "api"]), None).unwrap_err();
        assert!(e.contains("MusicKit not found"), "{e}");
    }

    #[test]
    fn config_electron_path() {
        let t = tempfile::tempdir().unwrap();
        assert!(Launch::electron(t.path(), &[]).unwrap_err().to_string().contains("path.txt"));
        std::fs::create_dir_all(t.path().join("node_modules/electron")).unwrap();
        std::fs::write(t.path().join("node_modules/electron/path.txt"), "electron\n").unwrap();
        let l = Launch::electron(t.path(), &["--x"]).unwrap();
        assert_eq!(l.program, t.path().join("node_modules/electron/dist/electron"));
        assert_eq!(l.args, [t.path().as_os_str(), "--x".as_ref()]);
    }
}

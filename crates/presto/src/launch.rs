//! Core configs for demo (mock engine) and real (Electron engine) runs.
use presto_core::paths::Paths;
use presto_core::{CoreConfig, Launch, Timings};
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const DEMO_ARGS: &[&str] = &["--library-songs", "250"];

/// `PRESTO_ENGINE_MOCK` wins if the file exists, else the binary next to the executable.
pub fn mock_path(exe_dir: &Path, env: Option<OsString>) -> Result<PathBuf, String> {
    if let Some(p) = env.map(PathBuf::from).filter(|p| p.exists()) {
        return Ok(p);
    }
    let p = exe_dir.join("presto-engine-mock");
    if p.exists() {
        return Ok(p);
    }
    Err(format!(
        "presto-engine-mock not found at {}; run cargo build -p presto-engine-mock or set PRESTO_ENGINE_MOCK",
        p.display()
    ))
}

/// Faults pass straight to the mock (D-16).
pub fn demo_extra(faults: &[String]) -> Vec<OsString> {
    faults.iter().flat_map(|f| ["--fault".into(), f.into()]).collect()
}

/// Wipes `state_root/demo` so a stale cached mock catalog never shows up; the real cache is untouched.
pub fn demo_config(state_root: &Path, runtime_dir: &Path, mock: PathBuf, extra: Vec<OsString>) -> io::Result<CoreConfig> {
    let dir = state_root.join("demo");
    match std::fs::remove_dir_all(&dir) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
        _ => {}
    }
    let mut args: Vec<OsString> = DEMO_ARGS.iter().map(OsString::from).collect();
    args.extend(extra);
    let socket_dir = runtime_dir.join("presto");
    presto_core::paths::ensure_private_dir(&socket_dir)?;
    Ok(CoreConfig {
        launcher: Arc::new(move |_| Launch { program: mock.clone(), args: args.clone() }),
        socket: socket_dir.join("engine-demo.sock"),
        paths: Paths::under(dir),
        timings: Timings::default(),
    })
}

/// D-03: an installed engine next to the executable (`<exe>/../lib/presto/engine`) wins; else `./engine` for dev checkouts.
pub fn default_engine_dir(exe: Option<&Path>) -> PathBuf {
    if let Some(p) = exe.and_then(Path::parent).map(|d| d.join("../lib/presto/engine"))
        && p.join("node_modules/electron/path.txt").exists()
    {
        return p;
    }
    PathBuf::from("engine")
}

pub fn real_config(engine_dir: &Path) -> io::Result<CoreConfig> {
    let launch = Launch::electron(engine_dir, &[])?;
    Ok(CoreConfig {
        launcher: Arc::new(move |_| launch.clone()),
        socket: presto_ipc::transport::socket_path().map_err(io::Error::other)?,
        paths: Paths::from_env(),
        timings: Timings::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_dir_installed() {
        let d = tempfile::tempdir().unwrap();
        let e = d.path().join("usr/lib/presto/engine/node_modules/electron");
        std::fs::create_dir_all(&e).unwrap();
        std::fs::write(e.join("path.txt"), "x").unwrap();
        std::fs::create_dir_all(d.path().join("usr/bin")).unwrap();
        let exe = d.path().join("usr/bin/presto");
        assert_eq!(
            default_engine_dir(Some(&exe)),
            d.path().join("usr/bin/../lib/presto/engine")
        );
    }

    #[test]
    fn engine_dir_missing_falls_back() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(default_engine_dir(Some(&d.path().join("usr/bin/presto"))), PathBuf::from("engine"));
    }

    #[test]
    fn engine_dir_no_exe() {
        assert_eq!(default_engine_dir(None), PathBuf::from("engine"));
    }

    #[test]
    fn mock_path_missing() {
        let d = tempfile::tempdir().unwrap();
        let err = mock_path(d.path(), None).unwrap_err();
        assert!(err.contains("cargo build -p presto-engine-mock") && err.contains("PRESTO_ENGINE_MOCK"));
    }

    #[test]
    fn mock_path_sibling() {
        let d = tempfile::tempdir().unwrap();
        let sib = d.path().join("presto-engine-mock");
        std::fs::write(&sib, "").unwrap();
        assert_eq!(mock_path(d.path(), None).unwrap(), sib);
    }

    #[test]
    fn mock_path_env() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("presto-engine-mock"), "").unwrap();
        let envp = d.path().join("other");
        std::fs::write(&envp, "").unwrap();
        assert_eq!(mock_path(d.path(), Some(envp.clone().into())).unwrap(), envp);
    }

    #[test]
    fn demo_wiped() {
        let d = tempfile::tempdir().unwrap();
        let state = d.path().join("s");
        std::fs::create_dir_all(state.join("demo")).unwrap();
        std::fs::write(state.join("demo/sentinel"), "").unwrap();
        demo_config(&state, &d.path().join("r"), "/x/mock".into(), vec![]).unwrap();
        assert!(!state.join("demo/sentinel").exists());
    }

    #[test]
    fn demo_isolated() {
        let d = tempfile::tempdir().unwrap();
        let (state, rt) = (d.path().join("s"), d.path().join("r"));
        let mock = PathBuf::from("/x/mock");
        let c = demo_config(&state, &rt, mock.clone(), demo_extra(&["slow".into()])).unwrap();
        let p = &c.paths;
        assert_eq!(p.state, state.join("demo"));
        for f in [&p.profile, &p.logs, &p.pidfile, &p.cache, &p.artwork, &p.db, &p.install_id] {
            assert!(f.starts_with(&p.state));
        }
        assert_eq!(c.socket, rt.join("presto/engine-demo.sock"));
        let l = (c.launcher)(0);
        assert_eq!(l.program, mock);
        assert_eq!(l.args, ["--library-songs", "250", "--fault", "slow"].map(OsString::from));
    }
}

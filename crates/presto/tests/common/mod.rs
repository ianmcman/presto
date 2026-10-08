#![allow(dead_code)]
use presto::backend::Backend;
use presto::launch::{demo_config, demo_extra};
use presto_core::CoreState;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tempfile::TempDir;

pub fn mock_bin() -> PathBuf {
    let p = std::env::current_exe().unwrap().parent().unwrap().parent().unwrap().join("presto-engine-mock");
    assert!(p.exists(), "run cargo build -p presto-engine-mock first");
    p
}

pub fn demo_backend(faults: &[&str]) -> (Backend, TempDir) {
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempfile::Builder::new().prefix("pb").tempdir_in("/tmp").unwrap();
    let rt = tmp.path().join("run");
    std::fs::create_dir(&rt).unwrap();
    std::fs::set_permissions(&rt, std::fs::Permissions::from_mode(0o700)).unwrap();
    let faults: Vec<String> = faults.iter().map(|s| s.to_string()).collect();
    let cfg = demo_config(&tmp.path().join("state"), &rt, mock_bin(), demo_extra(&faults)).unwrap();
    (Backend::start(cfg).unwrap(), tmp)
}

pub fn wait_state(b: &Backend, within: Duration, pred: impl Fn(&CoreState) -> bool) -> CoreState {
    let rx = b.state();
    let end = Instant::now() + within;
    loop {
        let s = rx.borrow().clone();
        if pred(&s) {
            return s;
        }
        assert!(Instant::now() < end, "timed out; last state: {s:?}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

pub fn ready(b: &Backend) -> CoreState {
    use presto_core::EngineStatus;
    wait_state(b, Duration::from_secs(10), |s| s.engine == EngineStatus::Ready)
}

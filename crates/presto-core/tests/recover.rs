mod common;
use common::*;
use presto_core::EngineStatus;
use presto_ipc::{Command, ErrorKind, FaultSpec, Outcome};
use std::time::Duration;

const T: Duration = Duration::from_secs(5);

fn ready(s: &presto_core::CoreState) -> bool {
    s.engine == EngineStatus::Ready
}

fn crashing(n: u32) -> Vec<String> {
    let _ = n;
    vec!["--fault".into(), "crash".into()]
}

#[tokio::test]
async fn crash_restarts() {
    let r = rig(|_| vec![]).await;
    let mut rx = r.core.state();
    wait_for(&mut rx, T, ready).await;
    r.core.mock(FaultSpec::Crash { after_ms: None }).await;
    wait_for(&mut rx, T, |s| s.restarts == 1).await;
    let s = wait_for(&mut rx, T, ready).await;
    assert_eq!(s.restarts, 1);
    assert_eq!(r.calls(), 2);
    r.core.shutdown().await;
}

#[tokio::test]
async fn hang_restarts() {
    let r = rig(|_| vec![]).await;
    let mut rx = r.core.state();
    wait_for(&mut rx, T, ready).await;
    r.core.mock(FaultSpec::Hang).await;
    wait_for(&mut rx, T, |s| s.restarts == 1).await;
    let s = wait_for(&mut rx, T, ready).await;
    assert_eq!(s.restarts, 1);
    r.core.shutdown().await;
}

#[tokio::test]
async fn gives_up_after_five_fast_failures() {
    let r = rig(crashing).await;
    let mut rx = r.core.state();
    let s = wait_for(&mut rx, T, |s| matches!(s.engine, EngineStatus::Failed { .. })).await;
    let EngineStatus::Failed { log_path, .. } = s.engine else { unreachable!() };
    assert!(!log_path.as_os_str().is_empty());
    assert_eq!(r.calls(), 5);
    let Outcome::Err { error } = r.core.command(Command::Play).await else { panic!("expected err") };
    assert_eq!(error.kind, ErrorKind::Unavailable);
    r.core.shutdown().await;
}

#[tokio::test]
async fn restart_engine_after_failed() {
    let r = rig(|n| if n < 5 { crashing(n) } else { vec![] }).await;
    let mut rx = r.core.state();
    wait_for(&mut rx, T, |s| matches!(s.engine, EngineStatus::Failed { .. })).await;
    r.core.restart_engine();
    wait_for(&mut rx, T, ready).await;
    r.core.shutdown().await;
}

#[tokio::test]
async fn shutdown_reaps_engine_and_pidfile() {
    let r = rig(|_| vec![]).await;
    let mut rx = r.core.state();
    wait_for(&mut rx, T, ready).await;
    let pidfile = r.dir.path().join("state/engine.pid");
    let pid = presto_core::paths::Pidfile::read(&pidfile).unwrap().expect("pidfile").pid;
    r.core.shutdown().await;
    assert!(!pidfile.exists());
    assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
}

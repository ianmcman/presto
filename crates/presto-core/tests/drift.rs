mod common;
use common::*;
use presto_core::EngineStatus;
use presto_ipc::{Command, Outcome};
use std::time::Duration;

const T: Duration = Duration::from_secs(5);

#[tokio::test]
async fn no_bridge_ready_is_drift() {
    let r = rig(|_| vec!["--bridge-missing".into()]).await;
    let mut rx = r.core.state();
    wait_for(&mut rx, Duration::from_secs(2), |s| matches!(s.engine, EngineStatus::Drift { .. })).await;
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert_eq!(r.calls(), 1);
    assert_eq!(rx.borrow().restarts, 0);
    r.core.shutdown().await;
}

#[tokio::test]
async fn missing_capability_is_drift() {
    let r = rig(|_| vec!["--bridge-caps".into(), "playback,queue".into()]).await;
    let mut rx = r.core.state();
    let s = wait_for(&mut rx, T, |s| matches!(s.engine, EngineStatus::Drift { .. })).await;
    let EngineStatus::Drift { reason, .. } = s.engine else { unreachable!() };
    assert!(reason.contains("api"), "{reason}");
    r.core.shutdown().await;
}

#[tokio::test]
async fn command_before_ready_is_delivered() {
    let r = rig(|_| vec![]).await;
    let out = r.core.command(Command::SetVolume { volume: 0.5 }).await;
    assert!(matches!(out, Outcome::Ok { .. }), "{out:?}");
    r.core.shutdown().await;
}

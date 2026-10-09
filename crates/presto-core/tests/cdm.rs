mod common;
use common::*;
use presto_core::EngineStatus;
use presto_ipc::CdmState;
use std::time::Duration;

fn args(d: &str, extra: &[&str]) -> Vec<String> {
    ["--cdm-delay-ms", d].iter().chain(extra).map(|s| s.to_string()).collect()
}

#[tokio::test]
async fn cdm_checking_defers_drift() {
    let r = rig(|_| args("2500", &[])).await;
    let mut rx = r.core.state();
    wait_for(&mut rx, Duration::from_secs(2), |s| s.cdm.as_ref().is_some_and(|c| c.state == CdmState::Checking)).await;
    let s = wait_for(&mut rx, Duration::from_secs(8), |s| {
        s.engine == EngineStatus::Ready || matches!(s.engine, EngineStatus::Drift { .. })
    })
    .await;
    assert_eq!(s.engine, EngineStatus::Ready);
    let c = s.cdm.unwrap();
    assert_eq!(c.state, CdmState::Ready);
    assert_eq!(c.version.as_deref(), Some("mock"));
    assert_eq!(r.calls(), 1);
    r.core.shutdown().await;
}

#[tokio::test]
async fn cdm_failed_is_reported() {
    let r = rig(|_| args("200", &["--cdm-fail"])).await;
    let mut rx = r.core.state();
    let s = wait_for(&mut rx, Duration::from_secs(3), |s| s.cdm.as_ref().is_some_and(|c| c.state == CdmState::Failed)).await;
    assert!(s.cdm.unwrap().message.unwrap().contains("mock"));
    tokio::time::sleep(Duration::from_millis(2500)).await;
    assert_eq!(rx.borrow().engine, EngineStatus::Starting);
    assert_eq!(r.calls(), 1);
    r.core.shutdown().await;
}

#[tokio::test]
async fn cdm_ready_rearms_drift() {
    let r = rig(|_| args("200", &["--bridge-missing"])).await;
    let mut rx = r.core.state();
    wait_for(&mut rx, Duration::from_secs(4), |s| matches!(s.engine, EngineStatus::Drift { .. })).await;
    r.core.shutdown().await;
}

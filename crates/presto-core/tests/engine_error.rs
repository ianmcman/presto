mod common;
use common::*;
use presto_core::EngineStatus;
use presto_ipc::{AuthState, Command, ErrorKind};
use std::time::Duration;

#[tokio::test(flavor = "multi_thread")]
async fn error_event_reaches_state() {
    let r = rig(|_| vec![]).await;
    let mut rx = r.core.state();
    let t = Duration::from_secs(5);
    wait_for(&mut rx, t, |s| s.engine == EngineStatus::Ready && s.auth == Some(AuthState::SignedIn)).await;
    r.core.command(Command::SetQueue { ids: vec!["s8".into()], start: 0, play: true }).await;
    let s = wait_for(&mut rx, t, |s| s.engine_errors == 1).await;
    assert_eq!(s.last_engine_error.unwrap().kind, ErrorKind::Upstream { status: 503 });
    r.core.shutdown().await;
}

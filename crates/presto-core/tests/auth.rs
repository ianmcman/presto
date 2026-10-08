mod common;
use common::*;
use presto_core::EngineStatus;
use presto_ipc::{ApiRequest, AuthState, Command, ErrorKind, FaultSpec, Outcome, PlayState};
use std::time::{Duration, Instant};

const T: Duration = Duration::from_secs(5);

fn is_auth_err(o: &Outcome) -> bool {
    matches!(o, Outcome::Err { error } if error.kind == ErrorKind::AuthExpired)
}

#[tokio::test]
async fn signed_out_blocks_and_fails_fast() {
    let r = rig(|_| vec!["--auth".into(), "signed_out".into()]).await;
    let mut rx = r.core.state();
    wait_for(&mut rx, T, |s| s.auth == Some(AuthState::SignedOut) && s.engine == EngineStatus::Ready).await;
    let t = Instant::now();
    let o = r.core.request(ApiRequest::get("/v1/me/library/playlists")).await;
    assert!(is_auth_err(&o), "{o:?}");
    assert!(t.elapsed() < Duration::from_millis(50));
    assert!(matches!(r.core.show_sign_in().await, Outcome::Ok { .. }));
    r.core.mock(FaultSpec::None).await;
    wait_for(&mut rx, T, |s| s.auth == Some(AuthState::SignedIn)).await;
    let o = r.core.request(ApiRequest::get("/v1/me/library/playlists")).await;
    assert!(matches!(o, Outcome::Ok { .. }), "{o:?}");
    r.core.shutdown().await;
}

#[tokio::test]
async fn expired_keeps_mirror_and_reauth_resumes() {
    let r = rig(|_| vec![]).await;
    let mut rx = r.core.state();
    wait_for(&mut rx, T, |s| s.auth == Some(AuthState::SignedIn)).await;
    let q = Command::SetQueue { ids: vec!["s1".into(), "s2".into()], start: 0, play: true };
    assert!(matches!(r.core.command(q).await, Outcome::Ok { .. }));
    wait_for(&mut rx, T, |s| s.player.state == PlayState::Playing).await;
    r.core.mock(FaultSpec::AuthExpired).await;
    let s = wait_for(&mut rx, T, |s| s.auth == Some(AuthState::Expired)).await;
    assert_eq!(s.queue.ids(), ["s1", "s2"]);
    let t = Instant::now();
    assert!(is_auth_err(&r.core.command(Command::Pause).await));
    assert!(t.elapsed() < Duration::from_millis(50));
    r.core.mock(FaultSpec::None).await;
    let s = wait_for(&mut rx, T, |s| s.auth == Some(AuthState::SignedIn) && s.engine == EngineStatus::Ready).await;
    assert_eq!(s.queue.ids(), ["s1", "s2"]);
    assert_eq!(s.player.state, PlayState::Playing);
    assert_eq!(s.restarts, 0);
    // traffic flows again
    assert!(matches!(r.core.command(Command::Pause).await, Outcome::Ok { .. }));
    r.core.shutdown().await;
}

mod common;

use common::*;
use presto_ipc::transport;
use presto_ipc::*;
use std::time::{Duration, Instant};

const S3: Duration = Duration::from_secs(3);

fn is_res(id: u64) -> impl Fn(&Frame) -> bool {
    move |f| matches!(f, Frame::Res { id: i, .. } if *i == id)
}
fn is_pong(seq: u64) -> impl Fn(&Frame) -> bool {
    move |f| matches!(f, Frame::Pong { seq: s } if *s == seq)
}
fn is_ok(f: &Frame) -> bool {
    matches!(
        f,
        Frame::Res {
            outcome: Outcome::Ok { .. },
            ..
        }
    )
}
fn is_auth(state: AuthState) -> impl Fn(&Frame) -> bool {
    move |f| matches!(f, Frame::Evt { evt: Event::Auth { state: s } } if *s == state)
}
fn err_kind(f: Frame) -> ErrorKind {
    match f {
        Frame::Res {
            outcome: Outcome::Err { error },
            ..
        } => error.kind,
        other => panic!("expected err res, got {other:?}"),
    }
}
fn playlists(id: u64) -> Frame {
    Frame::Req {
        id,
        req: ApiRequest::get("/v1/me/library/playlists"),
    }
}
async fn mock(h: &mut Harness, id: u64, fault: FaultSpec) {
    send(h, Frame::Mock { id, fault }).await;
}
/// Asserts silence: nothing arrives for 3 s and the stream stays open.
async fn assert_silent(h: &mut Harness) {
    let r = tokio::time::timeout(S3, transport::recv(&mut h.conn)).await;
    assert!(r.is_err(), "expected silence, got {r:?}");
    assert!(h.child.try_wait().unwrap().is_none(), "mock exited");
}

#[tokio::test]
async fn hang_flag() {
    let mut h = start(&["--fault", "hang"]).await;
    send(&mut h, Frame::Ping { seq: 1 }).await;
    assert_silent(&mut h).await;
    mock(&mut h, 1, FaultSpec::None).await;
    expect(&mut h, T, is_res(1)).await;
    send(&mut h, Frame::Ping { seq: 2 }).await;
    expect(&mut h, T, is_pong(2)).await;
}

#[tokio::test]
async fn hang_live() {
    let mut h = start(&[]).await;
    mock(&mut h, 1, FaultSpec::Hang).await;
    expect(&mut h, T, is_res(1)).await;
    send(&mut h, Frame::Ping { seq: 1 }).await;
    assert_silent(&mut h).await;
    mock(&mut h, 2, FaultSpec::None).await;
    expect(&mut h, T, is_res(2)).await;
    send(&mut h, Frame::Ping { seq: 2 }).await;
    expect(&mut h, T, is_pong(2)).await;
}

#[tokio::test]
async fn crash_flag_delayed() {
    let mut h = start(&["--fault", "crash@300"]).await;
    let st = wait_exit(&mut h, S3).await;
    assert_eq!(st.code(), Some(101));
    while let Ok(Some(_)) = transport::recv(&mut h.conn).await {} // drain buffered frames to EOF
}

#[tokio::test]
async fn crash_live() {
    let mut h = start(&[]).await;
    mock(&mut h, 1, FaultSpec::Crash { after_ms: None }).await;
    assert_eq!(wait_exit(&mut h, S3).await.code(), Some(101));
}

#[tokio::test]
async fn auth_expired_flag() {
    let mut h = start(&["--fault", "auth_expired"]).await;
    expect(&mut h, T, is_auth(AuthState::Expired)).await;
    send(&mut h, playlists(1)).await;
    assert_eq!(
        err_kind(expect(&mut h, T, is_res(1)).await),
        ErrorKind::AuthExpired
    );
    send(
        &mut h,
        Frame::Cmd {
            id: 2,
            cmd: Command::Play,
        },
    )
    .await;
    assert_eq!(
        err_kind(expect(&mut h, T, is_res(2)).await),
        ErrorKind::AuthExpired
    );
    send(&mut h, Frame::Ping { seq: 1 }).await;
    expect(&mut h, T, is_pong(1)).await;
}

#[tokio::test]
async fn auth_expired_live_and_clear() {
    let mut h = start(&[]).await;
    mock(&mut h, 1, FaultSpec::AuthExpired).await;
    expect(&mut h, T, is_auth(AuthState::Expired)).await;
    expect(&mut h, T, is_res(1)).await;
    send(&mut h, playlists(2)).await;
    assert_eq!(
        err_kind(expect(&mut h, T, is_res(2)).await),
        ErrorKind::AuthExpired
    );
    mock(&mut h, 3, FaultSpec::None).await;
    expect(&mut h, T, is_auth(AuthState::SignedIn)).await;
    expect(&mut h, T, is_res(3)).await;
    send(&mut h, playlists(4)).await;
    assert!(is_ok(&expect(&mut h, T, is_res(4)).await));
}

#[tokio::test]
async fn slow_flag() {
    let mut h = start(&["--fault", "slow=1500"]).await;
    let t0 = Instant::now();
    send(
        &mut h,
        Frame::Req {
            id: 1,
            req: ApiRequest::get("/v1/me/library/songs"),
        },
    )
    .await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    send(&mut h, Frame::Ping { seq: 1 }).await;
    // is_pong skips anything before it; a res arriving first would be skipped silently,
    // so check the res timing below as well
    expect(&mut h, T, is_pong(1)).await;
    assert!(t0.elapsed() < Duration::from_millis(600), "pong late");
    expect(&mut h, T, is_res(1)).await;
    assert!(
        t0.elapsed() >= Duration::from_millis(1500),
        "res not delayed"
    );
}

#[tokio::test]
async fn slow_live() {
    let mut h = start(&[]).await;
    let t = Instant::now();
    mock(&mut h, 1, FaultSpec::Slow { delay_ms: 800 }).await;
    expect(&mut h, T, is_res(1)).await;
    assert!(t.elapsed() < Duration::from_millis(300), "ack delayed");
    let t = Instant::now();
    send(&mut h, playlists(2)).await;
    expect(&mut h, T, is_res(2)).await;
    assert!(t.elapsed() >= Duration::from_millis(800), "res not delayed");
}

#[tokio::test]
async fn bad_fault_flag() {
    let st = tokio::process::Command::new(env!("CARGO_BIN_EXE_presto-engine-mock"))
        .args(["--socket", "/tmp/x", "--fault", "bogus"])
        .kill_on_drop(true)
        .status()
        .await
        .unwrap();
    assert!(!st.success());
}

#[tokio::test]
async fn signed_out_start() {
    let mut h = start(&["--auth", "signed_out"]).await;
    expect(&mut h, T, is_auth(AuthState::SignedOut)).await;
    send(
        &mut h,
        Frame::Cmd {
            id: 1,
            cmd: Command::Play,
        },
    )
    .await;
    assert_eq!(
        err_kind(expect(&mut h, T, is_res(1)).await),
        ErrorKind::AuthExpired
    );
    send(
        &mut h,
        Frame::Cmd {
            id: 2,
            cmd: Command::ShowWindow { show: true },
        },
    )
    .await;
    assert!(is_ok(&expect(&mut h, T, is_res(2)).await));
    mock(&mut h, 3, FaultSpec::None).await;
    expect(&mut h, T, is_auth(AuthState::SignedIn)).await;
    expect(&mut h, T, is_res(3)).await;
    let q = Command::SetQueue {
        ids: vec!["s1".into()],
        start: 0,
        play: true,
    };
    send(&mut h, Frame::Cmd { id: 4, cmd: q }).await;
    assert!(is_ok(&expect(&mut h, T, is_res(4)).await));
    send(
        &mut h,
        Frame::Cmd {
            id: 5,
            cmd: Command::Play,
        },
    )
    .await;
    assert!(is_ok(&expect(&mut h, T, is_res(5)).await));
}

#[tokio::test]
async fn bridge_missing_is_silent_but_answers_ping() {
    let mut h = start(&["--bridge-missing"]).await;
    let r = tokio::time::timeout(Duration::from_secs(1), transport::recv(&mut h.conn)).await;
    assert!(r.is_err(), "expected silence, got {r:?}");
    send(&mut h, Frame::Ping { seq: 1 }).await;
    expect(&mut h, T, is_pong(1)).await;
}

#[tokio::test]
async fn bridge_caps_flag() {
    let mut h = start(&["--bridge-caps", "playback,queue"]).await;
    let f = expect(&mut h, T, |f| matches!(f, Frame::Evt { .. })).await;
    assert!(matches!(
        f,
        Frame::Evt { evt: Event::BridgeReady { capabilities, .. } } if capabilities == ["playback", "queue"]
    ));
}

#[tokio::test]
async fn rate_limited_flag_and_clear() {
    let mut h = start(&["--fault", "rate_limited=250"]).await;
    send(&mut h, playlists(1)).await;
    assert_eq!(
        err_kind(expect(&mut h, T, is_res(1)).await),
        ErrorKind::RateLimited {
            retry_after_ms: Some(250)
        }
    );
    send(
        &mut h,
        Frame::Cmd {
            id: 2,
            cmd: Command::SetQueue {
                ids: vec!["s1".into()],
                start: 0,
                play: false,
            },
        },
    )
    .await;
    assert!(is_ok(&expect(&mut h, T, is_res(2)).await));
    mock(&mut h, 3, FaultSpec::None).await;
    expect(&mut h, T, is_res(3)).await;
    send(&mut h, playlists(4)).await;
    assert!(is_ok(&expect(&mut h, T, is_res(4)).await));
}

#[tokio::test]
async fn signed_out_live() {
    let mut h = start(&[]).await;
    mock(&mut h, 1, FaultSpec::SignedOut).await;
    expect(&mut h, T, is_auth(AuthState::SignedOut)).await;
    expect(&mut h, T, is_res(1)).await;
    send(&mut h, playlists(2)).await;
    assert_eq!(
        err_kind(expect(&mut h, T, is_res(2)).await),
        ErrorKind::AuthExpired
    );
}

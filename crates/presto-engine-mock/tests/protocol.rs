mod common;

use common::*;
use presto_ipc::*;
use std::time::Duration;

fn cmd(id: u64, cmd: Command) -> Frame {
    Frame::Cmd { id, cmd }
}
fn set_queue(ids: &[&str]) -> Command {
    Command::SetQueue {
        ids: ids.iter().map(|s| s.to_string()).collect(),
        start: 0,
    }
}
fn is_res(id: u64) -> impl Fn(&Frame) -> bool {
    move |f| matches!(f, Frame::Res { id: i, .. } if *i == id)
}
fn res_data(f: Frame) -> serde_json::Value {
    match f {
        Frame::Res {
            outcome: Outcome::Ok { data },
            ..
        } => data,
        other => panic!("expected ok res, got {other:?}"),
    }
}
fn res_err(f: Frame) -> ErrorKind {
    match f {
        Frame::Res {
            outcome: Outcome::Err { error },
            ..
        } => error.kind,
        other => panic!("expected err res, got {other:?}"),
    }
}
fn get(id: u64, path: &str, query: &[(&str, &str)]) -> Frame {
    let mut req = ApiRequest::get(path);
    for (k, v) in query {
        req.query.insert(k.to_string(), v.to_string());
    }
    Frame::Req { id, req }
}

#[tokio::test]
async fn handshake_and_ping() {
    let mut h = start(&[]).await;
    let eh = h.engine_hello.clone();
    assert_eq!(eh.role, Role::Engine);
    assert_eq!(eh.proto.major, 1);
    for c in [caps::PLAYBACK, caps::QUEUE, caps::API, caps::MOCK] {
        assert!(eh.has(c), "missing {c}");
    }
    assert!(eh.engine.unwrap().starts_with("presto-engine-mock"));
    expect(&mut h, T, |f| {
        matches!(
            f,
            Frame::Evt {
                evt: Event::Auth {
                    state: AuthState::SignedIn
                }
            }
        )
    })
    .await;
    send(&mut h, Frame::Ping { seq: 9 }).await;
    expect(&mut h, T, |f| matches!(f, Frame::Pong { seq: 9 })).await;
}

#[tokio::test]
async fn playback_flow() {
    let mut h = start(&[]).await;
    send(&mut h, cmd(1, Command::Play)).await;
    assert_eq!(
        res_err(expect(&mut h, T, is_res(1)).await),
        ErrorKind::Unavailable
    );

    send(&mut h, cmd(2, set_queue(&["s1", "s2"]))).await;
    let q = expect(&mut h, T, |f| {
        matches!(
            f,
            Frame::Evt {
                evt: Event::QueueChanged { .. }
            }
        )
    })
    .await;
    let Frame::Evt {
        evt: Event::QueueChanged { rev, items, index },
    } = q
    else {
        unreachable!()
    };
    assert_eq!((items.len(), index), (2, Some(0)));
    res_data(expect(&mut h, T, is_res(2)).await);

    let pos = |f: &Frame| match f {
        Frame::Evt {
            evt: Event::Progress { position_ms, .. },
        } => Some(*position_ms),
        _ => None,
    };
    let a = pos(&expect(&mut h, T, |f| pos(f).is_some()).await).unwrap();
    // ticks are 500 ms apart; wait for a later one
    let b = pos(&expect(&mut h, T, |f| pos(f).is_some_and(|p| p > a)).await).unwrap();
    assert!(b > a);

    send(&mut h, cmd(3, Command::Pause)).await;
    expect(&mut h, T, |f| {
        matches!(
            f,
            Frame::Evt {
                evt: Event::PlaybackState {
                    state: PlayState::Paused,
                    ..
                }
            }
        )
    })
    .await;
    res_data(expect(&mut h, T, is_res(3)).await);
    send(&mut h, cmd(4, Command::Play)).await;
    expect(&mut h, T, |f| {
        matches!(
            f,
            Frame::Evt {
                evt: Event::PlaybackState {
                    state: PlayState::Playing,
                    ..
                }
            }
        )
    })
    .await;

    let dur = catalog_dur_s1();
    send(&mut h, cmd(5, Command::Seek { ms: dur - 1000 })).await;
    let p = expect(&mut h, T, |f| pos(f).is_some_and(|p| p >= dur - 1500)).await;
    assert!(pos(&p).unwrap() >= dur - 1500);
    let n = expect(&mut h, Duration::from_secs(4), |f| {
        matches!(
            f,
            Frame::Evt {
                evt: Event::QueueChanged { index: Some(1), .. }
            }
        )
    })
    .await;
    let Frame::Evt {
        evt: Event::QueueChanged { rev: r2, .. },
    } = n
    else {
        unreachable!()
    };
    assert_eq!(r2, rev + 1);
}

fn catalog_dur_s1() -> u64 {
    183000
}

#[tokio::test]
async fn api_routes() {
    let mut h = start(&[]).await;
    send(&mut h, get(1, "/v1/me/library/playlists", &[])).await;
    let d = res_data(expect(&mut h, T, is_res(1)).await);
    assert!(!d["data"].as_array().unwrap().is_empty());

    send(&mut h, get(2, "/v1/catalog/us/search", &[("term", "neon")])).await;
    let d = res_data(expect(&mut h, T, is_res(2)).await);
    assert!(!d["results"]["songs"]["data"].as_array().unwrap().is_empty());

    send(&mut h, get(3, "/v1/nope", &[])).await;
    assert_eq!(
        res_err(expect(&mut h, T, is_res(3)).await),
        ErrorKind::NotFound
    );
}

#[tokio::test]
async fn version_handling() {
    let mut h = start_raw(&[]).await;
    let mut bad = Hello::new(Role::Presto, &[], None);
    bad.proto = ProtoVersion { major: 2, minor: 0 };
    send(&mut h, Frame::Hello(bad)).await;
    assert_eq!(wait_exit(&mut h, T).await.code(), Some(2));

    let mut h = start_raw(&[]).await;
    let mut minor = Hello::new(Role::Presto, &[], None);
    minor.proto = ProtoVersion { major: 1, minor: 9 };
    send(&mut h, Frame::Hello(minor)).await;
    send(&mut h, Frame::Ping { seq: 1 }).await;
    expect(&mut h, T, |f| matches!(f, Frame::Pong { seq: 1 })).await;
}

#[tokio::test]
async fn exits_zero_on_eof() {
    let h = start(&[]).await;
    let Harness {
        conn, mut child, ..
    } = h;
    drop(conn);
    let st = tokio::time::timeout(T, child.wait())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(st.code(), Some(0));
}

#![allow(dead_code)]

use presto_ipc::transport::{self, Conn};
use presto_ipc::{Frame, Hello, Role};
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

pub const T: Duration = Duration::from_secs(5);

pub struct Harness {
    pub conn: Conn,
    pub child: tokio::process::Child,
    pub engine_hello: Hello,
    _dir: tempfile::TempDir,
}

/// Spawns the mock and reads its hello; does not answer it.
pub async fn start_raw(args: &[&str]) -> Harness {
    // short path: unix socket paths are limited to ~108 bytes
    let dir = tempfile::Builder::new()
        .prefix("p")
        .tempdir_in("/tmp")
        .unwrap();
    let sock = dir.path().join("e.sock");
    let listener = transport::bind(&sock).unwrap();
    let child = Command::new(env!("CARGO_BIN_EXE_presto-engine-mock"))
        .arg("--socket")
        .arg(&sock)
        .args(args)
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let (stream, _) = timeout(T, listener.accept())
        .await
        .expect("accept timed out")
        .unwrap();
    let mut conn = transport::framed(stream);
    let engine_hello = match timeout(T, transport::recv(&mut conn))
        .await
        .expect("hello timed out")
    {
        Ok(Some(Frame::Hello(h))) => h,
        other => panic!("expected engine hello, got {other:?}"),
    };
    Harness {
        conn,
        child,
        engine_hello,
        _dir: dir,
    }
}

pub async fn start(args: &[&str]) -> Harness {
    let mut h = start_raw(args).await;
    send(&mut h, Frame::Hello(Hello::new(Role::Presto, &[], None))).await;
    h
}

pub async fn send(h: &mut Harness, f: Frame) {
    transport::send(&mut h.conn, &f).await.unwrap();
}

pub async fn expect(h: &mut Harness, within: Duration, pred: impl Fn(&Frame) -> bool) -> Frame {
    let mut skipped = vec![];
    let r = timeout(within, async {
        loop {
            match transport::recv(&mut h.conn).await {
                Ok(Some(f)) if pred(&f) => return f,
                Ok(Some(f)) => skipped.push(format!("{f:?}").chars().take(60).collect::<String>()),
                other => panic!("connection ended: {other:?}; skipped {skipped:?}"),
            }
        }
    })
    .await;
    match r {
        Ok(f) => f,
        Err(_) => panic!("timed out waiting for frame"),
    }
}

pub async fn wait_exit(h: &mut Harness, within: Duration) -> std::process::ExitStatus {
    timeout(within, h.child.wait())
        .await
        .expect("mock did not exit")
        .unwrap()
}

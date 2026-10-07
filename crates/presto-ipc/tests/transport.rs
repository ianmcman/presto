use futures_util::SinkExt;
use presto_ipc::Frame;
use presto_ipc::transport::*;
use std::os::unix::fs::PermissionsExt;

fn tmp() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("p")
        .tempdir_in("/tmp")
        .unwrap()
}

fn mode(p: &std::path::Path) -> u32 {
    std::fs::metadata(p).unwrap().permissions().mode() & 0o777
}

#[tokio::test]
async fn bind_modes_and_stale() {
    let d = tmp();
    let p = d.path().join("s/engine.sock");
    let l = bind(&p).unwrap();
    assert_eq!(mode(p.parent().unwrap()), 0o700);
    assert_eq!(mode(&p), 0o600);
    drop(l);
    bind(&p).unwrap();
}

#[tokio::test]
async fn frame_and_bad_line() {
    let d = tmp();
    let p = d.path().join("s/engine.sock");
    let l = bind(&p).unwrap();
    let mut c = connect(&p).await.unwrap();
    let mut a = framed(l.accept().await.unwrap().0);

    send(&mut c, &Frame::Ping { seq: 42 }).await.unwrap();
    assert_eq!(recv(&mut a).await.unwrap(), Some(Frame::Ping { seq: 42 }));

    c.send("not json".to_string()).await.unwrap();
    assert!(matches!(recv(&mut a).await, Err(TransportError::Json(_))));
    send(&mut c, &Frame::Pong { seq: 1 }).await.unwrap();
    assert_eq!(recv(&mut a).await.unwrap(), Some(Frame::Pong { seq: 1 }));
}

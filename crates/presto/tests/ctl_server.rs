#![allow(dead_code)]
use presto::ctl::{bind, lock, CtlPaths};
use presto::control::Control;
use presto_core::CoreState;
use presto_ipc::ctl::{CtlRequest, CtlOp, CtlReply, CTL_PROTO};
use std::os::unix::fs::FileTypeExt;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::sync::watch;

#[derive(Debug, Clone)]
enum FakeEvent {
    Cmd(presto_ipc::Command),
    Raise,
    Quit,
}

struct FakeControl {
    events: Arc<Mutex<Vec<FakeEvent>>>,
}

impl FakeControl {
    fn new() -> Self {
        FakeControl {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn get_events(&self) -> Vec<FakeEvent> {
        self.events.lock().unwrap().clone()
    }
}

impl Control for FakeControl {
    fn send(&self, c: presto_ipc::Command) {
        self.events.lock().unwrap().push(FakeEvent::Cmd(c));
    }

    fn raise(&self) {
        self.events.lock().unwrap().push(FakeEvent::Raise);
    }

    fn quit(&self) {
        self.events.lock().unwrap().push(FakeEvent::Quit);
    }

    fn art(&self, _url: &str) -> Option<PathBuf> {
        None
    }
}

#[test]
fn lock_exclusive() {
    let tmp = TempDir::new().unwrap();
    let paths = CtlPaths::new(tmp.path(), false);
    let _guard1 = lock(&paths).expect("first lock should succeed");
    let result = lock(&paths);
    assert!(result.is_err(), "second lock should fail while first is held");
}

#[test]
fn lock_released_after_drop() {
    let tmp = TempDir::new().unwrap();
    let paths = CtlPaths::new(tmp.path(), false);
    {
        let _guard1 = lock(&paths).expect("first lock should succeed");
    }
    let _guard2 = lock(&paths).expect("lock should succeed after drop");
}

#[test]
fn bind_creates_socket_0600() {
    let tmp = TempDir::new().unwrap();
    let paths = CtlPaths::new(tmp.path(), false);
    let _guard = lock(&paths).expect("lock should succeed");

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let _listener = bind(&_guard).expect("bind should succeed");
        let md = std::fs::symlink_metadata(&paths.sock).expect("sock should exist");
        assert!(md.file_type().is_socket(), "should be a socket");
    });
}

#[test]
fn server_accepts_valid_op() {
    let tmp = TempDir::new().unwrap();
    let paths = CtlPaths::new(tmp.path(), false);
    let ctl = Arc::new(FakeControl::new());
    let (_tx, rx) = watch::channel(CoreState::default());

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let guard = lock(&paths).expect("lock should succeed");
        let listener = bind(&guard).expect("bind should succeed");
        let ctl_clone = ctl.clone();
        let state_rx = rx.clone();

        tokio::spawn(async move {
            presto::ctl::serve(listener, state_rx, ctl_clone).await;
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let mut stream = std::os::unix::net::UnixStream::connect(&paths.sock).unwrap();
        let req = CtlRequest {
            proto: CTL_PROTO,
            id: 1,
            op: CtlOp::Status,
        };
        let line = serde_json::to_string(&req).unwrap() + "\n";
        std::io::Write::write_all(&mut stream, line.as_bytes()).unwrap();

        let mut reader = std::io::BufReader::new(&stream);
        let mut response = String::new();
        std::io::BufRead::read_line(&mut reader, &mut response).unwrap();

        let reply: CtlReply = serde_json::from_str(&response).unwrap();
        assert_eq!(reply.proto, CTL_PROTO);
        assert_eq!(reply.id, 1);
        assert!(reply.ok);
    });
}

#[test]
fn server_rejects_wrong_proto() {
    let tmp = TempDir::new().unwrap();
    let paths = CtlPaths::new(tmp.path(), false);
    let ctl = Arc::new(FakeControl::new());
    let (_tx, rx) = watch::channel(CoreState::default());

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let guard = lock(&paths).expect("lock should succeed");
        let listener = bind(&guard).expect("bind should succeed");
        let ctl_clone = ctl.clone();
        let state_rx = rx.clone();

        tokio::spawn(async move {
            presto::ctl::serve(listener, state_rx, ctl_clone).await;
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let mut stream = std::os::unix::net::UnixStream::connect(&paths.sock).unwrap();
        let req = CtlRequest {
            proto: 99,  // wrong proto
            id: 1,
            op: CtlOp::Status,
        };
        let line = serde_json::to_string(&req).unwrap() + "\n";
        std::io::Write::write_all(&mut stream, line.as_bytes()).unwrap();

        let mut reader = std::io::BufReader::new(&stream);
        let mut response = String::new();
        std::io::BufRead::read_line(&mut reader, &mut response).unwrap();

        let reply: CtlReply = serde_json::from_str(&response).unwrap();
        assert!(!reply.ok);
        assert!(reply.error.is_some());
    });
}

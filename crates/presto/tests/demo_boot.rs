mod common;
use common::*;
use presto_core::data::view::{LibKind, Sort, ViewKey};
use presto_ipc::AuthState;
use std::time::Duration;

fn wait_rx<T: Clone>(rx: &tokio::sync::watch::Receiver<T>, pred: impl Fn(&T) -> bool) -> T {
    let end = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        let v = rx.borrow().clone();
        if pred(&v) {
            return v;
        }
        assert!(std::time::Instant::now() < end, "timed out");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn demo_boot() {
    let (b, _t) = demo_backend(&[]);
    let s = wait_state(&b, Duration::from_secs(10), |s| {
        s.engine == presto_core::EngineStatus::Ready && s.auth == Some(AuthState::SignedIn)
    });
    assert_eq!(s.auth, Some(AuthState::SignedIn));
    let shelves = wait_rx(&b.data().shelves(), |l| l.items.len() >= 3);
    assert_eq!(shelves.items.len(), 3);
    let lib = wait_rx(&b.data().list(ViewKey::Library { kind: LibKind::Songs, sort: Sort::Default }), |l| l.items.len() >= 100);
    assert_eq!(lib.items.len(), 100);
    assert!(lib.has_more);
    b.shutdown();
}

#[test]
fn demo_fault_passthrough() {
    let (b, _t) = demo_backend(&["auth_expired"]);
    wait_state(&b, Duration::from_secs(10), |s| s.auth == Some(AuthState::Expired));
    b.shutdown();
}

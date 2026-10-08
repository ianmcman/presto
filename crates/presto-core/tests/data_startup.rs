mod common;
use common::*;
use presto_core::data::DataHandle;
use presto_core::data::view::{ListState, Phase, ViewKey};
use presto_core::paths::Paths;
use presto_core::EngineStatus;
use std::time::Duration;
use tokio::sync::watch;

const T: Duration = Duration::from_secs(5);

async fn until<X: Clone>(rx: &mut watch::Receiver<ListState<X>>) -> ListState<X> {
    tokio::time::timeout(T, rx.wait_for(|s| !s.items.is_empty() && s.phase == Phase::Idle))
        .await
        .expect("view never loaded")
        .unwrap()
        .clone()
}

/// Home opens two views before the engine is ready; both must load on their own.
#[tokio::test(flavor = "multi_thread")]
async fn views_opened_before_ready_load_without_retry() {
    let r = rig(|_| vec![]).await;
    let dir = tempfile::tempdir().unwrap();
    let d = DataHandle::new(r.core.clone(), &Paths::under(dir.path().join("data"))).unwrap();
    assert_ne!(r.core.state().borrow().engine, EngineStatus::Ready, "engine already ready: test cannot reproduce");
    let mut recent = d.list(ViewKey::RecentlyPlayed);
    let mut shelves = d.shelves();
    let (a, b) = (until(&mut recent).await, until(&mut shelves).await);
    assert!(a.error.is_none() && b.error.is_none());
    r.core.shutdown().await;
}

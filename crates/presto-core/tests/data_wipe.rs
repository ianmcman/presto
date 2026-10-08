mod common;
use common::*;
use presto_core::data::DataHandle;
use presto_core::data::artwork::file_for;
use presto_core::data::models::Item;
use presto_core::data::view::{LibKind, ListState, Phase, Sort, ViewKey};
use presto_core::paths::Paths;
use presto_core::EngineStatus;
use presto_ipc::{AuthState, FaultSpec};
use std::time::Duration;
use tokio::sync::watch;

const T: Duration = Duration::from_secs(5);

fn songs() -> ViewKey {
    ViewKey::Library { kind: LibKind::Songs, sort: Sort::Default }
}

fn args(a: &[&str]) -> impl Fn(u32) -> Vec<String> + Send + Sync + 'static {
    let v: Vec<String> = ["--library-songs", "300"].iter().chain(a).map(|s| s.to_string()).collect();
    move |_| v.clone()
}

async fn up(r: &Rig, paths: &Paths, auth: AuthState) -> DataHandle {
    let mut rx = r.core.state();
    wait_for(&mut rx, T, |s| s.engine == EngineStatus::Ready && s.auth == Some(auth)).await;
    DataHandle::new(r.core.clone(), paths).unwrap()
}

async fn until<X: Clone>(rx: &mut watch::Receiver<ListState<X>>, pred: impl Fn(&ListState<X>) -> bool) -> ListState<X> {
    tokio::time::timeout(T, rx.wait_for(|s| pred(s))).await.expect("timed out").unwrap().clone()
}

async fn loaded(d: &DataHandle) -> watch::Receiver<ListState<Item>> {
    let mut rx = d.list(songs());
    until(&mut rx, |s| s.items.len() == 100 && s.phase == Phase::Idle).await;
    rx
}

/// Integration tests cannot reach the http artwork stub (test-only allowance), so seed the file directly.
fn seed_art(paths: &Paths, url: &str) {
    let f = file_for(&paths.artwork, url);
    std::fs::create_dir_all(f.parent().unwrap()).unwrap();
    std::fs::write(f, [7u8; 1000]).unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn startup_signed_out_keeps_cache() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path().join("data"));
    let a = rig(args(&[])).await;
    let da = up(&a, &paths, AuthState::SignedIn).await;
    loaded(&da).await;
    drop(da);
    a.core.shutdown().await;

    let b = rig(args(&["--auth", "signed_out"])).await;
    let db = up(&b, &paths, AuthState::SignedOut).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(db.store().page_count() > 0);
    let s = until(&mut db.list(songs()), |s| !s.items.is_empty()).await;
    assert!(s.from_cache);
    assert_eq!(s.items.len(), 100);
    b.core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn expired_keeps_cache() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path().join("data"));
    let r = rig(args(&[])).await;
    let d = up(&r, &paths, AuthState::SignedIn).await;
    let rx = loaded(&d).await;
    r.core.mock(FaultSpec::AuthExpired).await;
    wait_for(&mut r.core.state(), T, |s| s.auth == Some(AuthState::Expired)).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(d.store().page_count() > 0);
    assert_eq!(rx.borrow().items.len(), 100);
    r.core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn sign_out_wipes() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path().join("data"));
    paths.prepare().unwrap();
    seed_art(&paths, "https://is1.mzstatic.com/a.jpg");
    let r = rig(args(&[])).await;
    let d = up(&r, &paths, AuthState::SignedIn).await;
    let mut rx = loaded(&d).await;
    let acct = d.client().account_key().unwrap();
    d.store().push_search(&acct, "abba", 1);
    assert!(d.art().total_bytes() > 0);
    assert!(d.store().get_meta("storefront").is_some());

    r.core.mock(FaultSpec::SignedOut).await;
    until(&mut rx, |s| s.items.is_empty()).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(d.store().page_count(), 0);
    assert!(d.store().search_history(&acct).is_empty());
    assert_eq!(d.art().total_bytes(), 0);
    assert!(d.store().get_meta("storefront").is_none());
    r.core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn clear_cache_wipes_and_reloads() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path().join("data"));
    let r = rig(args(&[])).await;
    let d = up(&r, &paths, AuthState::SignedIn).await;
    let mut rx = loaded(&d).await;
    d.clear_cache();
    until(&mut rx, |s| s.items.is_empty()).await;
    let s = until(&mut rx, |s| s.items.len() == 100 && s.phase == Phase::Idle).await;
    assert!(!s.from_cache);
    assert!(d.store().page_count() > 0);
    r.core.shutdown().await;
}

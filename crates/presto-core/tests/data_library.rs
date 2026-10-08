mod common;
use common::*;
use presto_core::data::DataHandle;
use presto_core::data::models::Item;
use presto_core::data::store::{Store, now_ms, req_key};
use presto_core::data::view::{LibKind, ListState, Phase, Sort, ViewKey};
use presto_core::paths::Paths;
use presto_core::EngineStatus;
use presto_ipc::AuthState;
use std::time::Duration;
use tokio::sync::watch;

const T: Duration = Duration::from_secs(5);

fn songs() -> ViewKey {
    ViewKey::Library { kind: LibKind::Songs, sort: Sort::Default }
}

fn args(a: &[&str]) -> impl Fn(u32) -> Vec<String> + Send + Sync + 'static {
    let v: Vec<String> = a.iter().map(|s| s.to_string()).collect();
    move |_| v.clone()
}

async fn ready(r: &Rig, paths: &Paths) -> DataHandle {
    let mut rx = r.core.state();
    wait_for(&mut rx, T, |s| s.engine == EngineStatus::Ready && s.auth == Some(AuthState::SignedIn)).await;
    DataHandle::new(r.core.clone(), paths).unwrap()
}

async fn until<X: Clone>(rx: &mut watch::Receiver<ListState<X>>, pred: impl Fn(&ListState<X>) -> bool) -> ListState<X> {
    tokio::time::timeout(T, rx.wait_for(|s| pred(s))).await.expect("timed out").unwrap().clone()
}

fn name(s: &ListState<Item>) -> &str {
    &s.items[0].name
}

#[tokio::test(flavor = "multi_thread")]
async fn lazy_paging_10k() {
    let r = rig(args(&["--library-songs", "10000"])).await;
    let dir = tempfile::tempdir().unwrap();
    let d = ready(&r, &Paths::under(dir.path().join("data"))).await;
    let mut rx = d.list(songs());
    let s = until(&mut rx, |s| s.items.len() == 100 && s.phase == Phase::Idle).await;
    assert_eq!((s.total, s.has_more), (Some(10000), true));
    for n in 2..=4 {
        d.load_more(&songs());
        until(&mut rx, |s| s.items.len() == n * 100 && s.phase == Phase::Idle).await;
    }
    let s = rx.borrow().clone();
    for (i, it) in s.items.iter().enumerate() {
        assert_eq!(it.id, format!("i.{i:05}"));
    }
    assert_eq!(d.store().page_count(), 4);
    r.core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn cache_then_revalidate() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path().join("data"));
    let a = rig(args(&[])).await;
    let da = ready(&a, &paths).await;
    until(&mut da.list(songs()), |s| !s.items.is_empty() && s.phase == Phase::Idle).await;
    let acct = format!("us:{}", paths.install_id().unwrap());
    drop(da);
    a.core.shutdown().await;

    let st = Store::open(&paths.db).unwrap();
    let rk = req_key(&songs().request(0));
    let mut v: serde_json::Value = serde_json::from_slice(&st.get_page(&acct, &rk, 0).unwrap().body).unwrap();
    v["data"][0]["attributes"]["name"] = "Tampered".into();
    st.put_page(&acct, &rk, 0, now_ms() - 7_200_000, &serde_json::to_vec(&v).unwrap());
    drop(st);

    let b = rig(args(&[])).await;
    let db = ready(&b, &paths).await;
    let mut rx = db.list(songs());
    let s = until(&mut rx, |s| s.from_cache && !s.items.is_empty()).await;
    assert_eq!(name(&s), "Tampered");
    let s = until(&mut rx, |s| !s.from_cache && s.phase == Phase::Idle).await;
    assert_ne!(name(&s), "Tampered");
    assert!(s.updated_at.is_some());
    b.core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn fresh_cache_not_refetched() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path().join("data"));
    let a = rig(args(&[])).await;
    let da = ready(&a, &paths).await;
    until(&mut da.list(songs()), |s| !s.items.is_empty() && s.phase == Phase::Idle).await;
    drop(da);
    a.core.shutdown().await;

    let b = rig(args(&[])).await;
    let db = ready(&b, &paths).await;
    let mut rx = db.list(songs());
    let first = until(&mut rx, |s| !s.items.is_empty()).await;
    assert!(first.from_cache);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let now = rx.borrow().clone();
    assert!(now.from_cache && now.phase == Phase::Idle);
    assert_eq!(now.fetched_at, first.fetched_at);
    b.core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn refresh_bypasses_ttl() {
    let r = rig(args(&[])).await;
    let dir = tempfile::tempdir().unwrap();
    let d = ready(&r, &Paths::under(dir.path().join("data"))).await;
    let mut rx = d.list(songs());
    let s0 = until(&mut rx, |s| !s.items.is_empty() && s.phase == Phase::Idle).await;
    tokio::time::sleep(Duration::from_millis(20)).await;
    d.refresh(&songs());
    let s1 = until(&mut rx, |s| s.fetched_at > s0.fetched_at && s.phase == Phase::Idle).await;
    assert!(!s1.from_cache);
    r.core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn recent_always_revalidates() {
    let r = rig(args(&[])).await;
    let dir = tempfile::tempdir().unwrap();
    let d = ready(&r, &Paths::under(dir.path().join("data"))).await;
    let mut rx = d.list(ViewKey::RecentlyPlayed);
    let s0 = until(&mut rx, |s| s.items.len() == 4 && s.phase == Phase::Idle).await;
    tokio::time::sleep(Duration::from_millis(20)).await;
    let mut rx = d.list(ViewKey::RecentlyPlayed);
    let s1 = until(&mut rx, |s| s.fetched_at > s0.fetched_at && s.phase == Phase::Idle).await;
    assert_eq!(s1.items.len(), 4);
    r.core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn home_shelves() {
    let r = rig(args(&[])).await;
    let dir = tempfile::tempdir().unwrap();
    let d = ready(&r, &Paths::under(dir.path().join("data"))).await;
    let mut rx = d.shelves();
    let s = until(&mut rx, |s| !s.items.is_empty() && s.phase == Phase::Idle).await;
    let titles: Vec<_> = s.items.iter().map(|x| x.title.as_str()).collect();
    assert_eq!(titles, ["Made for You", "Albums You Might Like"]);
    assert!(s.items.iter().all(|x| x.items.len() == 2));
    r.core.shutdown().await;
}

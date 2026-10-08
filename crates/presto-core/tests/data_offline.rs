mod common;
use common::*;
use presto_core::data::DataHandle;
use presto_core::data::artwork::{ArtState, file_for};
use presto_core::data::error::UiErrorKind;
use presto_core::data::store::{Store, now_ms, req_key};
use presto_core::data::view::{ErrorDisplay, LibKind, ListState, Phase, Sort, ViewKey};
use presto_core::paths::Paths;
use presto_core::EngineStatus;
use presto_ipc::{AuthState, FaultSpec};
use std::time::{Duration, Instant};
use tokio::sync::watch;

const T: Duration = Duration::from_secs(5);

fn songs() -> ViewKey {
    ViewKey::Library { kind: LibKind::Songs, sort: Sort::Default }
}

fn args(a: &[&str]) -> impl Fn(u32) -> Vec<String> + Send + Sync + 'static {
    let v: Vec<String> = a.iter().map(|s| s.to_string()).collect();
    move |_| v.clone()
}

async fn until<X: Clone>(rx: &mut watch::Receiver<ListState<X>>, pred: impl Fn(&ListState<X>) -> bool) -> ListState<X> {
    tokio::time::timeout(T, rx.wait_for(|s| pred(s))).await.expect("timed out").unwrap().clone()
}

async fn signed_in(r: &Rig, paths: &Paths) -> DataHandle {
    wait_for(&mut r.core.state(), T, |s| s.engine == EngineStatus::Ready && s.auth == Some(AuthState::SignedIn)).await;
    DataHandle::new(r.core.clone(), paths).unwrap()
}

const ART: &str = "https://is1.mzstatic.com/a.jpg";

/// Run 1 loads 200 songs, seeds one artwork file (the http stub is unreachable from integration tests)
/// and ages page 0 past the TTL so run 2 has to try the network.
async fn warm(paths: &Paths) {
    let f = file_for(&paths.artwork, ART);
    paths.prepare().unwrap();
    std::fs::create_dir_all(f.parent().unwrap()).unwrap();
    std::fs::write(f, [7u8; 1000]).unwrap();
    let a = rig(args(&["--library-songs", "1000"])).await;
    let d = signed_in(&a, paths).await;
    let mut rx = d.list(songs());
    until(&mut rx, |s| s.items.len() == 100 && s.phase == Phase::Idle).await;
    d.load_more(&songs());
    until(&mut rx, |s| s.items.len() == 200 && s.phase == Phase::Idle).await;
    let acct = d.client().account_key().unwrap();
    drop(d);
    a.core.shutdown().await;
    let st = Store::open(&paths.db).unwrap();
    let rk = req_key(&songs().request(0));
    let body = st.get_page(&acct, &rk, 0).unwrap().body;
    st.put_page(&acct, &rk, 0, now_ms() - 7_200_000, &body);
}

async fn offline(paths: &Paths) -> (Rig, DataHandle) {
    let b = rig(args(&["--bridge-missing"])).await;
    wait_for(&mut b.core.state(), T, |s| matches!(s.engine, EngineStatus::Drift { .. } | EngineStatus::Failed { .. })).await;
    let d = DataHandle::new(b.core.clone(), paths).unwrap();
    (b, d)
}

#[tokio::test(flavor = "multi_thread")]
async fn offline_serves_cache() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path().join("data"));
    warm(&paths).await;
    let (b, d) = offline(&paths).await;
    let mut rx = d.list(songs());
    let s = until(&mut rx, |s| s.items.len() == 100 && s.error.is_some()).await;
    assert!(s.from_cache);
    assert_eq!(s.error.unwrap().kind, UiErrorKind::Offline);
    d.load_more(&songs());
    until(&mut rx, |s| s.items.len() == 200).await;
    assert!(matches!(d.art().get(ART), ArtState::Ready(_)));
    let alb = ViewKey::Library { kind: LibKind::Albums, sort: Sort::Default };
    let s = until(&mut d.list(alb), |s| s.error.is_some()).await;
    assert!(s.items.is_empty());
    assert_eq!(s.error.as_ref().unwrap().kind, UiErrorKind::Offline);
    assert_eq!(s.error_display(), Some(ErrorDisplay::FullView));
    b.core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn offline_fails_fast() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path().join("data"));
    warm(&paths).await;
    let (b, d) = offline(&paths).await;
    let mut rx = d.list(songs());
    until(&mut rx, |s| s.items.len() == 100 && s.error.is_some()).await;
    d.load_more(&songs());
    until(&mut rx, |s| s.items.len() == 200).await;
    d.refresh(&songs());
    let t0 = Instant::now();
    let s = until(&mut rx, |s| s.error.is_some() && s.phase == Phase::Idle).await;
    assert!(t0.elapsed() < Duration::from_millis(50), "{:?}", t0.elapsed());
    assert_eq!(s.error.unwrap().kind, UiErrorKind::Offline);
    assert_eq!(rx.borrow().items.len(), 200);
    b.core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn revalidates_after_restart() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path().join("data"));
    let r = rig(args(&["--library-songs", "300"])).await;
    let d = signed_in(&r, &paths).await;
    let mut rx = d.list(songs());
    let f1 = until(&mut rx, |s| s.items.len() == 100 && s.phase == Phase::Idle).await.fetched_at.unwrap();
    tokio::time::sleep(Duration::from_millis(20)).await;
    r.core.mock(FaultSpec::Crash { after_ms: None }).await;
    wait_for(&mut r.core.state(), T, |s| s.restarts >= 1 && s.engine == EngineStatus::Ready && s.auth == Some(AuthState::SignedIn)).await;
    until(&mut rx, |s| s.fetched_at.is_some_and(|f| f > f1)).await;
    r.core.shutdown().await;
}

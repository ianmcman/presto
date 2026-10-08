mod common;
use common::*;
use presto_core::data::DataHandle;
use presto_core::data::error::UiErrorKind;
use presto_core::data::view::{ErrorDisplay, LibKind, ListState, Phase, Sort, ViewKey};
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
    let v: Vec<String> = a.iter().map(|s| s.to_string()).collect();
    move |_| v.clone()
}

async fn ready(r: &Rig) -> DataHandle {
    let mut rx = r.core.state();
    wait_for(&mut rx, T, |s| s.engine == EngineStatus::Ready && s.auth == Some(AuthState::SignedIn)).await;
    DataHandle::new(r.core.clone(), &Paths::under(r.dir.path().join("data"))).unwrap()
}

async fn until<X: Clone>(rx: &mut watch::Receiver<ListState<X>>, pred: impl Fn(&ListState<X>) -> bool) -> ListState<X> {
    let r = tokio::time::timeout(T, rx.wait_for(|s| pred(s))).await.map(|r| r.unwrap().clone());
    match r {
        Ok(s) => s,
        Err(_) => panic!("timed out; last: {:?} {:?} {}", rx.borrow().error, rx.borrow().phase, rx.borrow().items.len()),
    }
}

fn rate_limited(s: &ListState<presto_core::data::models::Item>) -> bool {
    matches!(s.error.as_ref().map(|e| &e.kind), Some(UiErrorKind::RateLimited { .. }))
}

#[tokio::test(flavor = "multi_thread")]
async fn auto_retry_then_manual() {
    let r = rig(args(&["--library-songs", "300", "--fault", "rate_limited=100"])).await;
    let d = ready(&r).await;
    let mut rx = d.list(songs());
    let mut saw_countdown = false;
    let end = tokio::time::timeout(T, async {
        loop {
            let s = rx.borrow_and_update().clone();
            saw_countdown |= s.error.as_ref().is_some_and(|e| e.retry_at.is_some());
            if s.error.as_ref().is_some_and(|e| e.attempts == 3 && e.retry_at.is_none()) {
                return s;
            }
            rx.changed().await.unwrap();
        }
    })
    .await
    .expect("timed out");
    assert!(saw_countdown);
    assert!(rate_limited(&end) && end.items.is_empty());
    assert_eq!(end.error_display(), Some(ErrorDisplay::FullView));
    r.core.mock(FaultSpec::None).await;
    d.retry(&songs());
    let s = until(&mut rx, |s| s.items.len() == 100 && s.error.is_none()).await;
    assert_eq!(s.phase, Phase::Idle);
    r.core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn recovers_during_countdown() {
    let r = rig(args(&["--library-songs", "300", "--fault", "rate_limited=600"])).await;
    let d = ready(&r).await;
    let mut rx = d.list(songs());
    until(&mut rx, |s| s.error.as_ref().is_some_and(|e| e.retry_at.is_some())).await;
    r.core.mock(FaultSpec::None).await;
    until(&mut rx, |s| s.items.len() == 100 && s.error.is_none()).await;
    r.core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn banner_keeps_cache() {
    let r = rig(args(&["--library-songs", "300"])).await;
    let d = ready(&r).await;
    let mut rx = d.list(songs());
    until(&mut rx, |s| s.items.len() == 100 && s.phase == Phase::Idle).await;
    r.core.mock(FaultSpec::RateLimited { retry_after_ms: Some(5000) }).await;
    d.refresh(&songs());
    let s = until(&mut rx, rate_limited).await;
    assert_eq!(s.items.len(), 100);
    assert_eq!(s.error_display(), Some(ErrorDisplay::Banner));
    r.core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn not_found_full_view() {
    let r = rig(args(&[])).await;
    let d = ready(&r).await;
    let mut rx = d.list(ViewKey::Shelf { path: "/v1/me/nope".into() });
    let s = until(&mut rx, |s| s.error.is_some()).await;
    let e = s.error.as_ref().unwrap();
    assert_eq!((&e.kind, e.retry_at), (&UiErrorKind::NotFound, None));
    assert_eq!(s.error_display(), Some(ErrorDisplay::FullView));
    r.core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn load_more_error_is_banner() {
    let r = rig(args(&["--library-songs", "300"])).await;
    let d = ready(&r).await;
    let mut rx = d.list(songs());
    until(&mut rx, |s| s.items.len() == 100 && s.phase == Phase::Idle).await;
    r.core.mock(FaultSpec::RateLimited { retry_after_ms: Some(5000) }).await;
    d.load_more(&songs());
    let s = until(&mut rx, rate_limited).await;
    assert_eq!(s.items.len(), 100);
    assert_eq!(s.error_display(), Some(ErrorDisplay::Banner));
    r.core.mock(FaultSpec::None).await;
    d.retry(&songs());
    until(&mut rx, |s| s.items.len() == 200 && s.error.is_none()).await;
    r.core.shutdown().await;
}

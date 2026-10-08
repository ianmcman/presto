mod common;
use common::*;
use presto_core::EngineStatus;
use presto_core::data::client::ApiClient;
use presto_core::data::error::UiErrorKind;
use presto_core::data::search::{Scope, Search, SearchState};
use presto_core::data::store::Store;
use presto_ipc::AuthState;
use std::time::{Duration, Instant};
use tokio::sync::watch;

const T: Duration = Duration::from_secs(5);

fn args(a: &[&str]) -> impl Fn(u32) -> Vec<String> + Send + Sync + 'static {
    let v: Vec<String> = a.iter().map(|s| s.to_string()).collect();
    move |_| v.clone()
}

async fn setup(a: &[&str], wait_ready: bool) -> (Rig, Search) {
    let r = rig(args(a)).await;
    if wait_ready {
        let mut rx = r.core.state();
        wait_for(&mut rx, T, |s| s.engine == EngineStatus::Ready && s.auth == Some(AuthState::SignedIn)).await;
    } else {
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    let client = ApiClient::new(r.core.clone(), "t".into());
    let store = Store::open(&r.dir.path().join("s.db")).unwrap();
    let s = Search::new(client, store);
    (r, s)
}

/// Wait until `f` holds, recording every observed state.
async fn until(rx: &mut watch::Receiver<SearchState>, d: Duration, f: impl Fn(&SearchState) -> bool) -> Vec<SearchState> {
    let mut seen = vec![];
    tokio::time::timeout(d, async {
        loop {
            let st = rx.borrow_and_update().clone();
            let ok = f(&st);
            seen.push(st);
            if ok {
                return;
            }
            rx.changed().await.unwrap();
        }
    })
    .await
    .expect("timed out waiting for search state");
    seen
}

#[tokio::test(flavor = "multi_thread")]
async fn typing_debounced() {
    let (r, s) = setup(&[], true).await;
    let mut rx = s.state();
    for t in ["n", "ne", "neo"] {
        s.input(t);
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let seen = until(&mut rx, Duration::from_secs(2) + Duration::from_millis(500), |st| {
        st.results.as_ref().is_some_and(|r| r.term == "neo") && st.hints.iter().any(|h| h == "neon static")
    })
    .await;
    assert!(seen.iter().all(|st| st.results.as_ref().is_none_or(|r| r.term == "neo")));
    let res = seen.last().unwrap().results.clone().unwrap();
    assert!(res.songs.iter().any(|i| i.name == "Neon Static"));
    assert!(!res.top.is_empty());
    r.core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn submit_immediate() {
    let (r, s) = setup(&[], true).await;
    let mut rx = s.state();
    let t = Instant::now();
    s.input("neon");
    s.submit();
    until(&mut rx, T, |st| st.results.is_some()).await;
    assert!(t.elapsed() < Duration::from_millis(700));
    r.core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn storefront_gb() {
    let (r, s) = setup(&["--storefront", "gb"], true).await;
    let mut rx = s.state();
    s.input("neon");
    s.submit();
    let seen = until(&mut rx, T, |st| st.results.is_some() || st.error.is_some()).await;
    let st = seen.last().unwrap();
    assert!(st.error.is_none());
    assert!(!st.results.as_ref().unwrap().songs.is_empty());
    r.core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn library_scope() {
    let (r, s) = setup(&[], true).await;
    let mut rx = s.state();
    s.set_scope(Scope::Library);
    s.input("neon");
    s.submit();
    let seen = until(&mut rx, T, |st| st.results.is_some() || st.error.is_some()).await;
    let st = seen.last().unwrap();
    assert_eq!(st.scope, Scope::Library);
    assert!(!st.results.as_ref().expect("results").songs.is_empty());
    r.core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn history() {
    let (r, s) = setup(&[], true).await;
    let mut rx = s.state();
    for i in 0..12 {
        let term = format!("term{i}");
        s.input(&term);
        s.submit();
        until(&mut rx, T, |st| st.term == term && st.results.as_ref().is_some_and(|r| r.term == term)).await;
        // history is recorded just before the fetch; give distinct timestamps
        tokio::time::sleep(Duration::from_millis(3)).await;
    }
    let h = rx.borrow().history.clone();
    assert_eq!(h.len(), 10);
    assert_eq!(h[0], "term11");
    s.clear_history();
    assert!(rx.borrow().history.is_empty());
    r.core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn offline_error() {
    let (r, s) = setup(&["--bridge-missing"], false).await;
    let mut rx = s.state();
    let t = Instant::now();
    s.input("neon");
    s.submit();
    let seen = until(&mut rx, T, |st| st.error.is_some()).await;
    let st = seen.last().unwrap();
    assert_eq!(st.error.as_ref().unwrap().kind, UiErrorKind::Offline);
    assert!(st.results.is_none());
    assert!(t.elapsed() < Duration::from_secs(1));
    r.core.shutdown().await;
}

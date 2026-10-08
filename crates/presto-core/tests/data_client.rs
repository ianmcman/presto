mod common;
use common::*;
use presto_core::data::client::ApiClient;
use presto_core::data::error::UiErrorKind;
use presto_core::EngineStatus;
use presto_ipc::{ApiRequest, AuthState};
use std::time::{Duration, Instant};

const T: Duration = Duration::from_secs(5);

fn args(a: &[&str]) -> impl Fn(u32) -> Vec<String> + Send + Sync + 'static {
    let v: Vec<String> = a.iter().map(|s| s.to_string()).collect();
    move |_| v.clone()
}

async fn ready(r: &Rig) -> ApiClient {
    let mut rx = r.core.state();
    wait_for(&mut rx, T, |s| s.engine == EngineStatus::Ready && s.auth == Some(AuthState::SignedIn)).await;
    ApiClient::new(r.core.clone(), "test-install".into())
}

#[tokio::test]
async fn storefront_applied() {
    let r = rig(args(&["--storefront", "gb"])).await;
    let c = ready(&r).await;
    assert_eq!(c.storefront().await.unwrap(), "gb");
    assert_eq!(c.account_key().as_deref(), Some("gb:test-install"));
    let v = c.catalog("search", &[("term", "neon")]).await.unwrap();
    assert!(!v["results"]["songs"]["data"].as_array().unwrap().is_empty());
    r.core.shutdown().await;
}

#[tokio::test]
async fn wrong_storefront_is_not_found() {
    let r = rig(args(&["--storefront", "gb"])).await;
    let c = ready(&r).await;
    c.set_storefront(Some("us".into()));
    assert_eq!(c.catalog("search", &[("term", "neon")]).await, Err(UiErrorKind::NotFound));
    r.core.shutdown().await;
}

#[tokio::test]
async fn signed_out_fails_fast() {
    let r = rig(args(&["--auth", "signed_out"])).await;
    let mut rx = r.core.state();
    wait_for(&mut rx, T, |s| s.engine == EngineStatus::Ready && s.auth == Some(AuthState::SignedOut)).await;
    let c = ApiClient::new(r.core.clone(), "test-install".into());
    let t = Instant::now();
    assert_eq!(c.get(ApiRequest::get("/v1/me/library/songs")).await, Err(UiErrorKind::SignedOut));
    assert!(t.elapsed() < Duration::from_millis(50));
    r.core.shutdown().await;
}

#[tokio::test]
async fn offline_fails_fast() {
    let r = rig(args(&["--bridge-missing"])).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    let c = ApiClient::new(r.core.clone(), "test-install".into());
    let t = Instant::now();
    assert_eq!(c.get(ApiRequest::get("/v1/me/library/songs")).await, Err(UiErrorKind::Offline));
    assert!(t.elapsed() < Duration::from_millis(50));
    r.core.shutdown().await;
}

#[tokio::test]
async fn rate_limited_mapped() {
    let r = rig(args(&["--fault", "rate_limited=250"])).await;
    let c = ready(&r).await;
    assert_eq!(
        c.get(ApiRequest::get("/v1/me/library/songs")).await,
        Err(UiErrorKind::RateLimited { retry_after_ms: Some(250) })
    );
    r.core.shutdown().await;
}

//! ApiClient over CoreHandle: storefront, gating, concurrency bound, error mapping.
use crate::data::error::UiErrorKind;
use crate::{CoreHandle, CoreState, EngineStatus};
use presto_ipc::{ApiRequest, AuthState, Outcome};
use serde_json::Value;
use std::sync::{Arc, RwLock};
use tokio::sync::{Semaphore, watch};

pub const MAX_IN_FLIGHT: usize = 4;

/// Build a catalog request. Every catalog path in the data layer must come from here.
pub fn catalog_req(sf: &str, rest: &str, query: &[(&str, &str)]) -> ApiRequest {
    let mut r = ApiRequest::get(format!("/v1/catalog/{sf}/{}", rest.trim_start_matches('/')));
    for (k, v) in query {
        r.query.insert((*k).into(), (*v).into());
    }
    r
}

fn gate_for(s: &CoreState) -> Result<(), UiErrorKind> {
    if s.engine != EngineStatus::Ready {
        return Err(UiErrorKind::Offline);
    }
    match s.auth {
        Some(AuthState::SignedIn) => Ok(()),
        Some(AuthState::SignedOut) => Err(UiErrorKind::SignedOut),
        Some(AuthState::Expired) => Err(UiErrorKind::AuthExpired),
        _ => Err(UiErrorKind::Offline),
    }
}

fn key(sf: Option<&str>, id: &str) -> Option<String> {
    sf.map(|s| format!("{s}:{id}"))
}

#[derive(Clone)]
pub struct ApiClient {
    core: CoreHandle,
    state: watch::Receiver<CoreState>,
    sem: Arc<Semaphore>,
    storefront: Arc<RwLock<Option<String>>>,
    install_id: Arc<str>,
}

impl ApiClient {
    pub fn new(core: CoreHandle, install_id: String) -> ApiClient {
        ApiClient {
            state: core.state(),
            core,
            sem: Arc::new(Semaphore::new(MAX_IN_FLIGHT)),
            storefront: Arc::new(RwLock::new(None)),
            install_id: install_id.into(),
        }
    }

    pub fn core(&self) -> &CoreHandle {
        &self.core
    }

    pub fn gate(&self) -> Result<(), UiErrorKind> {
        gate_for(&self.state.borrow())
    }

    pub async fn get(&self, req: ApiRequest) -> Result<Value, UiErrorKind> {
        self.gate()?;
        let _permit = self.sem.acquire().await.map_err(|_| UiErrorKind::Internal)?;
        self.gate()?;
        match self.core.request(req).await {
            Outcome::Ok { data } => Ok(data),
            Outcome::Err { error } => Err(UiErrorKind::from_ipc(&error.kind)),
        }
    }

    pub async fn storefront(&self) -> Result<String, UiErrorKind> {
        match self.cached_storefront() {
            Some(s) => Ok(s),
            None => self.refresh_storefront().await,
        }
    }

    pub async fn refresh_storefront(&self) -> Result<String, UiErrorKind> {
        let v = self.get(ApiRequest::get("/v1/me/storefront")).await?;
        let sf = v["data"][0]["id"].as_str().ok_or(UiErrorKind::Internal)?.to_string();
        self.set_storefront(Some(sf.clone()));
        Ok(sf)
    }

    pub fn cached_storefront(&self) -> Option<String> {
        self.storefront.read().unwrap().clone()
    }

    pub fn set_storefront(&self, sf: Option<String>) {
        *self.storefront.write().unwrap() = sf;
    }

    pub fn account_key(&self) -> Option<String> {
        key(self.cached_storefront().as_deref(), &self.install_id)
    }

    pub async fn catalog(&self, rest: &str, query: &[(&str, &str)]) -> Result<Value, UiErrorKind> {
        let sf = self.storefront().await?;
        self.get(catalog_req(&sf, rest, query)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn st(engine: EngineStatus, auth: Option<AuthState>) -> CoreState {
        CoreState { engine, auth, ..Default::default() }
    }

    #[test]
    fn catalog_paths() {
        let r = catalog_req("gb", "search", &[("term", "neon")]);
        assert_eq!(r.path, "/v1/catalog/gb/search");
        assert_eq!(r.query.get("term").map(String::as_str), Some("neon"));
    }

    #[test]
    fn catalog_leading_slash() {
        assert_eq!(catalog_req("us", "/albums/1", &[]).path, "/v1/catalog/us/albums/1");
    }

    #[test]
    fn gate() {
        let ready = EngineStatus::Ready;
        assert_eq!(gate_for(&st(ready.clone(), Some(AuthState::SignedIn))), Ok(()));
        assert_eq!(gate_for(&st(EngineStatus::Starting, Some(AuthState::SignedIn))), Err(UiErrorKind::Offline));
        assert_eq!(gate_for(&st(ready.clone(), Some(AuthState::SignedOut))), Err(UiErrorKind::SignedOut));
        assert_eq!(gate_for(&st(ready.clone(), Some(AuthState::Expired))), Err(UiErrorKind::AuthExpired));
        assert_eq!(gate_for(&st(ready.clone(), Some(AuthState::SigningIn))), Err(UiErrorKind::Offline));
        assert_eq!(gate_for(&st(ready, None)), Err(UiErrorKind::Offline));
    }

    #[test]
    fn account_key() {
        assert_eq!(key(None, "abc"), None);
        assert_eq!(key(Some("gb"), "abc").as_deref(), Some("gb:abc"));
    }
}

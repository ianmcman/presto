//! Debounced search: hints, grouped results, catalog/library scope, per-account history.
//! Results are never cached; only history persists.
use super::client::ApiClient;
use super::models::{SearchResults, parse_hints, parse_search};
use super::store::{Store, now_ms};
use super::view::UiError;
use presto_ipc::ApiRequest;
use serde::Serialize;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering::SeqCst};
use std::time::Duration;
use tokio::runtime::Handle;
use tokio::sync::watch;

pub const DEBOUNCE: Duration = Duration::from_millis(300);
/// Total pause before full results while typing.
pub const RESULTS_PAUSE: Duration = Duration::from_millis(1000);
pub const HISTORY_MAX: usize = 10;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub enum Scope {
    #[default]
    Catalog,
    Library,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SearchState {
    pub term: String,
    pub scope: Scope,
    pub hints: Vec<String>,
    pub results: Option<SearchResults>,
    pub searching: bool,
    pub error: Option<UiError>,
    pub history: Vec<String>,
}

#[derive(Clone)]
pub struct Search {
    client: ApiClient,
    store: Store,
    tx: Arc<watch::Sender<SearchState>>,
    seq: Arc<AtomicU64>,
    rt: Handle,
}

/// Sleep `d`, then report whether `seq` is still `mine` (no newer input).
pub(crate) async fn settled(seq: &AtomicU64, mine: u64, d: Duration) -> bool {
    tokio::time::sleep(d).await;
    seq.load(SeqCst) == mine
}

impl Search {
    /// Must be called inside a tokio runtime.
    pub fn new(client: ApiClient, store: Store) -> Search {
        let s = Search {
            client,
            store,
            tx: Arc::new(watch::channel(SearchState::default()).0),
            seq: Arc::new(AtomicU64::new(0)),
            rt: Handle::current(),
        };
        s.tx.send_modify(|st| st.history = s.load_history());
        s
    }

    pub fn state(&self) -> watch::Receiver<SearchState> {
        self.tx.subscribe()
    }

    fn load_history(&self) -> Vec<String> {
        self.client.account_key().map(|a| self.store.search_history(&a)).unwrap_or_default()
    }

    pub fn input(&self, term: &str) {
        let t = term.trim().to_owned();
        let mine = self.seq.fetch_add(1, SeqCst) + 1;
        if t.is_empty() {
            let history = self.load_history();
            self.tx.send_modify(|st| {
                st.term.clear();
                st.error = None;
                st.hints.clear();
                st.results = None;
                st.searching = false;
                st.history = history;
            });
            return;
        }
        self.tx.send_modify(|st| {
            st.term = t.clone();
            st.error = None;
        });
        let me = self.clone();
        self.rt.spawn(async move {
            if !settled(&me.seq, mine, DEBOUNCE).await {
                return;
            }
            if me.tx.borrow().scope == Scope::Catalog {
                if let Ok(v) = me.client.catalog("search/hints", &[("term", t.as_str()), ("limit", "10")]).await {
                    if me.seq.load(SeqCst) == mine {
                        me.tx.send_modify(|st| st.hints = parse_hints(&v));
                    }
                }
            }
            if !settled(&me.seq, mine, RESULTS_PAUSE - DEBOUNCE).await {
                return;
            }
            me.full(t, mine).await;
        });
    }

    pub fn submit(&self) {
        self.run(true);
    }

    fn run(&self, record: bool) {
        let t = self.tx.borrow().term.clone();
        let mine = self.seq.fetch_add(1, SeqCst) + 1;
        if t.is_empty() {
            return;
        }
        let me = self.clone();
        self.rt.spawn(async move {
            if record {
                let _ = me.client.storefront().await; // account key needs the storefront
                if let Some(a) = me.client.account_key() {
                    let (store, term) = (me.store.clone(), t.clone());
                    let _ = tokio::task::spawn_blocking(move || store.push_search(&a, &term, now_ms())).await;
                }
                let h = me.load_history();
                me.tx.send_modify(|st| st.history = h);
            }
            me.full(t, mine).await;
        });
    }

    async fn full(&self, t: String, mine: u64) {
        self.tx.send_modify(|st| st.searching = true);
        let scope = self.tx.borrow().scope;
        let res = match scope {
            Scope::Catalog => {
                self.client
                    .catalog(
                        "search",
                        &[("term", t.as_str()), ("types", "songs,albums,artists,playlists"), ("limit", "25"), ("with", "topResults")],
                    )
                    .await
            }
            Scope::Library => {
                let mut r = ApiRequest::get("/v1/me/library/search");
                for (k, v) in [
                    ("term", t.as_str()),
                    ("types", "library-songs,library-albums,library-artists,library-playlists"),
                    ("limit", "25"),
                ] {
                    r.query.insert(k.into(), v.into());
                }
                self.client.get(r).await
            }
        };
        if self.seq.load(SeqCst) != mine {
            return;
        }
        self.tx.send_modify(|st| {
            st.searching = false;
            match res {
                Ok(v) => st.results = Some(parse_search(&t, &v)),
                Err(kind) => st.error = Some(UiError { kind, attempts: 0, retry_at: None }),
            }
        });
    }

    pub fn set_scope(&self, scope: Scope) {
        self.seq.fetch_add(1, SeqCst);
        self.tx.send_modify(|st| {
            st.scope = scope;
            st.hints.clear();
            st.results = None;
            st.error = None;
            st.searching = false;
        });
        self.run(false);
    }

    pub fn clear_history(&self) {
        if let Some(a) = self.client.account_key() {
            self.store.clear_search(&a);
        }
        self.tx.send_modify(|st| st.history.clear());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn settled_true_without_bump() {
        let g = AtomicU64::new(1);
        assert!(settled(&g, 1, DEBOUNCE).await);
    }

    #[tokio::test(start_paused = true)]
    async fn settled_false_after_bump() {
        let g = Arc::new(AtomicU64::new(1));
        let g2 = g.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            g2.store(2, SeqCst);
        });
        assert!(!settled(&g, 1, DEBOUNCE).await);
    }
}

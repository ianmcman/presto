//! Data layer: proxied API reads, SQLite page cache, artwork cache, per-view state (Phase 4). No UI.
pub mod artwork;
pub mod client;
pub mod error;
pub mod models;
pub mod search;
pub mod store;
pub mod view;

use std::collections::HashMap;
use std::io;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::Value;
use tokio::sync::watch;

use crate::{CoreHandle, CoreState, EngineStatus};
use presto_ipc::{ApiRequest, AuthState};
use crate::paths::Paths;
use artwork::{ART_CAP, ArtCache};
use client::ApiClient;
use error::UiErrorKind;
use models::{Detail, Item, Rows, Shelf};
use store::{Store, now_ms, req_key};
use view::{ListState, Phase, SF, TTL, UiError, ViewKey, is_stale, next_retry};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Op {
    First,
    More,
}

struct Slot<T> {
    tx: watch::Sender<ListState<T>>,
    gn: u64,
    pages: u32,
    fetched_at_ms: Option<i64>,
    cached_body0: Option<Vec<u8>>,
    last_op: Op,
}

type Slots<T> = Mutex<HashMap<ViewKey, Slot<T>>>;

trait Slotted: Rows {
    fn slots(i: &Inner) -> &Slots<Self>;
}
impl Slotted for Item {
    fn slots(i: &Inner) -> &Slots<Self> {
        &i.items
    }
}
impl Slotted for Shelf {
    fn slots(i: &Inner) -> &Slots<Self> {
        &i.shelves
    }
}

impl Slotted for Detail {
    fn slots(i: &Inner) -> &Slots<Self> {
        &i.details
    }
}

/// `key.request(off)` with the storefront placeholder filled in. None while the storefront is unknown.
fn req(i: &Inner, key: &ViewKey, off: u32) -> Option<ApiRequest> {
    let mut r = key.request(off);
    if r.path.contains(SF) {
        r.path = r.path.replace(SF, &i.client.cached_storefront()?);
    }
    Some(r)
}

struct Inner {
    rt: tokio::runtime::Handle,
    client: ApiClient,
    store: Store,
    art: Arc<ArtCache>,
    items: Slots<Item>,
    shelves: Slots<Shelf>,
    details: Slots<Detail>,
    focused: Mutex<Option<ViewKey>>,
    invalidated_at_ms: AtomicI64,
}

#[derive(Clone)]
pub struct DataHandle {
    inner: Arc<Inner>,
}

fn upd<T: Slotted, R>(i: &Inner, key: &ViewKey, f: impl FnOnce(&mut Slot<T>) -> R) -> Option<R> {
    T::slots(i).lock().unwrap().get_mut(key).map(f)
}

async fn blk<R: Send + 'static>(f: impl FnOnce() -> R + Send + 'static) -> Option<R> {
    tokio::task::spawn_blocking(f).await.ok()
}

fn sys(ms: i64) -> SystemTime {
    UNIX_EPOCH + Duration::from_millis(ms.max(0) as u64)
}

/// Mark a fetch as started: loading phase for the op, countdown cleared.
fn begin<T: Slotted>(i: &Inner, key: &ViewKey, gn: u64, op: Op) -> bool {
    upd::<T, _>(i, key, |sl| {
        if sl.gn != gn {
            return false;
        }
        sl.last_op = op;
        sl.tx.send_modify(|s| {
            s.phase = match (s.items.is_empty(), op) {
                (true, _) => Phase::Loading,
                (_, Op::More) => Phase::LoadingMore,
                _ => Phase::Revalidating,
            };
            if let Some(e) = &mut s.error {
                e.retry_at = None;
            }
        });
        true
    })
    .unwrap_or(false)
}

fn run<T: Slotted>(i: Arc<Inner>, key: ViewKey, gn: u64, op: Op, attempts: u8) -> std::pin::Pin<Box<dyn Future<Output = ()> + Send>> {
    Box::pin(async move {
        match op {
            Op::First => fetch0::<T>(i, key, gn, attempts).await,
            Op::More => more::<T>(i, key, gn, attempts).await,
        }
    })
}

fn fail<T: Slotted>(i: &Arc<Inner>, key: &ViewKey, gn: u64, op: Op, kind: UiErrorKind, attempts: u8) {
    let retry_at = next_retry(&kind, attempts, Instant::now());
    let live = upd::<T, _>(i, key, |sl| {
        if sl.gn != gn {
            return false;
        }
        sl.last_op = op;
        sl.tx.send_modify(|s| {
            s.error = Some(UiError { kind, attempts, retry_at });
            s.phase = Phase::Idle;
        });
        true
    })
    .unwrap_or(false);
    if let (true, Some(at)) = (live, retry_at) {
        let (i, key) = (i.clone(), key.clone());
        i.rt.clone().spawn(async move {
            tokio::time::sleep_until(at.into()).await;
            if begin::<T>(&i, &key, gn, op) {
                run::<T>(i, key, gn, op, attempts + 1).await;
            }
        });
    }
}

async fn first<T: Slotted>(i: Arc<Inner>, key: ViewKey, gn: u64) {
    if let Some(acct) = i.client.account_key() {
        let hit = match req(&i, &key, 0) {
            Some(r) => {
                let (st, rk) = (i.store.clone(), req_key(&r));
                blk(move || st.get_page(&acct, &rk, 0)).await.flatten()
            }
            None => None,
        };
        if let Some((c, v)) = hit.and_then(|c| serde_json::from_slice::<Value>(&c.body).ok().map(|v| (c, v))) {
            let p = T::parse_page(&v);
            let stale = is_stale(key.clone(), c.fetched_at_ms, now_ms(), false, i.invalidated_at_ms.load(Ordering::Relaxed));
            let live = upd::<T, _>(&i, &key, |sl| {
                if sl.gn != gn {
                    return false;
                }
                sl.pages = 1;
                sl.fetched_at_ms = Some(c.fetched_at_ms);
                sl.cached_body0 = Some(c.body);
                sl.tx.send_modify(|s| {
                    s.has_more = p.next.is_some();
                    s.total = p.total;
                    s.items = p.items;
                    s.from_cache = true;
                    s.fetched_at = Some(sys(c.fetched_at_ms));
                    s.phase = if stale { Phase::Revalidating } else { Phase::Idle };
                });
                true
            })
            .unwrap_or(false);
            if !live || !stale {
                return;
            }
        }
    }
    fetch0::<T>(i, key, gn, 0).await
}

async fn fetch0<T: Slotted>(i: Arc<Inner>, key: ViewKey, gn: u64, attempts: u8) {
    let acct = match i.client.account_key() {
        Some(a) => a,
        None => match i.client.storefront().await {
            Ok(sf) => {
                i.store.set_meta("storefront", &sf);
                match i.client.account_key() {
                    Some(a) => a,
                    None => return fail::<T>(&i, &key, gn, Op::First, UiErrorKind::Internal, attempts),
                }
            }
            Err(k) => return fail::<T>(&i, &key, gn, Op::First, k, attempts),
        },
    };
    let Some(rq) = req(&i, &key, 0) else {
        return fail::<T>(&i, &key, gn, Op::First, UiErrorKind::Internal, attempts);
    };
    let rk = req_key(&rq);
    let v = match i.client.get(rq).await {
        Ok(v) => v,
        Err(k) => return fail::<T>(&i, &key, gn, Op::First, k, attempts),
    };
    let Some(prev) = upd::<T, _>(&i, &key, |sl| (sl.gn == gn).then(|| sl.cached_body0.clone())).flatten() else {
        return;
    };
    let body = serde_json::to_vec(&v).unwrap_or_default();
    let changed = prev.as_ref().is_some_and(|p| *p != body);
    let (st, b, size, now) = (i.store.clone(), body.clone(), key.page_size(), now_ms());
    blk(move || {
        st.put_page(&acct, &rk, 0, now, &b);
        if changed {
            // offsets may have shifted; never splice old pages onto new ones
            st.drop_pages_from(&acct, &rk, size);
        }
    })
    .await;
    let p = T::parse_page(&v);
    upd::<T, _>(&i, &key, |sl| {
        if sl.gn != gn {
            return;
        }
        sl.pages = 1;
        sl.fetched_at_ms = Some(now);
        sl.cached_body0 = Some(body);
        sl.tx.send_modify(|s| {
            s.has_more = p.next.is_some();
            s.total = p.total;
            s.items = p.items;
            s.from_cache = false;
            s.fetched_at = Some(sys(now));
            if changed {
                s.updated_at = Some(Instant::now());
            }
            s.error = None;
            s.phase = Phase::Idle;
        });
    });
}

async fn more<T: Slotted>(i: Arc<Inner>, key: ViewKey, gn: u64, attempts: u8) {
    let Some(off) = upd::<T, _>(&i, &key, |sl| (sl.gn == gn).then(|| sl.pages * key.page_size())).flatten() else {
        return;
    };
    let Some(rq) = req(&i, &key, off) else {
        return fail::<T>(&i, &key, gn, Op::More, UiErrorKind::Internal, attempts);
    };
    let rk = req_key(&rq);
    let acct = i.client.account_key();
    let cached = match &acct {
        Some(a) => {
            let (st, a, rk) = (i.store.clone(), a.clone(), rk.clone());
            blk(move || st.get_page(&a, &rk, off)).await.flatten()
        }
        None => None,
    };
    let ttl = TTL.as_millis() as i64;
    let hit = cached
        .filter(|c| i.client.gate().is_err() || now_ms() - c.fetched_at_ms < ttl)
        .and_then(|c| serde_json::from_slice::<Value>(&c.body).ok());
    let v = match hit {
        Some(v) => v,
        None => match i.client.get(rq).await {
            Ok(v) => {
                if let Some(a) = acct {
                    let (st, b) = (i.store.clone(), serde_json::to_vec(&v).unwrap_or_default());
                    blk(move || st.put_page(&a, &rk, off, now_ms(), &b)).await;
                }
                v
            }
            Err(k) => return fail::<T>(&i, &key, gn, Op::More, k, attempts),
        },
    };
    let p = T::parse_page(&v);
    upd::<T, _>(&i, &key, |sl| {
        if sl.gn != gn {
            return;
        }
        sl.pages += 1;
        sl.tx.send_modify(|s| {
            s.items.extend(p.items);
            s.has_more = p.next.is_some();
            s.total = p.total.or(s.total);
            s.error = None;
            s.phase = Phase::Idle;
        });
    });
}

fn open<T: Slotted>(i: &Arc<Inner>, key: ViewKey) -> watch::Receiver<ListState<T>> {
    *i.focused.lock().unwrap() = Some(key.clone());
    let mut m = T::slots(i).lock().unwrap();
    if let Some(sl) = m.get_mut(&key) {
        let rx = sl.tx.subscribe();
        let inv = i.invalidated_at_ms.load(Ordering::Relaxed);
        let stale = sl.fetched_at_ms.is_none_or(|f| is_stale(key.clone(), f, now_ms(), false, inv));
        let (idle, counting) = {
            let s = sl.tx.borrow();
            (s.phase == Phase::Idle, s.error.as_ref().is_some_and(|e| e.retry_at.is_some()))
        };
        if stale && idle && !counting {
            let gn = sl.gn;
            sl.last_op = Op::First;
            sl.tx.send_modify(|s| s.phase = if s.items.is_empty() { Phase::Loading } else { Phase::Revalidating });
            i.rt.spawn(fetch0::<T>(i.clone(), key, gn, 0));
        }
        return rx;
    }
    let (tx, rx) = watch::channel(ListState { phase: Phase::Loading, ..Default::default() });
    m.insert(key.clone(), Slot { tx, gn: 0, pages: 0, fetched_at_ms: None, cached_body0: None, last_op: Op::First });
    i.rt.spawn(first::<T>(i.clone(), key, 0));
    rx
}

fn restart<T: Slotted>(i: &Arc<Inner>, key: &ViewKey, op: Op) {
    let Some(gn) = upd::<T, _>(i, key, |sl| {
        sl.gn += 1;
        sl.gn
    }) else {
        return;
    };
    if begin::<T>(i, key, gn, op) {
        i.rt.spawn(run::<T>(i.clone(), key.clone(), gn, op, 0));
    }
}

fn load_more_t<T: Slotted>(i: &Arc<Inner>, key: &ViewKey) {
    let go = upd::<T, _>(i, key, |sl| {
        let s = sl.tx.borrow();
        // after a failed page the user (or the countdown) owns the next attempt
        let blocked = s.error.is_some() && sl.last_op == Op::More;
        (s.phase == Phase::Idle && s.has_more && !blocked).then_some(sl.gn)
    })
    .flatten();
    if let Some(gn) = go {
        if begin::<T>(i, key, gn, Op::More) {
            i.rt.spawn(more::<T>(i.clone(), key.clone(), gn, 0));
        }
    }
}

fn usable(s: &CoreState) -> bool {
    s.engine == EngineStatus::Ready && s.auth == Some(AuthState::SignedIn)
}

/// Mid-session sign-out only: never a startup signed_out, never expiry, never across a restart.
fn signed_out_transition(prev: &CoreState, cur: &CoreState) -> bool {
    prev.auth == Some(AuthState::SignedIn) && cur.auth == Some(AuthState::SignedOut) && prev.restarts == cur.restarts
}

fn reset<T: Slotted>(i: &Inner) {
    for sl in T::slots(i).lock().unwrap().values_mut() {
        sl.gn += 1;
        sl.pages = 0;
        sl.fetched_at_ms = None;
        sl.cached_body0 = None;
        sl.tx.send_replace(ListState::default());
    }
}

fn reset_all(i: &Inner) {
    reset::<Item>(i);
    reset::<Shelf>(i);
    reset::<Detail>(i);
}

fn refresh_focused(i: &Arc<Inner>) {
    let f = i.focused.lock().unwrap().clone();
    if let Some(k) = f {
        DataHandle { inner: i.clone() }.refresh(&k);
    }
}

// ponytail: an in-flight fetch that already passed its gen check can still write one page after a wipe.
async fn wipe(i: &Arc<Inner>, reload: bool) {
    let (st, art) = (i.store.clone(), i.art.clone());
    blk(move || {
        st.wipe();
        let _ = art.clear();
    })
    .await;
    i.client.set_storefront(None);
    reset_all(i);
    if reload {
        refresh_focused(i);
    }
}

fn offline_keys<T: Slotted>(i: &Inner) -> Vec<ViewKey> {
    let slots = T::slots(i).lock().unwrap();
    let off = |sl: &Slot<T>| sl.tx.borrow().error.as_ref().is_some_and(|e| matches!(e.kind, UiErrorKind::Offline));
    slots.iter().filter(|(_, sl)| off(sl)).map(|(k, _)| k.clone()).collect()
}

/// Views opened before the engine was ready failed with Offline; only the focused one is refreshed,
/// so a page with several views (Home) would leave the rest failed until a manual retry.
fn retry_offline(i: &Arc<Inner>) {
    let focused = i.focused.lock().unwrap().clone();
    let keys = [offline_keys::<Item>(i), offline_keys::<Shelf>(i), offline_keys::<Detail>(i)].concat();
    let h = DataHandle { inner: i.clone() };
    for k in keys.iter().filter(|k| Some(*k) != focused.as_ref()) {
        h.retry(k);
    }
}

async fn on_transition(i: &Arc<Inner>, prev: &CoreState, cur: &CoreState) {
    if signed_out_transition(prev, cur) {
        return wipe(i, false).await;
    }
    if usable(cur) && (!usable(prev) || prev.restarts != cur.restarts) {
        let old = i.client.cached_storefront();
        if let Ok(sf) = i.client.refresh_storefront().await {
            let (st, v) = (i.store.clone(), sf.clone());
            blk(move || st.set_meta("storefront", &v)).await;
            if old.is_some_and(|o| o != sf) {
                reset_all(i);
            }
        }
        // D-04: other views revalidate on next open; only the focused one now
        i.invalidated_at_ms.store(now_ms(), Ordering::Relaxed);
        refresh_focused(i);
        retry_offline(i);
    }
}

impl DataHandle {
    /// Call inside a tokio runtime: background work is spawned on it.
    pub fn new(core: CoreHandle, paths: &Paths) -> io::Result<DataHandle> {
        paths.prepare()?;
        let store = Store::open(&paths.db).map_err(io::Error::other)?;
        let art = ArtCache::open(paths.artwork.clone(), ART_CAP)?;
        let client = ApiClient::new(core, paths.install_id()?);
        // an offline start can still compute the account key
        client.set_storefront(store.get_meta("storefront"));
        let inner = Arc::new(Inner {
            rt: tokio::runtime::Handle::current(),
            client,
            store,
            art,
            items: Default::default(),
            shelves: Default::default(),
            details: Default::default(),
            focused: Default::default(),
            invalidated_at_ms: AtomicI64::new(0),
        });
        let (weak, mut rx) = (Arc::downgrade(&inner), inner.client.core().state());
        let mut prev = rx.borrow_and_update().clone();
        inner.rt.spawn(async move {
            while rx.changed().await.is_ok() {
                let cur = rx.borrow_and_update().clone();
                let Some(i) = weak.upgrade() else { return };
                on_transition(&i, &prev, &cur).await;
                prev = cur;
            }
        });
        Ok(DataHandle { inner })
    }

    /// D-17 Settings button: wipes cache and artwork, keeps sign-in, reloads the open view.
    pub fn clear_cache(&self) {
        let i = self.inner.clone();
        self.inner.rt.spawn(async move { wipe(&i, true).await });
    }

    /// Library, RecentlyPlayed, Shelf, Tracks. Use `shelves()` for Recommendations.
    pub fn list(&self, key: ViewKey) -> watch::Receiver<ListState<Item>> {
        debug_assert!(key != ViewKey::Recommendations);
        open::<Item>(&self.inner, key)
    }

    pub fn shelves(&self) -> watch::Receiver<ListState<Shelf>> {
        open::<Shelf>(&self.inner, ViewKey::Recommendations)
    }

    /// Album, playlist or artist head (`view::detail_key`).
    pub fn detail(&self, key: ViewKey) -> watch::Receiver<ListState<Detail>> {
        debug_assert!(matches!(key, ViewKey::Detail { .. }));
        open::<Detail>(&self.inner, key)
    }

    pub fn load_more(&self, key: &ViewKey) {
        match key {
            ViewKey::Recommendations => load_more_t::<Shelf>(&self.inner, key),
            ViewKey::Detail { .. } => load_more_t::<Detail>(&self.inner, key),
            _ => load_more_t::<Item>(&self.inner, key),
        }
    }

    /// D-03: bypasses the TTL.
    pub fn refresh(&self, key: &ViewKey) {
        match key {
            ViewKey::Recommendations => restart::<Shelf>(&self.inner, key, Op::First),
            ViewKey::Detail { .. } => restart::<Detail>(&self.inner, key, Op::First),
            _ => restart::<Item>(&self.inner, key, Op::First),
        }
    }

    /// Manual Retry: repeats the failed operation with the attempt count reset.
    pub fn retry(&self, key: &ViewKey) {
        let i = &self.inner;
        match key {
            ViewKey::Recommendations => {
                if let Some(op) = upd::<Shelf, _>(i, key, |s| s.last_op) {
                    restart::<Shelf>(i, key, op)
                }
            }
            ViewKey::Detail { .. } => {
                if let Some(op) = upd::<Detail, _>(i, key, |s| s.last_op) {
                    restart::<Detail>(i, key, op)
                }
            }
            _ => {
                if let Some(op) = upd::<Item, _>(i, key, |s| s.last_op) {
                    restart::<Item>(i, key, op)
                }
            }
        }
    }

    pub fn client(&self) -> &ApiClient {
        &self.inner.client
    }

    pub fn store(&self) -> &Store {
        &self.inner.store
    }

    pub fn art(&self) -> &Arc<ArtCache> {
        &self.inner.art
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn st(auth: Option<AuthState>, restarts: u32) -> CoreState {
        CoreState { auth, restarts, ..Default::default() }
    }

    #[test]
    fn wipe_rule() {
        use AuthState::*;
        assert!(!signed_out_transition(&st(None, 0), &st(Some(SignedOut), 0)));
        assert!(!signed_out_transition(&st(Some(SignedIn), 0), &st(Some(Expired), 0)));
        assert!(!signed_out_transition(&st(Some(SignedIn), 0), &st(Some(SignedOut), 1)));
        assert!(signed_out_transition(&st(Some(SignedIn), 0), &st(Some(SignedOut), 0)));
    }

    #[test]
    fn usable_needs_ready_and_signed_in() {
        let mut s = st(Some(AuthState::SignedIn), 0);
        assert!(!usable(&s));
        s.engine = EngineStatus::Ready;
        assert!(usable(&s));
        s.auth = Some(AuthState::Expired);
        assert!(!usable(&s));
    }
}

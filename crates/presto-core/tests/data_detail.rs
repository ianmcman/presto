mod common;
use common::*;
use presto_core::data::DataHandle;
use presto_core::data::models::{Item, ItemKind};
use presto_core::data::view::{ListState, Phase, detail_key, tracks_key};
use presto_core::paths::Paths;
use presto_core::EngineStatus;
use presto_ipc::AuthState;
use std::time::Duration;
use tokio::sync::watch;

const T: Duration = Duration::from_secs(5);

fn item(id: &str, kind: ItemKind, library: bool) -> Item {
    Item {
        id: id.into(), kind, library, name: String::new(), subtitle: None, album: None, duration_ms: None,
        artwork: None, play_params: None, release_date: None, track_count: None, playable: true,
    }
}

async fn ready(r: &Rig, paths: &Paths) -> DataHandle {
    let mut rx = r.core.state();
    wait_for(&mut rx, T, |s| s.engine == EngineStatus::Ready && s.auth == Some(AuthState::SignedIn)).await;
    DataHandle::new(r.core.clone(), paths).unwrap()
}

async fn until<X: Clone>(rx: &mut watch::Receiver<ListState<X>>, pred: impl Fn(&ListState<X>) -> bool) -> ListState<X> {
    tokio::time::timeout(T, rx.wait_for(|s| pred(s))).await.expect("timed out").unwrap().clone()
}

fn ids(v: &[Item]) -> Vec<&str> {
    v.iter().map(|i| i.id.as_str()).collect()
}

async fn setup() -> (Rig, DataHandle, tempfile::TempDir) {
    let r = rig(|_| vec![]).await;
    let dir = tempfile::tempdir().unwrap();
    let d = ready(&r, &Paths::under(dir.path().join("data"))).await;
    (r, d, dir)
}

#[tokio::test(flavor = "multi_thread")]
async fn album_detail() {
    let (r, d, _g) = setup().await;
    let k = detail_key(&item("al3", ItemKind::Album, false)).unwrap();
    let s = until(&mut d.detail(k), |s| !s.items.is_empty() && s.phase == Phase::Idle).await;
    let x = &s.items[0];
    assert_eq!((x.head.name.as_str(), x.head.release_date.as_deref(), x.head.track_count), ("Night Shift", Some("2023-10-06"), Some(3)));
    assert_eq!(ids(&x.tracks), ["s8", "s9", "s10"]);
    r.core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn playlist_tracks_paging() {
    let (r, d, _g) = setup().await;
    let k = tracks_key(&item("p4", ItemKind::Playlist, false)).unwrap();
    let mut rx = d.list(k.clone());
    let s = until(&mut rx, |s| s.items.len() == 100 && s.phase == Phase::Idle).await;
    assert!(s.has_more);
    d.load_more(&k);
    let s = until(&mut rx, |s| s.items.len() == 150 && s.phase == Phase::Idle).await;
    assert!(!s.has_more);
    r.core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn library_album_tracks() {
    let (r, d, _g) = setup().await;
    let k = tracks_key(&item("al1", ItemKind::Album, true)).unwrap();
    let s = until(&mut d.list(k), |s| !s.items.is_empty() && s.phase == Phase::Idle).await;
    assert_eq!(ids(&s.items), ["s1", "s2", "s3"]);
    r.core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn artist_detail() {
    let (r, d, _g) = setup().await;
    let k = detail_key(&item("a1", ItemKind::Artist, false)).unwrap();
    let s = until(&mut d.detail(k), |s| !s.items.is_empty() && s.phase == Phase::Idle).await;
    let x = &s.items[0];
    assert!(!x.top_songs.is_empty());
    assert_eq!((ids(&x.albums), ids(&x.singles)), (vec!["al1", "al3"], vec!["al5"]));
    r.core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn unavailable_flag() {
    let (r, d, _g) = setup().await;
    let k = tracks_key(&item("p3", ItemKind::Playlist, false)).unwrap();
    let s = until(&mut d.list(k), |s| !s.items.is_empty() && s.phase == Phase::Idle).await;
    assert_eq!(s.items.iter().map(|i| i.playable).collect::<Vec<_>>(), [false, true, true, true]);
    r.core.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn detail_cached_offline() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path().join("data"));
    let k = detail_key(&item("al3", ItemKind::Album, false)).unwrap();
    let a = rig(|_| vec![]).await;
    let da = ready(&a, &paths).await;
    until(&mut da.detail(k.clone()), |s| !s.items.is_empty() && s.phase == Phase::Idle).await;
    drop(da);
    a.core.shutdown().await;

    let b = rig(|_| vec!["--bridge-missing".to_string()]).await;
    wait_for(&mut b.core.state(), T, |s| matches!(s.engine, EngineStatus::Drift { .. } | EngineStatus::Failed { .. })).await;
    let d = DataHandle::new(b.core.clone(), &paths).unwrap();
    let s = until(&mut d.detail(k), |s| !s.items.is_empty()).await;
    assert!(s.from_cache);
    b.core.shutdown().await;
}

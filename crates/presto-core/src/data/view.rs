//! Per-view state and policies.
use std::time::{Duration, Instant, SystemTime};

use presto_ipc::ApiRequest;
use serde::Serialize;

use super::error::UiErrorKind;
use super::models::{Item, ItemKind};

/// Placeholder for the storefront in paths; DataHandle substitutes it.
pub const SF: &str = "{sf}";
pub const TTL: Duration = Duration::from_secs(3600);
pub const MAX_AUTO_RETRIES: u8 = 3;
pub const RETRY_LADDER_MS: [u64; 3] = [5_000, 10_000, 20_000];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum LibKind {
    Songs,
    Albums,
    Artists,
    Playlists,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum Sort {
    Default,
    NameAsc,
    NameDesc,
    AddedNewest,
    AddedOldest,
}

impl Sort {
    pub fn param(self) -> Option<&'static str> {
        match self {
            Sort::Default => None,
            Sort::NameAsc => Some("name"),
            Sort::NameDesc => Some("-name"),
            Sort::AddedOldest => Some("dateAdded"),
            Sort::AddedNewest => Some("-dateAdded"),
        }
    }
}

/// Sorts Apple accepted in the 04-02 live probe. Others fall back to Apple's order (D-08).
/// Artists reject dateAdded with HTTP 400.
pub const SORT_SUPPORT: &[(LibKind, &[Sort])] = &[
    (LibKind::Songs, &[Sort::NameAsc, Sort::NameDesc, Sort::AddedOldest, Sort::AddedNewest]),
    (LibKind::Albums, &[Sort::NameAsc, Sort::NameDesc, Sort::AddedOldest, Sort::AddedNewest]),
    (LibKind::Artists, &[Sort::NameAsc, Sort::NameDesc]),
    (LibKind::Playlists, &[Sort::NameAsc, Sort::NameDesc, Sort::AddedOldest, Sort::AddedNewest]),
];

fn supported(kind: LibKind) -> &'static [Sort] {
    SORT_SUPPORT.iter().find(|(k, _)| *k == kind).map(|(_, s)| *s).unwrap_or(&[])
}

/// Always starts with `Sort::Default`.
pub fn sorts(kind: LibKind) -> Vec<Sort> {
    std::iter::once(Sort::Default).chain(supported(kind).iter().copied()).collect()
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum ViewKey {
    Library { kind: LibKind, sort: Sort },
    RecentlyPlayed,
    Recommendations,
    /// See all page; unused while 04-02 found no paged contents route.
    Shelf { path: String },
    /// Album or playlist track list.
    Tracks { path: String },
    /// Album, playlist or artist head with inline lists. Not paged.
    Detail { path: String, query: Vec<(String, String)> },
}

impl ViewKey {
    pub fn path(&self) -> String {
        match self {
            ViewKey::Library { kind, .. } => format!(
                "/v1/me/library/{}",
                match kind {
                    LibKind::Songs => "songs",
                    LibKind::Albums => "albums",
                    LibKind::Artists => "artists",
                    LibKind::Playlists => "playlists",
                }
            ),
            ViewKey::RecentlyPlayed => "/v1/me/recent/played".into(),
            ViewKey::Recommendations => "/v1/me/recommendations".into(),
            ViewKey::Shelf { path } | ViewKey::Tracks { path } | ViewKey::Detail { path, .. } => path.clone(),
        }
    }

    /// Library max is 100 (101 is rejected, 04-02). Others stay at 10.
    pub fn page_size(&self) -> u32 {
        match self {
            ViewKey::Library { .. } | ViewKey::Tracks { .. } => 100,
            _ => 10,
        }
    }

    pub fn always_revalidate(&self) -> bool {
        matches!(self, ViewKey::RecentlyPlayed)
    }

    pub fn request(&self, offset: u32) -> ApiRequest {
        let mut r = ApiRequest::get(self.path());
        if let ViewKey::Detail { query, .. } = self {
            for (k, v) in query {
                r.query.insert(k.clone(), v.clone());
            }
            return r;
        }
        r.query.insert("limit".into(), self.page_size().to_string());
        r.query.insert("offset".into(), offset.to_string());
        if let ViewKey::Library { kind, sort } = self {
            if let (true, Some(p)) = (supported(*kind).contains(sort), sort.param()) {
                r.query.insert("sort".into(), p.into());
            }
        }
        r
    }
}

fn detail_base(item: &Item) -> Option<String> {
    let kind = match item.kind {
        ItemKind::Album => "albums",
        ItemKind::Playlist => "playlists",
        _ => return None,
    };
    Some(if item.library {
        format!("/v1/me/library/{kind}/{}", item.id)
    } else {
        format!("/v1/catalog/{SF}/{kind}/{}", item.id)
    })
}

pub fn tracks_key(item: &Item) -> Option<ViewKey> {
    detail_base(item).map(|p| ViewKey::Tracks { path: format!("{p}/tracks") })
}

pub fn detail_key(item: &Item) -> Option<ViewKey> {
    let q = |k: &str, v: &str| vec![(k.to_owned(), v.to_owned())];
    if let Some(path) = detail_base(item) {
        return Some(ViewKey::Detail { path, query: vec![] });
    }
    if item.kind != ItemKind::Artist {
        return None;
    }
    Some(if item.library {
        ViewKey::Detail { path: format!("/v1/me/library/artists/{}", item.id), query: q("include", "catalog") }
    } else {
        ViewKey::Detail {
            path: format!("/v1/catalog/{SF}/artists/{}", item.id),
            query: q("views", "top-songs,full-albums,singles"),
        }
    })
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub enum Phase {
    #[default]
    Idle,
    Loading,
    Revalidating,
    LoadingMore,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ErrorDisplay {
    Banner,
    FullView,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UiError {
    pub kind: UiErrorKind,
    pub attempts: u8,
    pub retry_at: Option<Instant>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ListState<T> {
    pub items: Vec<T>,
    pub total: Option<u64>,
    pub has_more: bool,
    pub phase: Phase,
    pub from_cache: bool,
    pub fetched_at: Option<SystemTime>,
    pub updated_at: Option<Instant>,
    pub error: Option<UiError>,
}

impl<T> Default for ListState<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            total: None,
            has_more: false,
            phase: Phase::Idle,
            from_cache: false,
            fetched_at: None,
            updated_at: None,
            error: None,
        }
    }
}

impl<T> ListState<T> {
    /// Banner when items exist (D-10), full view when empty (D-12).
    pub fn error_display(&self) -> Option<ErrorDisplay> {
        self.error.as_ref().map(|_| if self.items.is_empty() { ErrorDisplay::FullView } else { ErrorDisplay::Banner })
    }
}

pub fn is_stale(key: ViewKey, fetched_at_ms: i64, now_ms: i64, force: bool, invalidated_at_ms: i64) -> bool {
    force
        || key.always_revalidate()
        || fetched_at_ms < invalidated_at_ms
        || now_ms - fetched_at_ms >= TTL.as_millis() as i64
}

pub fn next_retry(kind: &UiErrorKind, attempts: u8, now: Instant) -> Option<Instant> {
    match kind {
        UiErrorKind::RateLimited { retry_after_ms } if attempts < MAX_AUTO_RETRIES => {
            let ms = retry_after_ms.unwrap_or(RETRY_LADDER_MS[attempts as usize]);
            Some(now + Duration::from_millis(ms))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: i64 = 60_000;
    fn lib(kind: LibKind, sort: Sort) -> ViewKey {
        ViewKey::Library { kind, sort }
    }

    #[test]
    fn paths_and_sizes() {
        let k = lib(LibKind::Songs, Sort::Default);
        assert_eq!((k.path().as_str(), k.page_size()), ("/v1/me/library/songs", 100));
        assert_eq!((ViewKey::RecentlyPlayed.path().as_str(), ViewKey::RecentlyPlayed.page_size()), ("/v1/me/recent/played", 10));
        assert_eq!(ViewKey::Recommendations.path(), "/v1/me/recommendations");
        let s = ViewKey::Shelf { path: "/x".into() };
        assert_eq!((s.path().as_str(), s.page_size()), ("/x", 10));
    }

    #[test]
    fn request_query() {
        let r = lib(LibKind::Songs, Sort::Default).request(200);
        assert_eq!(r.query["limit"], "100");
        assert_eq!(r.query["offset"], "200");
        assert!(!r.query.contains_key("sort"));
        for (s, p) in [(Sort::NameAsc, "name"), (Sort::NameDesc, "-name"), (Sort::AddedOldest, "dateAdded"), (Sort::AddedNewest, "-dateAdded")] {
            assert_eq!(lib(LibKind::Songs, s).request(0).query["sort"], p);
        }
    }

    #[test]
    fn unsupported_sort_omitted() {
        assert!(!lib(LibKind::Artists, Sort::AddedNewest).request(0).query.contains_key("sort"));
    }

    #[test]
    fn sorts_list() {
        assert_eq!(sorts(LibKind::Artists), vec![Sort::Default, Sort::NameAsc, Sort::NameDesc]);
        assert_eq!(sorts(LibKind::Songs).len(), 5);
    }

    fn it(id: &str, kind: ItemKind, library: bool) -> Item {
        Item {
            id: id.into(), kind, library, name: String::new(), subtitle: None, album: None, duration_ms: None,
            artwork: None, play_params: None, release_date: None, track_count: None, playable: true,
        }
    }

    #[test]
    fn detail_and_track_keys() {
        let k = tracks_key(&it("al3", ItemKind::Album, false)).unwrap();
        assert_eq!((k.path().as_str(), k.page_size()), ("/v1/catalog/{sf}/albums/al3/tracks", 100));
        assert_eq!(tracks_key(&it("p.x", ItemKind::Playlist, true)).unwrap().path(), "/v1/me/library/playlists/p.x/tracks");
        let k = detail_key(&it("a1", ItemKind::Artist, false)).unwrap();
        let r = k.request(0);
        assert_eq!(k.path(), "/v1/catalog/{sf}/artists/a1");
        assert_eq!(r.query["views"], "top-songs,full-albums,singles");
        assert!(!r.query.contains_key("limit") && !r.query.contains_key("offset"));
        assert_eq!(detail_key(&it("a1", ItemKind::Artist, true)).unwrap().request(0).query["include"], "catalog");
        assert!(tracks_key(&it("s", ItemKind::Song, false)).is_none() && detail_key(&it("s", ItemKind::Song, false)).is_none());
    }

    #[test]
    fn staleness() {
        let k = || lib(LibKind::Songs, Sort::Default);
        let now = 100 * MIN;
        assert!(!is_stale(k(), now - 59 * MIN, now, false, 0));
        assert!(is_stale(k(), now - 61 * MIN, now, false, 0));
        assert!(is_stale(k(), now - MIN, now, true, 0));
        assert!(is_stale(ViewKey::RecentlyPlayed, now - 1000, now, false, 0));
        assert!(is_stale(k(), now - MIN, now, false, now - MIN + 1));
    }

    #[test]
    fn error_display() {
        let mut s = ListState::<u8>::default();
        assert_eq!(s.error_display(), None);
        s.error = Some(UiError { kind: UiErrorKind::Timeout, attempts: 0, retry_at: None });
        assert_eq!(s.error_display(), Some(ErrorDisplay::FullView));
        s.items.push(1);
        assert_eq!(s.error_display(), Some(ErrorDisplay::Banner));
    }

    #[test]
    fn retry_policy() {
        let now = Instant::now();
        let rl = |ms| UiErrorKind::RateLimited { retry_after_ms: ms };
        assert_eq!(next_retry(&rl(Some(1500)), 0, now), Some(now + Duration::from_millis(1500)));
        for (a, ms) in [(0, 5000), (1, 10_000), (2, 20_000)] {
            assert_eq!(next_retry(&rl(None), a, now), Some(now + Duration::from_millis(ms)));
        }
        assert_eq!(next_retry(&rl(None), 3, now), None);
        assert_eq!(next_retry(&UiErrorKind::Timeout, 0, now), None);
        assert_eq!(next_retry(&UiErrorKind::NotFound, 0, now), None);
    }
}

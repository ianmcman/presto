//! Tolerant JSON models. Parsed by hand from `Value`: unknown fields, unknown types and
//! missing artwork never fail.
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Artwork {
    pub url: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum ItemKind {
    Song,
    Album,
    Artist,
    Playlist,
    Station,
    Other(String),
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Item {
    pub id: String,
    pub kind: ItemKind,
    pub library: bool,
    pub name: String,
    /// artistName, else curatorName
    pub subtitle: Option<String>,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
    pub artwork: Option<Artwork>,
    /// Raw playParams, for Phase 5.
    pub play_params: Option<Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Shelf {
    pub id: String,
    pub title: String,
    pub items: Vec<Item>,
    pub see_all: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct SearchResults {
    pub term: String,
    pub top: Vec<Item>,
    pub songs: Vec<Item>,
    pub albums: Vec<Item>,
    pub artists: Vec<Item>,
    pub playlists: Vec<Item>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next: Option<String>,
    pub total: Option<u64>,
}

pub trait Rows: Clone + Send + Sync + 'static {
    fn parse_page(v: &Value) -> Page<Self>;
}

fn s(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(Value::as_str).map(str::to_owned)
}

fn data_items(v: &Value) -> impl Iterator<Item = &Value> {
    v.get("data").and_then(Value::as_array).into_iter().flatten()
}

fn page<T>(v: &Value, items: Vec<T>) -> Page<T> {
    Page {
        items,
        next: s(v, "next"),
        total: v.get("meta").and_then(|m| m.get("total")).and_then(Value::as_u64),
    }
}

pub fn parse_item(v: &Value) -> Option<Item> {
    let id = s(v, "id").filter(|i| !i.is_empty())?;
    let ty = s(v, "type").unwrap_or_default();
    let (library, base) = match ty.strip_prefix("library-") {
        Some(b) => (true, b),
        None => (false, ty.as_str()),
    };
    let kind = match base {
        "songs" => ItemKind::Song,
        "albums" => ItemKind::Album,
        "artists" => ItemKind::Artist,
        "playlists" => ItemKind::Playlist,
        "stations" => ItemKind::Station,
        _ => ItemKind::Other(ty.clone()),
    };
    let null = Value::Null;
    let a = v.get("attributes").unwrap_or(&null);
    let artwork = a.get("artwork").and_then(|w| {
        Some(Artwork {
            url: s(w, "url")?,
            width: w.get("width").and_then(Value::as_u64).map(|n| n as u32),
            height: w.get("height").and_then(Value::as_u64).map(|n| n as u32),
        })
    });
    Some(Item {
        id,
        kind,
        library,
        name: s(a, "name").unwrap_or_default(),
        subtitle: s(a, "artistName").or_else(|| s(a, "curatorName")),
        album: s(a, "albumName"),
        duration_ms: a.get("durationInMillis").and_then(Value::as_u64),
        artwork,
        play_params: a.get("playParams").cloned(),
    })
}

impl Rows for Item {
    fn parse_page(v: &Value) -> Page<Self> {
        page(v, data_items(v).filter_map(parse_item).collect())
    }
}

impl Rows for Shelf {
    fn parse_page(v: &Value) -> Page<Self> {
        let shelves = data_items(v)
            .filter_map(|g| {
                let id = s(g, "id")?;
                let title = g
                    .pointer("/attributes/title/stringForDisplay")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned();
                let items = data_items(g.pointer("/relationships/contents").unwrap_or(&Value::Null))
                    .filter_map(parse_item)
                    .collect();
                // 04-02: See all uses the inline contents; no paged route needed.
                Some(Shelf { id, title, items, see_all: None })
            })
            .collect();
        page(v, shelves)
    }
}

fn group(v: &Value, keys: &[&str]) -> Vec<Item> {
    let Some(r) = v.get("results") else { return vec![] };
    keys.iter()
        .find_map(|k| r.get(*k))
        .map(|g| data_items(g).filter_map(parse_item).collect())
        .unwrap_or_default()
}

pub fn parse_search(term: &str, v: &Value) -> SearchResults {
    SearchResults {
        term: term.to_owned(),
        top: group(v, &["topResults", "top"]),
        songs: group(v, &["songs", "library-songs"]),
        albums: group(v, &["albums", "library-albums"]),
        artists: group(v, &["artists", "library-artists"]),
        playlists: group(v, &["playlists", "library-playlists"]),
    }
}

pub fn parse_hints(v: &Value) -> Vec<String> {
    v.pointer("/results/terms")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|t| t.as_str().map(str::to_owned)).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn song(id: &str) -> Value {
        json!({"id": id, "type": "songs", "attributes": {"name": "N", "artistName": "A", "albumName": "Al",
            "durationInMillis": 1000, "playParams": {"id": id},
            "artwork": {"url": "https://x/{w}x{h}bb.jpg", "width": 600, "height": 600}}})
    }

    #[test]
    fn song_fields() {
        let i = parse_item(&song("s1")).unwrap();
        assert_eq!(i.kind, ItemKind::Song);
        assert!(!i.library);
        assert_eq!(i.subtitle.as_deref(), Some("A"));
        assert_eq!(i.album.as_deref(), Some("Al"));
        assert_eq!(i.duration_ms, Some(1000));
        assert!(i.artwork.unwrap().url.contains("{w}x{h}"));
        assert!(i.play_params.is_some());
    }

    #[test]
    fn library_prefix() {
        let i = parse_item(&json!({"id": "p", "type": "library-playlists"})).unwrap();
        assert_eq!((i.kind, i.library), (ItemKind::Playlist, true));
        let i = parse_item(&json!({"id": "p", "type": "library-songs"})).unwrap();
        assert_eq!((i.kind, i.library), (ItemKind::Song, true));
    }

    #[test]
    fn other_kinds() {
        assert_eq!(parse_item(&json!({"id": "1", "type": "stations"})).unwrap().kind, ItemKind::Station);
        assert_eq!(
            parse_item(&json!({"id": "1", "type": "music-videos"})).unwrap().kind,
            ItemKind::Other("music-videos".into())
        );
    }

    #[test]
    fn missing_artwork_and_id() {
        assert!(parse_item(&json!({"id": "1", "type": "songs", "attributes": {}})).unwrap().artwork.is_none());
        assert!(parse_item(&json!({"type": "songs"})).is_none());
    }

    #[test]
    fn curator_subtitle() {
        let i = parse_item(&json!({"id": "1", "type": "playlists", "attributes": {"curatorName": "Apple"}})).unwrap();
        assert_eq!(i.subtitle.as_deref(), Some("Apple"));
    }

    #[test]
    fn item_page() {
        let p = Item::parse_page(&json!({"data": [song("a"), song("b")], "next": "/x?offset=2", "meta": {"total": 9}}));
        assert_eq!((p.items.len(), p.next.as_deref(), p.total), (2, Some("/x?offset=2"), Some(9)));
        let p = Item::parse_page(&json!({}));
        assert_eq!((p.items.len(), p.next, p.total), (0, None, None));
    }

    #[test]
    fn shelf_page() {
        let g = |id: &str, t: &str| {
            json!({"id": id, "type": "personal-recommendation",
                "attributes": {"title": {"stringForDisplay": t}},
                "relationships": {"contents": {"data": [song("a"), song("b")]}}})
        };
        let p = Shelf::parse_page(&json!({"data": [g("r1", "Made for You"), g("r2", "Albums You Might Like")]}));
        assert_eq!(p.items.len(), 2);
        assert_eq!(p.items[0].title, "Made for You");
        assert_eq!(p.items[1].title, "Albums You Might Like");
        assert!(p.items.iter().all(|s| s.items.len() == 2));
    }

    #[test]
    fn search_groups() {
        let r = parse_search(
            "neon",
            &json!({"results": {"topResults": {"data": [song("s1")]}, "songs": {"data": [song("s1")]}}}),
        );
        assert_eq!((r.term.as_str(), r.top.len(), r.songs.len(), r.albums.len()), ("neon", 1, 1, 0));
        let r = parse_search("a", &json!({"results": {"library-songs": {"data": [song("s2")]}}}));
        assert_eq!(r.songs.len(), 1);
        assert!(r.playlists.is_empty());
    }

    #[test]
    fn hints() {
        assert_eq!(parse_hints(&json!({"results": {"terms": ["a", "b"]}})), vec!["a", "b"]);
        assert!(parse_hints(&json!({})).is_empty());
    }
}

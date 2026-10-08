//! Fixed canned catalog and Apple-shaped API routes.

use presto_ipc::{ApiRequest, ErrorKind, HttpMethod, IpcError, Outcome, QueueItem};
use serde_json::{Value, json};

// (id, title, artist, album, duration_ms)
const SONGS: &[(&str, &str, &str, &str, u64)] = &[
    (
        "s1",
        "Neon Static",
        "Mock Artist One",
        "Neon Static",
        183000,
    ),
    (
        "s2",
        "Low Battery",
        "Mock Artist One",
        "Neon Static",
        204000,
    ),
    ("s3", "Cache Miss", "Mock Artist One", "Neon Static", 241000),
    ("s4", "Hang Time", "The Placeholders", "Fault Lines", 167000),
    ("s5", "Slow Burn", "The Placeholders", "Fault Lines", 199000),
    (
        "s6",
        "Crash Course",
        "The Placeholders",
        "Fault Lines",
        222000,
    ),
];
// (id, name, artist)
const ARTISTS: &[(&str, &str)] = &[("a1", "Mock Artist One"), ("a2", "The Placeholders")];
// (id, name, artist, song ids)
const ALBUMS: &[(&str, &str, &str, &[&str])] = &[
    ("al1", "Neon Static", "Mock Artist One", &["s1", "s2", "s3"]),
    (
        "al2",
        "Fault Lines",
        "The Placeholders",
        &["s4", "s5", "s6"],
    ),
];
const PLAYLISTS: &[(&str, &str, &[&str])] = &[
    ("p1", "Mock Mix", &["s1", "s4", "s2"]),
    ("p2", "Late Night", &["s5", "s6"]),
];

fn art(id: &str, size: &str) -> String {
    format!("https://example.invalid/artwork/{id}/{size}.jpg")
}

fn artwork(id: &str) -> Value {
    json!({"url": art(id, "{w}x{h}"), "width": 1000, "height": 1000})
}

pub fn song(id: &str) -> Option<QueueItem> {
    SONGS.iter().find(|s| s.0 == id).map(|s| QueueItem {
        id: s.0.into(),
        title: s.1.into(),
        artist: s.2.into(),
        album: s.3.into(),
        duration_ms: s.4,
        artwork_url: Some(art(s.0, "600x600")),
        playable: true,
    })
}

fn song_res(id: &str) -> Value {
    let s = SONGS.iter().find(|s| s.0 == id).expect("known song");
    json!({"id": s.0, "type": "songs", "attributes": {
        "name": s.1, "artistName": s.2, "albumName": s.3,
        "durationInMillis": s.4, "artwork": artwork(s.0)}})
}

fn tracks(ids: &[&str]) -> Vec<Value> {
    ids.iter().map(|i| song_res(i)).collect()
}

/// All resources of a type as (name, json).
fn all(ty: &str) -> Vec<(&'static str, Value)> {
    match ty {
        "songs" => SONGS.iter().map(|s| (s.1, song_res(s.0))).collect(),
        "artists" => ARTISTS
            .iter()
            .map(|a| {
                (
                    a.1,
                    json!({"id": a.0, "type": "artists",
                    "attributes": {"name": a.1, "artwork": artwork(a.0)}}),
                )
            })
            .collect(),
        "albums" => ALBUMS
            .iter()
            .map(|a| {
                (
                    a.1,
                    json!({"id": a.0, "type": "albums", "attributes": {
                    "name": a.1, "artistName": a.2, "artwork": artwork(a.0),
                    "trackCount": a.3.len()},
                    "relationships": {"tracks": {"data": tracks(a.3)}}}),
                )
            })
            .collect(),
        "playlists" => PLAYLISTS
            .iter()
            .map(|p| {
                (
                    p.1,
                    json!({"id": p.0, "type": "playlists", "attributes": {
                    "name": p.1, "curatorName": "Presto Mock", "artwork": artwork(p.0),
                    "trackCount": p.2.len()},
                    "relationships": {"tracks": {"data": tracks(p.2)}}}),
                )
            })
            .collect(),
        _ => vec![],
    }
}

fn not_found(msg: String) -> Outcome {
    Outcome::Err {
        error: IpcError::new(ErrorKind::NotFound, msg),
    }
}

fn is_type(t: &str) -> bool {
    matches!(t, "songs" | "albums" | "artists" | "playlists")
}

pub struct Catalog {
    pub storefront: String,
    pub library_songs: usize,
}

impl Default for Catalog {
    fn default() -> Self {
        Catalog {
            storefront: "us".into(),
            library_songs: 0,
        }
    }
}

fn bad_request(msg: &str) -> Outcome {
    Outcome::Err {
        error: IpcError::new(ErrorKind::Upstream { status: 400 }, msg),
    }
}

fn gen_song(n: usize) -> Value {
    let id = format!("i.{n:05}");
    json!({"id": id, "type": "library-songs", "attributes": {
        "name": format!("Song {n:05}"),
        "artistName": format!("Gen Artist {}", n % 50),
        "albumName": format!("Gen Album {}", n % 500),
        "durationInMillis": 200000,
        "artwork": artwork(&id),
        "playParams": {"id": id, "kind": "song", "isLibrary": true}}})
}

/// One page over `total` items computed per index; `next` is a path with `offset` only.
fn page(
    total: usize,
    limit: usize,
    offset: usize,
    path: &str,
    item: impl Fn(usize) -> Value,
) -> Value {
    let end = total.min(offset.saturating_add(limit));
    let data: Vec<Value> = (offset.min(end)..end).map(item).collect();
    let mut out = json!({"data": data});
    if end < total {
        out["next"] = json!(format!("{path}?offset={end}"));
    }
    out
}

fn group(id: &str, title: &str, ids: &[&str], ty: &str) -> Value {
    let contents: Vec<Value> = all(ty)
        .into_iter()
        .map(|i| i.1)
        .filter(|r| ids.iter().any(|i| r["id"] == *i))
        .collect();
    json!({"id": id, "type": "personal-recommendation",
        "attributes": {"title": {"stringForDisplay": title}, "kind": "music-recommendations"},
        "relationships": {"contents": {"data": contents}}})
}

pub fn handle(req: &ApiRequest, cat: &Catalog) -> Outcome {
    if req.method != HttpMethod::Get {
        return Outcome::Err {
            error: IpcError::new(ErrorKind::Unavailable, "mock engine is read-only"),
        };
    }
    let (path, qs) = req.path.split_once('?').unwrap_or((&req.path, ""));
    let mut query = req.query.clone();
    for kv in qs.split('&').filter(|s| !s.is_empty()) {
        let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
        query.entry(k.into()).or_insert_with(|| v.into());
    }
    let num = |k: &str, d: usize| query.get(k).and_then(|v| v.parse().ok()).unwrap_or(d);
    let term = query
        .get("term")
        .map(|t| t.to_lowercase())
        .unwrap_or_default();
    let hits = |ty: &str| -> Vec<Value> {
        if term.is_empty() {
            return vec![];
        }
        all(ty)
            .into_iter()
            .filter(|i| i.0.to_lowercase().contains(&term))
            .map(|i| i.1)
            .collect()
    };
    let segs: Vec<&str> = path.trim_matches('/').split('/').collect();
    match segs.as_slice() {
        ["v1", "catalog", sf, ..] if *sf != cat.storefront => {
            not_found(format!("mock storefront is {}", cat.storefront))
        }
        ["v1", "me", "storefront"] => Outcome::Ok {
            data: json!({"data": [{"id": cat.storefront, "type": "storefronts",
                "attributes": {"name": "Mock Storefront"}}]}),
        },
        ["v1", "me", "library", "search"] => {
            let mut results = serde_json::Map::new();
            let types = query.get("types").cloned().unwrap_or_default();
            for t in types.split(',') {
                let Some(ty) = t.strip_prefix("library-").filter(|ty| is_type(ty)) else {
                    continue;
                };
                let data: Vec<Value> = hits(ty)
                    .into_iter()
                    .map(|mut r| {
                        r["type"] = json!(t);
                        r
                    })
                    .collect();
                results.insert(t.into(), json!({"data": data}));
            }
            Outcome::Ok {
                data: json!({"results": results}),
            }
        }
        ["v1", "me", "library", ty] if is_type(ty) => {
            let (limit, offset) = (num("limit", 25), num("offset", 0));
            if limit > 100 {
                return bad_request("mock: library limit max is 100");
            }
            let p = format!("/v1/me/library/{ty}");
            let out = if *ty == "songs" && cat.library_songs > 0 {
                let t = cat.library_songs;
                let mut o = page(t, limit, offset, &p, gen_song);
                o["meta"] = json!({"total": t});
                o
            } else {
                let items = all(ty);
                let mut o = page(items.len(), limit, offset, &p, |i| items[i].1.clone());
                o["meta"] = json!({"total": items.len()});
                o
            };
            Outcome::Ok { data: out }
        }
        ["v1", "me", "recent", "played"] => {
            let (limit, offset) = (num("limit", 10), num("offset", 0));
            if limit > 10 {
                return bad_request("mock: recent limit max is 10");
            }
            let (albums, lists) = (all("albums"), all("playlists"));
            let mixed: Vec<Value> = albums
                .into_iter()
                .zip(lists)
                .flat_map(|(a, p)| [a.1, p.1])
                .collect();
            Outcome::Ok {
                data: page(mixed.len(), limit, offset, path, |i| mixed[i].clone()),
            }
        }
        ["v1", "me", "recommendations"] => {
            let groups = [
                group("rec1", "Made for You", &["p1", "p2"], "playlists"),
                group("rec2", "Albums You Might Like", &["al1", "al2"], "albums"),
            ];
            Outcome::Ok {
                data: page(
                    groups.len(),
                    num("limit", 10),
                    num("offset", 0),
                    path,
                    |i| groups[i].clone(),
                ),
            }
        }
        ["v1", "catalog", _, "search", "hints"] => {
            let mut terms: Vec<String> = vec![];
            if !term.is_empty() {
                for ty in ["songs", "artists", "albums", "playlists"] {
                    for (name, _) in all(ty) {
                        let n = name.to_lowercase();
                        // word-prefix match, as real hints do
                        if n.split_whitespace().any(|w| w.starts_with(&term)) && !terms.contains(&n)
                        {
                            terms.push(n);
                        }
                    }
                }
            }
            terms.truncate(num("limit", 10));
            Outcome::Ok {
                data: json!({"results": {"terms": terms}}),
            }
        }
        ["v1", "me", "library", ty, id] | ["v1", "catalog", _, ty, id] if is_type(ty) => {
            match all(ty).into_iter().find(|i| i.1["id"] == *id) {
                Some((_, r)) => Outcome::Ok {
                    data: json!({"data": [r]}),
                },
                None => not_found(format!("mock engine has no {ty} {id}")),
            }
        }
        ["v1", "catalog", _, "search"] => {
            let songs = hits("songs");
            let mut results = json!({
                "songs": {"data": songs},
                "albums": {"data": hits("albums")},
                "artists": {"data": hits("artists")},
                "playlists": {"data": hits("playlists")}});
            let top = query.get("with").is_some_and(|w| w.contains("topResults"));
            if let (true, Some(first)) = (top, results["songs"]["data"].get(0).cloned()) {
                results["top"] = json!({"data": [first]});
            }
            Outcome::Ok {
                data: json!({"results": results}),
            }
        }
        _ => not_found(format!("mock engine has no route for {path}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data(o: Outcome) -> Value {
        match o {
            Outcome::Ok { data } => data,
            Outcome::Err { error } => panic!("{error:?}"),
        }
    }

    #[test]
    fn library_pagination() {
        let mut r = ApiRequest::get("/v1/me/library/songs");
        r.query.insert("limit".into(), "2".into());
        let d = data(handle(&r, &Catalog::default()));
        assert_eq!(d["data"].as_array().unwrap().len(), 2);
        assert_eq!(d["next"], "/v1/me/library/songs?offset=2");
        let d = get("/v1/me/library/songs?offset=4&limit=5");
        assert_eq!(d["data"].as_array().unwrap().len(), 2);
        assert!(d.get("next").is_none());
    }

    #[test]
    fn search_is_case_insensitive() {
        let mut r = ApiRequest::get("/v1/catalog/us/search");
        r.query.insert("term".into(), "NEON".into());
        let d = data(handle(&r, &Catalog::default()));
        let songs = d["results"]["songs"]["data"].as_array().unwrap();
        assert_eq!(songs.len(), 1);
        assert_eq!(songs[0]["attributes"]["name"], "Neon Static");
    }

    #[test]
    fn lookups_and_errors() {
        let d = data(handle(
            &ApiRequest::get("/v1/catalog/us/albums/al1"),
            &Catalog::default(),
        ));
        assert_eq!(
            d["data"][0]["relationships"]["tracks"]["data"]
                .as_array()
                .unwrap()
                .len(),
            3
        );
        assert!(matches!(
            handle(&ApiRequest::get("/v1/nope"), &Catalog::default()),
            Outcome::Err {
                error: IpcError {
                    kind: ErrorKind::NotFound,
                    ..
                }
            }
        ));
        let mut post = ApiRequest::get("/v1/me/library/songs");
        post.method = HttpMethod::Post;
        assert!(matches!(
            handle(&post, &Catalog::default()),
            Outcome::Err {
                error: IpcError {
                    kind: ErrorKind::Unavailable,
                    ..
                }
            }
        ));
        assert_eq!(
            song("s2").unwrap().artwork_url.unwrap(),
            "https://example.invalid/artwork/s2/600x600.jpg"
        );
    }

    fn get(path: &str) -> Value {
        data(handle(&ApiRequest::get(path), &Catalog::default()))
    }
    fn is_400(o: Outcome) -> bool {
        matches!(
            o,
            Outcome::Err {
                error: IpcError {
                    kind: ErrorKind::Upstream { status: 400 },
                    ..
                }
            }
        )
    }
    fn ids(d: &Value) -> Vec<String> {
        d["data"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["id"].as_str().unwrap().into())
            .collect()
    }

    #[test]
    fn recent_played() {
        let d = get("/v1/me/recent/played?limit=10");
        assert_eq!(ids(&d), ["al1", "p1", "al2", "p2"]);
        assert!(d.get("next").is_none());
        let d = get("/v1/me/recent/played?limit=2");
        assert_eq!(ids(&d), ["al1", "p1"]);
        assert_eq!(d["next"], "/v1/me/recent/played?offset=2");
        assert!(is_400(handle(
            &ApiRequest::get("/v1/me/recent/played?limit=11"),
            &Catalog::default()
        )));
    }

    #[test]
    fn recommendations() {
        let d = get("/v1/me/recommendations");
        assert_eq!(ids(&d), ["rec1", "rec2"]);
        assert_eq!(
            d["data"][0]["attributes"]["title"]["stringForDisplay"],
            "Made for You"
        );
        assert_eq!(
            ids(&d["data"][1]["relationships"]["contents"]),
            ["al1", "al2"]
        );
    }

    #[test]
    fn hints_and_top_results() {
        assert_eq!(
            get("/v1/catalog/us/search/hints?term=ne")["results"]["terms"],
            json!(["neon static"])
        );
        let d = get("/v1/catalog/us/search?term=neon&with=topResults");
        assert_eq!(d["results"]["top"]["data"][0]["id"], "s1");
        assert!(
            get("/v1/catalog/us/search?term=neon")["results"]
                .get("top")
                .is_none()
        );
    }

    #[test]
    fn library_search() {
        let d = get("/v1/me/library/search?term=neon&types=library-songs,library-albums");
        assert_eq!(
            d["results"]["library-songs"]["data"][0]["type"],
            "library-songs"
        );
        assert_eq!(d["results"]["library-albums"]["data"][0]["id"], "al1");
        assert!(d["results"].get("library-artists").is_none());
    }

    #[test]
    fn storefront_is_checked() {
        let gb = Catalog {
            storefront: "gb".into(),
            library_songs: 0,
        };
        let r = |p: &str| handle(&ApiRequest::get(p), &gb);
        assert!(matches!(
            r("/v1/catalog/us/search"),
            Outcome::Err {
                error: IpcError {
                    kind: ErrorKind::NotFound,
                    ..
                }
            }
        ));
        assert!(matches!(r("/v1/catalog/gb/search"), Outcome::Ok { .. }));
        assert_eq!(data(r("/v1/me/storefront"))["data"][0]["id"], "gb");
    }

    #[test]
    fn big_library() {
        let big = Catalog {
            storefront: "us".into(),
            library_songs: 10000,
        };
        let r = |p: &str| handle(&ApiRequest::get(p), &big);
        let d = data(r("/v1/me/library/songs?limit=100&offset=9950"));
        let got = ids(&d);
        assert_eq!(
            (got.len(), got[0].as_str(), got[49].as_str()),
            (50, "i.09950", "i.09999")
        );
        assert_eq!(d["meta"]["total"], 10000);
        assert!(d.get("next").is_none());
        assert_eq!(
            data(r("/v1/me/library/songs?limit=100"))["next"],
            "/v1/me/library/songs?offset=100"
        );
        assert!(is_400(r("/v1/me/library/songs?limit=101")));
        assert_eq!(get("/v1/me/library/albums")["meta"]["total"], 2);
    }
}

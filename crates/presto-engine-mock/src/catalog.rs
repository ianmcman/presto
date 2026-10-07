//! Fixed canned catalog and Apple-shaped API routes.

use presto_ipc::{ApiRequest, ErrorKind, HttpMethod, IpcError, Outcome, QueueItem};
use serde_json::{Value, json};

// (id, title, artist, album, duration_ms)
const SONGS: &[(&str, &str, &str, &str, u64)] = &[
    ("s1", "Neon Static", "Mock Artist One", "Neon Static", 183000),
    ("s2", "Low Battery", "Mock Artist One", "Neon Static", 204000),
    ("s3", "Cache Miss", "Mock Artist One", "Neon Static", 241000),
    ("s4", "Hang Time", "The Placeholders", "Fault Lines", 167000),
    ("s5", "Slow Burn", "The Placeholders", "Fault Lines", 199000),
    ("s6", "Crash Course", "The Placeholders", "Fault Lines", 222000),
];
// (id, name, artist)
const ARTISTS: &[(&str, &str)] = &[("a1", "Mock Artist One"), ("a2", "The Placeholders")];
// (id, name, artist, song ids)
const ALBUMS: &[(&str, &str, &str, &[&str])] = &[
    ("al1", "Neon Static", "Mock Artist One", &["s1", "s2", "s3"]),
    ("al2", "Fault Lines", "The Placeholders", &["s4", "s5", "s6"]),
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
                (a.1, json!({"id": a.0, "type": "artists",
                    "attributes": {"name": a.1, "artwork": artwork(a.0)}}))
            })
            .collect(),
        "albums" => ALBUMS
            .iter()
            .map(|a| {
                (a.1, json!({"id": a.0, "type": "albums", "attributes": {
                    "name": a.1, "artistName": a.2, "artwork": artwork(a.0),
                    "trackCount": a.3.len()},
                    "relationships": {"tracks": {"data": tracks(a.3)}}}))
            })
            .collect(),
        "playlists" => PLAYLISTS
            .iter()
            .map(|p| {
                (p.1, json!({"id": p.0, "type": "playlists", "attributes": {
                    "name": p.1, "curatorName": "Presto Mock", "artwork": artwork(p.0),
                    "trackCount": p.2.len()},
                    "relationships": {"tracks": {"data": tracks(p.2)}}}))
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

pub fn handle(req: &ApiRequest) -> Outcome {
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
    let segs: Vec<&str> = path.trim_matches('/').split('/').collect();
    match segs.as_slice() {
        ["v1", "me", "storefront"] => Outcome::Ok {
            data: json!({"data": [{"id": "us", "type": "storefronts",
                "attributes": {"name": "United States"}}]}),
        },
        ["v1", "me", "library", ty] if is_type(ty) => {
            let num = |k: &str, d: usize| query.get(k).and_then(|v| v.parse().ok()).unwrap_or(d);
            let (limit, offset) = (num("limit", 25), num("offset", 0));
            let items = all(ty);
            let page: Vec<Value> = items
                .iter()
                .skip(offset)
                .take(limit)
                .map(|i| i.1.clone())
                .collect();
            let mut out = json!({"data": page});
            if offset + limit < items.len() {
                out["next"] = json!(format!("/v1/me/library/{ty}?offset={}", offset + limit));
            }
            Outcome::Ok { data: out }
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
            let term = query.get("term").map(|t| t.to_lowercase()).unwrap_or_default();
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
            Outcome::Ok {
                data: json!({"results": {
                    "songs": {"data": hits("songs")},
                    "albums": {"data": hits("albums")},
                    "artists": {"data": hits("artists")},
                    "playlists": {"data": hits("playlists")}}}),
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
        let d = data(handle(&r));
        assert_eq!(d["data"].as_array().unwrap().len(), 2);
        assert_eq!(d["next"], "/v1/me/library/songs?offset=2");
        let d = data(handle(&ApiRequest::get("/v1/me/library/songs?offset=4&limit=5")));
        assert_eq!(d["data"].as_array().unwrap().len(), 2);
        assert!(d.get("next").is_none());
    }

    #[test]
    fn search_is_case_insensitive() {
        let mut r = ApiRequest::get("/v1/catalog/us/search");
        r.query.insert("term".into(), "NEON".into());
        let d = data(handle(&r));
        let songs = d["results"]["songs"]["data"].as_array().unwrap();
        assert_eq!(songs.len(), 1);
        assert_eq!(songs[0]["attributes"]["name"], "Neon Static");
    }

    #[test]
    fn lookups_and_errors() {
        let d = data(handle(&ApiRequest::get("/v1/catalog/us/albums/al1")));
        assert_eq!(d["data"][0]["relationships"]["tracks"]["data"].as_array().unwrap().len(), 3);
        assert!(matches!(
            handle(&ApiRequest::get("/v1/nope")),
            Outcome::Err { error: IpcError { kind: ErrorKind::NotFound, .. } }
        ));
        let mut post = ApiRequest::get("/v1/me/library/songs");
        post.method = HttpMethod::Post;
        assert!(matches!(
            handle(&post),
            Outcome::Err { error: IpcError { kind: ErrorKind::Unavailable, .. } }
        ));
        assert_eq!(song("s2").unwrap().artwork_url.unwrap(), "https://example.invalid/artwork/s2/600x600.jpg");
    }
}

//! SQLite page cache, search history, meta. Disposable: unknown version or corruption means recreate.
use presto_ipc::ApiRequest;
use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

const VERSION: i64 = 1;

#[derive(Clone)]
pub struct Store {
    conn: Arc<Mutex<Connection>>,
}

pub struct CachedPage {
    pub fetched_at_ms: i64,
    pub body: Vec<u8>,
}

const GET_PAGE: &str = "SELECT fetched_at, body FROM pages WHERE account=?1 AND req=?2 AND off=?3";

/// Path plus query without paging params, so all pages of one list share a key.
pub fn req_key(req: &ApiRequest) -> String {
    let q: Vec<String> = req
        .query
        .iter()
        .filter(|(k, _)| k.as_str() != "offset" && k.as_str() != "limit")
        .map(|(k, v)| format!("{k}={v}"))
        .collect();
    if q.is_empty() { req.path.clone() } else { format!("{}?{}", req.path, q.join("&")) }
}

pub fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}

fn init(c: Connection) -> rusqlite::Result<Connection> {
    let v: i64 = c.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if v > VERSION {
        return Err(rusqlite::Error::InvalidQuery);
    }
    c.execute_batch(
        "PRAGMA journal_mode=WAL;
         CREATE TABLE IF NOT EXISTS pages(account TEXT NOT NULL, req TEXT NOT NULL, off INTEGER NOT NULL, fetched_at INTEGER NOT NULL, body BLOB NOT NULL, PRIMARY KEY(account, req, off)) WITHOUT ROWID;
         CREATE TABLE IF NOT EXISTS search_history(account TEXT NOT NULL, term TEXT NOT NULL, at INTEGER NOT NULL, PRIMARY KEY(account, term)) WITHOUT ROWID;
         CREATE TABLE IF NOT EXISTS meta(k TEXT PRIMARY KEY, v TEXT NOT NULL) WITHOUT ROWID;
         PRAGMA user_version = 1;",
    )?;
    Ok(c)
}

fn log<T>(r: rusqlite::Result<T>) -> Option<T> {
    r.map_err(|e| eprintln!("presto-core: store: {e}")).ok()
}

impl Store {
    pub fn open(path: &Path) -> Result<Store, rusqlite::Error> {
        let c = match Connection::open(path).and_then(init) {
            Ok(c) => c,
            Err(_) => {
                for ext in ["", "-wal", "-shm"] {
                    let mut p = path.as_os_str().to_owned();
                    p.push(ext);
                    let _ = std::fs::remove_file(p);
                }
                init(Connection::open(path)?)?
            }
        };
        Ok(Store { conn: Arc::new(Mutex::new(c)) })
    }

    pub fn open_in_memory() -> Store {
        let c = init(Connection::open_in_memory().unwrap()).unwrap();
        Store { conn: Arc::new(Mutex::new(c)) }
    }

    fn with<T>(&self, f: impl FnOnce(&Connection) -> rusqlite::Result<T>) -> Option<T> {
        log(f(&self.conn.lock().unwrap_or_else(|e| e.into_inner())))
    }

    pub fn get_page(&self, account: &str, req: &str, off: u32) -> Option<CachedPage> {
        self.with(|c| {
            c.prepare_cached(GET_PAGE)?
                .query_row(params![account, req, off], |r| {
                    Ok(CachedPage { fetched_at_ms: r.get(0)?, body: r.get(1)? })
                })
                .optional()
        })
        .flatten()
    }

    pub fn put_page(&self, account: &str, req: &str, off: u32, fetched_at_ms: i64, body: &[u8]) {
        self.with(|c| {
            c.prepare_cached("INSERT OR REPLACE INTO pages VALUES(?1,?2,?3,?4,?5)")?
                .execute(params![account, req, off, fetched_at_ms, body])
        });
    }

    pub fn drop_pages_from(&self, account: &str, req: &str, off: u32) {
        self.with(|c| {
            c.prepare_cached("DELETE FROM pages WHERE account=?1 AND req=?2 AND off>=?3")?
                .execute(params![account, req, off])
        });
    }

    pub fn page_count(&self) -> u64 {
        self.with(|c| c.query_row("SELECT count(*) FROM pages", [], |r| r.get::<_, i64>(0))).unwrap_or(0) as u64
    }

    pub fn push_search(&self, account: &str, term: &str, at_ms: i64) {
        self.with(|c| {
            c.prepare_cached("INSERT OR REPLACE INTO search_history VALUES(?1,?2,?3)")?
                .execute(params![account, term, at_ms])?;
            c.prepare_cached(
                "DELETE FROM search_history WHERE account=?1 AND term NOT IN (SELECT term FROM search_history WHERE account=?1 ORDER BY at DESC LIMIT 10)",
            )?
            .execute(params![account])
        });
    }

    /// Newest first, max 10.
    pub fn search_history(&self, account: &str) -> Vec<String> {
        self.with(|c| {
            c.prepare_cached("SELECT term FROM search_history WHERE account=?1 ORDER BY at DESC LIMIT 10")?
                .query_map(params![account], |r| r.get(0))?
                .collect()
        })
        .unwrap_or_default()
    }

    pub fn clear_search(&self, account: &str) {
        self.with(|c| c.execute("DELETE FROM search_history WHERE account=?1", params![account]));
    }

    pub fn get_meta(&self, k: &str) -> Option<String> {
        self.with(|c| c.query_row("SELECT v FROM meta WHERE k=?1", params![k], |r| r.get(0)).optional()).flatten()
    }

    pub fn set_meta(&self, k: &str, v: &str) {
        self.with(|c| c.execute("INSERT OR REPLACE INTO meta VALUES(?1,?2)", params![k, v]));
    }

    pub fn wipe(&self) {
        self.with(|c| c.execute_batch("DELETE FROM pages; DELETE FROM search_history; DELETE FROM meta;"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_drops_paging() {
        let mut r = ApiRequest::get("/v1/me/library/songs");
        assert_eq!(req_key(&r), "/v1/me/library/songs");
        for (k, v) in [("limit", "100"), ("offset", "200"), ("sort", "name")] {
            r.query.insert(k.into(), v.into());
        }
        assert_eq!(req_key(&r), "/v1/me/library/songs?sort=name");
    }

    #[test]
    fn page_roundtrip_and_replace() {
        let s = Store::open_in_memory();
        s.put_page("a", "r", 0, 5, b"one");
        let p = s.get_page("a", "r", 0).unwrap();
        assert_eq!((p.fetched_at_ms, p.body.as_slice()), (5, &b"one"[..]));
        assert!(s.get_page("b", "r", 0).is_none());
        assert!(s.get_page("a", "r", 100).is_none());
        s.put_page("a", "r", 0, 6, b"two");
        assert_eq!(s.get_page("a", "r", 0).unwrap().body, b"two");
        assert_eq!(s.page_count(), 1);
    }

    #[test]
    fn drop_from_offset() {
        let s = Store::open_in_memory();
        for o in [0, 100, 200] {
            s.put_page("a", "r", o, 1, b"x");
        }
        s.drop_pages_from("a", "r", 100);
        assert!(s.get_page("a", "r", 0).is_some());
        assert!(s.get_page("a", "r", 100).is_none());
        assert!(s.get_page("a", "r", 200).is_none());
    }

    #[test]
    fn get_page_uses_primary_key() {
        let s = Store::open_in_memory();
        let c = s.conn.lock().unwrap();
        let plan: String = c
            .prepare(&format!("EXPLAIN QUERY PLAN {GET_PAGE}"))
            .unwrap()
            .query_map(params!["a", "r", 0], |r| r.get::<_, String>(3))
            .unwrap()
            .map(|r| r.unwrap())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(plan.contains("PRIMARY KEY"), "{plan}");
    }

    #[test]
    fn search_history_cap_order_clear() {
        let s = Store::open_in_memory();
        for i in 0..12 {
            s.push_search("a", &format!("t{i}"), i);
        }
        s.push_search("b", "other", 1);
        let h = s.search_history("a");
        assert_eq!(h.len(), 10);
        assert_eq!(h[0], "t11");
        assert_eq!(h[9], "t2");
        s.push_search("a", "t5", 100);
        assert_eq!(s.search_history("a")[0], "t5");
        assert_eq!(s.search_history("a").len(), 10);
        s.clear_search("a");
        assert!(s.search_history("a").is_empty());
        assert_eq!(s.search_history("b"), ["other"]);
    }

    #[test]
    fn meta_and_wipe() {
        let s = Store::open_in_memory();
        s.set_meta("k", "v");
        assert_eq!(s.get_meta("k").as_deref(), Some("v"));
        s.put_page("a", "r", 0, 1, b"x");
        s.push_search("a", "t", 1);
        s.wipe();
        assert_eq!(s.page_count(), 0);
        assert!(s.search_history("a").is_empty());
        assert!(s.get_meta("k").is_none());
    }

    fn version(s: &Store) -> i64 {
        s.conn.lock().unwrap().query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap()
    }

    #[test]
    fn future_version_recreated() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("db");
        {
            let s = Store::open(&p).unwrap();
            s.put_page("a", "r", 0, 1, b"x");
            s.conn.lock().unwrap().execute_batch("PRAGMA user_version = 99;").unwrap();
        }
        let s = Store::open(&p).unwrap();
        assert_eq!(version(&s), 1);
        assert_eq!(s.page_count(), 0);
    }

    #[test]
    fn garbage_file_recreated() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("db");
        std::fs::write(&p, vec![0xABu8; 4096]).unwrap();
        let s = Store::open(&p).unwrap();
        assert_eq!(version(&s), 1);
        assert_eq!(s.page_count(), 0);
    }

    #[test]
    fn file_persists() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("db");
        Store::open(&p).unwrap().put_page("a", "r", 0, 1, b"x");
        assert!(Store::open(&p).unwrap().get_page("a", "r", 0).is_some());
    }
}

//! Artwork disk LRU. Fetches from Apple's artwork CDN only (never api.music.apple.com).
//! No decoding here; callers poll `get` and decode the file.
use crate::paths::ensure_private_dir;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

pub const ART_CAP: u64 = 500 * 1024 * 1024;
pub const ART_SIZES: [u32; 3] = [160, 320, 640];
pub const FAIL_RETRY: Duration = Duration::from_secs(60);

#[derive(Clone, Debug, PartialEq)]
pub enum ArtState {
    Ready(PathBuf),
    Pending,
    Failed,
}

/// Fill `{w}`/`{h}` with the smallest cached size >= `px` (640 max).
pub fn expand(template: &str, px: u32) -> String {
    let s = ART_SIZES.iter().copied().find(|&s| s >= px).unwrap_or(640).to_string();
    template.replace("{w}", &s).replace("{h}", &s)
}

pub fn file_for(dir: &Path, url: &str) -> PathBuf {
    let hex: String = Sha256::digest(url.as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
    dir.join(&hex[..2]).join(hex)
}

pub struct ArtCache {
    dir: PathBuf,
    cap: u64,
    total: Mutex<u64>,
    http: reqwest::Client,
    inflight: Mutex<HashSet<String>>,
    failed: Mutex<HashMap<String, Instant>>,
}

/// All files in the two-level tree as (mtime, len, path).
fn scan(dir: &Path) -> Vec<(SystemTime, u64, PathBuf)> {
    let mut out = Vec::new();
    let Ok(shards) = fs::read_dir(dir) else { return out };
    for shard in shards.flatten() {
        let Ok(files) = fs::read_dir(shard.path()) else { continue };
        for f in files.flatten() {
            if let Ok(md) = f.metadata() {
                if md.is_file() {
                    out.push((md.modified().unwrap_or(SystemTime::UNIX_EPOCH), md.len(), f.path()));
                }
            }
        }
    }
    out
}

impl ArtCache {
    pub fn open(dir: PathBuf, cap: u64) -> io::Result<Arc<ArtCache>> {
        ensure_private_dir(&dir)?;
        let mut total = 0;
        for (_, len, p) in scan(&dir) {
            if p.extension().is_some_and(|e| e == "part") {
                let _ = fs::remove_file(&p);
            } else {
                total += len;
            }
        }
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(io::Error::other)?;
        Ok(Arc::new(ArtCache {
            dir,
            cap,
            total: Mutex::new(total),
            http,
            inflight: Mutex::new(HashSet::new()),
            failed: Mutex::new(HashMap::new()),
        }))
    }

    pub fn total_bytes(&self) -> u64 {
        *self.total.lock().unwrap()
    }

    /// Non-blocking; must run inside a tokio runtime.
    pub fn get(self: &Arc<Self>, url: &str) -> ArtState {
        let p = file_for(&self.dir, url);
        if p.exists() {
            if let Ok(f) = fs::File::options().write(true).open(&p) {
                let _ = f.set_modified(SystemTime::now());
            }
            return ArtState::Ready(p);
        }
        if let Some(t) = self.failed.lock().unwrap().get(url) {
            if t.elapsed() < FAIL_RETRY {
                return ArtState::Failed;
            }
        }
        if !self.inflight.lock().unwrap().insert(url.to_string()) {
            return ArtState::Pending;
        }
        let me = self.clone();
        let url = url.to_string();
        tokio::spawn(async move {
            let r = me.fetch(&url).await;
            me.inflight.lock().unwrap().remove(&url);
            match r {
                Ok(_) => {
                    me.failed.lock().unwrap().remove(&url);
                }
                Err(_) => {
                    me.failed.lock().unwrap().insert(url, Instant::now());
                }
            }
        });
        ArtState::Pending
    }

    pub async fn fetch(&self, url: &str) -> io::Result<PathBuf> {
        let bytes = self
            .http
            .get(url)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(io::Error::other)?
            .bytes()
            .await
            .map_err(io::Error::other)?;
        let fin = file_for(&self.dir, url);
        let (f2, len) = (fin.clone(), bytes.len() as u64);
        tokio::task::spawn_blocking(move || -> io::Result<()> {
            fs::create_dir_all(f2.parent().unwrap())?;
            let part = f2.with_extension("part");
            fs::write(&part, &bytes)?;
            fs::rename(&part, &f2)
        })
        .await
        .map_err(io::Error::other)??;
        let over = {
            let mut t = self.total.lock().unwrap();
            *t += len;
            *t > self.cap
        };
        if over {
            let (dir, cap) = (self.dir.clone(), self.cap);
            let total = tokio::task::spawn_blocking(move || evict(&dir, cap)).await.map_err(io::Error::other)?;
            *self.total.lock().unwrap() = total;
        }
        Ok(fin)
    }

    pub fn clear(&self) -> io::Result<()> {
        fs::remove_dir_all(&self.dir)?;
        ensure_private_dir(&self.dir)?;
        *self.total.lock().unwrap() = 0;
        self.failed.lock().unwrap().clear();
        Ok(())
    }
}

/// Delete oldest-mtime files until under 90% of cap; returns the new total.
// ponytail: full directory scan per eviction; keep an in-memory index if eviction shows up in profiles.
fn evict(dir: &Path, cap: u64) -> u64 {
    let mut files = scan(dir);
    files.sort_by_key(|f| f.0);
    let mut total: u64 = files.iter().map(|f| f.1).sum();
    for (_, len, p) in files {
        if total <= cap / 10 * 9 {
            break;
        }
        if fs::remove_file(&p).is_ok() {
            total -= len;
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    /// Serves `n` bytes (or 404 when n == 0) to every connection; counts hits.
    async fn stub(n: usize) -> (String, Arc<AtomicUsize>, tokio::task::JoinHandle<()>) {
        let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", l.local_addr().unwrap());
        let hits = Arc::new(AtomicUsize::new(0));
        let h = hits.clone();
        let jh = tokio::spawn(async move {
            loop {
                let Ok((mut s, _)) = l.accept().await else { return };
                h.fetch_add(1, Ordering::SeqCst);
                tokio::spawn(async move {
                    let mut buf = [0u8; 2048];
                    let _ = s.read(&mut buf).await;
                    let head = if n == 0 {
                        "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string()
                    } else {
                        format!("HTTP/1.1 200 OK\r\nContent-Length: {n}\r\nConnection: close\r\n\r\n")
                    };
                    let _ = s.write_all(head.as_bytes()).await;
                    let _ = s.write_all(&vec![7u8; n]).await;
                    let _ = s.shutdown().await;
                });
            }
        });
        (base, hits, jh)
    }

    async fn ready(c: &Arc<ArtCache>, url: &str) -> ArtState {
        for _ in 0..40 {
            let s = c.get(url);
            if s != ArtState::Pending {
                return s;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        ArtState::Pending
    }

    #[test]
    fn expand_snaps_up() {
        let t = "https://x/a/{w}x{h}bb.jpg";
        assert_eq!(expand(t, 200), "https://x/a/320x320bb.jpg");
        assert_eq!(expand(t, 1000), "https://x/a/640x640bb.jpg");
        assert_eq!(expand(t, 100), "https://x/a/160x160bb.jpg");
    }

    #[test]
    fn file_for_is_stable_sharded() {
        let d = Path::new("/c");
        let p = file_for(d, "u");
        assert_eq!(p, file_for(d, "u"));
        let name = p.file_name().unwrap().to_str().unwrap();
        assert_eq!(name.len(), 64);
        assert_eq!(p.parent().unwrap().file_name().unwrap().to_str().unwrap(), &name[..2]);
    }

    #[tokio::test]
    async fn fetch_writes_bytes_no_part() {
        let (base, _, _j) = stub(100).await;
        let t = tempfile::tempdir().unwrap();
        let c = ArtCache::open(t.path().join("art"), ART_CAP).unwrap();
        let url = format!("{base}/a.jpg");
        let p = c.fetch(&url).await.unwrap();
        assert_eq!(p, file_for(&t.path().join("art"), &url));
        assert_eq!(fs::read(&p).unwrap(), vec![7u8; 100]);
        assert!(scan(&t.path().join("art")).iter().all(|f| f.2.extension().is_none_or(|e| e != "part")));
        assert_eq!(c.total_bytes(), 100);
    }

    #[tokio::test]
    async fn get_dedupes_and_hits_offline() {
        let (base, hits, j) = stub(100).await;
        let t = tempfile::tempdir().unwrap();
        let c = ArtCache::open(t.path().join("art"), ART_CAP).unwrap();
        let url = format!("{base}/a.jpg");
        assert_eq!(c.get(&url), ArtState::Pending);
        assert_eq!(c.get(&url), ArtState::Pending);
        let ArtState::Ready(p) = ready(&c, &url).await else { panic!("not ready") };
        assert_eq!(hits.load(Ordering::SeqCst), 1);
        j.abort();
        let _ = j.await;
        assert_eq!(c.get(&url), ArtState::Ready(p));
    }

    #[tokio::test]
    async fn http_404_fails() {
        let (base, _, _j) = stub(0).await;
        let t = tempfile::tempdir().unwrap();
        let c = ArtCache::open(t.path().join("art"), ART_CAP).unwrap();
        assert_eq!(ready(&c, &format!("{base}/x.jpg")).await, ArtState::Failed);
    }

    fn age(c: &Arc<ArtCache>, url: &str, secs: u64) {
        let f = fs::File::options().write(true).open(file_for(&c.dir, url)).unwrap();
        f.set_modified(SystemTime::now() - Duration::from_secs(secs)).unwrap();
    }

    #[tokio::test]
    async fn lru_evicts_oldest() {
        let (base, _, _j) = stub(100).await;
        let t = tempfile::tempdir().unwrap();
        let c = ArtCache::open(t.path().join("art"), 250).unwrap();
        let (a, b, d) = (format!("{base}/a"), format!("{base}/b"), format!("{base}/c"));
        c.fetch(&a).await.unwrap();
        age(&c, &a, 30);
        c.fetch(&b).await.unwrap();
        age(&c, &b, 20);
        c.fetch(&d).await.unwrap();
        assert!(!file_for(&c.dir, &a).exists());
        assert!(file_for(&c.dir, &b).exists());
        assert!(c.total_bytes() <= 250);
    }

    #[tokio::test]
    async fn get_touches_so_other_is_evicted() {
        let (base, _, _j) = stub(100).await;
        let t = tempfile::tempdir().unwrap();
        let c = ArtCache::open(t.path().join("art"), 250).unwrap();
        let (a, b, d) = (format!("{base}/a"), format!("{base}/b"), format!("{base}/c"));
        c.fetch(&a).await.unwrap();
        age(&c, &a, 30);
        c.fetch(&b).await.unwrap();
        age(&c, &b, 20);
        assert!(matches!(c.get(&a), ArtState::Ready(_)));
        c.fetch(&d).await.unwrap();
        assert!(file_for(&c.dir, &a).exists());
        assert!(!file_for(&c.dir, &b).exists());
    }

    #[tokio::test]
    async fn open_cleans_part_and_sums() {
        let t = tempfile::tempdir().unwrap();
        let dir = t.path().join("art");
        fs::create_dir_all(dir.join("ab")).unwrap();
        fs::write(dir.join("ab").join("x.part"), b"junk").unwrap();
        fs::write(dir.join("ab").join("keep"), [0u8; 10]).unwrap();
        let c = ArtCache::open(dir.clone(), ART_CAP).unwrap();
        assert!(!dir.join("ab").join("x.part").exists());
        assert_eq!(c.total_bytes(), 10);
    }

    #[tokio::test]
    async fn clear_empties() {
        let (base, _, _j) = stub(100).await;
        let t = tempfile::tempdir().unwrap();
        let c = ArtCache::open(t.path().join("art"), ART_CAP).unwrap();
        c.fetch(&format!("{base}/a")).await.unwrap();
        c.clear().unwrap();
        assert_eq!(c.total_bytes(), 0);
        assert!(scan(&c.dir).is_empty());
    }
}

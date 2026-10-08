#![allow(dead_code)]
use presto_core::paths::Paths;
use presto_core::{Core, CoreConfig, CoreHandle, CoreState, Launch, Timings};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use tempfile::TempDir;
use tokio::sync::watch;

pub fn mock_bin() -> PathBuf {
    let p = std::env::current_exe().unwrap().parent().unwrap().parent().unwrap().join("presto-engine-mock");
    assert!(p.exists(), "run cargo build -p presto-engine-mock first");
    p
}

pub fn fast_timings() -> Timings {
    let ms = Duration::from_millis;
    Timings {
        heartbeat: ms(100),
        misses: 3,
        connect: Duration::from_secs(5),
        drift: Duration::from_secs(1),
        backoff_base: ms(20),
        backoff_cap: ms(160),
        fast: Duration::from_secs(60),
        stable: Duration::from_secs(120),
        kill_grace: ms(200),
        quit_wait: Duration::from_secs(1),
    }
}

pub struct Rig {
    pub core: CoreHandle,
    pub calls: Arc<AtomicU32>,
    pub dir: TempDir,
}

impl Rig {
    pub fn calls(&self) -> u32 {
        self.calls.load(Ordering::SeqCst)
    }
}

pub async fn rig(argv: impl Fn(u32) -> Vec<String> + Send + Sync + 'static) -> Rig {
    let dir = tempfile::Builder::new().prefix("pc").tempdir_in("/tmp").unwrap();
    let calls = Arc::new(AtomicU32::new(0));
    let c = calls.clone();
    let cfg = CoreConfig {
        launcher: Arc::new(move |n| {
            c.fetch_add(1, Ordering::SeqCst);
            Launch { program: mock_bin(), args: argv(n).into_iter().map(Into::into).collect() }
        }),
        socket: dir.path().join("e.sock"),
        paths: Paths::under(dir.path().join("state")),
        timings: fast_timings(),
    };
    let core = Core::start(cfg).await.unwrap();
    Rig { core, calls, dir }
}

pub async fn wait_for(
    rx: &mut watch::Receiver<CoreState>,
    within: Duration,
    pred: impl Fn(&CoreState) -> bool,
) -> CoreState {
    let hit = match tokio::time::timeout(within, rx.wait_for(|s| pred(s))).await {
        Ok(Ok(s)) => Some(s.clone()),
        _ => None,
    };
    if let Some(s) = hit {
        return s;
    }
    panic!("timed out; last state: {:?}", *rx.borrow())
}

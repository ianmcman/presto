//! One engine process plus its IPC connection (seed of the Phase 3 supervisor).

use presto_ipc::transport::{self, Conn};
use presto_ipc::{
    ApiRequest, AuthState, Command, Event, Frame, HEARTBEAT_INTERVAL, HEARTBEAT_MISSES, Hello,
    Kind, Outcome, PROTO, PlayState, Role,
};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub type R<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub struct EngineOpts {
    pub bin: PathBuf,
    pub app_path: Option<PathBuf>,
    pub socket: PathBuf,
    pub profile: PathBuf,
    pub extra: Vec<String>,
    pub log_dir: PathBuf,
    pub label: String,
}

pub struct Session {
    conn: Conn,
    pub child: tokio::process::Child,
    pub engine_hello: Hello,
    events: std::fs::File,
    next_id: u64,
    ping_seq: u64,
    unanswered: u32,
    pub counts: BTreeMap<&'static str, u32>,
    pub states: Vec<PlayState>,
    pub last_auth: Option<AuthState>,
    /// (position_ms, duration_ms)
    pub last_progress: Option<(u64, u64)>,
}

pub fn unix_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl Session {
    pub async fn start(o: &EngineOpts) -> R<Session> {
        let ts = unix_ts();
        let listener = transport::bind(&o.socket)?;
        let log = std::fs::File::create(o.log_dir.join(format!("engine-{}-{ts}.log", o.label)))?;
        let mut cmd = tokio::process::Command::new(&o.bin);
        if let Some(a) = &o.app_path {
            cmd.arg(a);
        }
        cmd.arg("--socket")
            .arg(&o.socket)
            .arg("--profile")
            .arg(&o.profile)
            .args(&o.extra)
            .stdout(log.try_clone()?)
            .stderr(log)
            .kill_on_drop(true);
        let child = cmd.spawn()?;
        let (stream, _) = tokio::time::timeout(Duration::from_secs(30), listener.accept())
            .await
            .map_err(|_| "engine did not connect within 30s")??;
        let mut conn = transport::framed(stream);
        let engine_hello =
            match tokio::time::timeout(Kind::Hello.timeout(), transport::recv(&mut conn))
                .await
                .map_err(|_| "engine hello timed out")??
            {
                Some(Frame::Hello(h)) => h,
                other => return Err(format!("expected engine hello, got {other:?}").into()),
            };
        engine_hello.check(PROTO)?;
        let ours = Hello::new(
            Role::Presto,
            &[],
            Some(format!("presto-spike {}", env!("CARGO_PKG_VERSION"))),
        );
        transport::send(&mut conn, &Frame::Hello(ours)).await?;
        let events =
            std::fs::File::create(o.log_dir.join(format!("events-{}-{ts}.ndjson", o.label)))?;
        Ok(Session {
            conn,
            child,
            engine_hello,
            events,
            next_id: 1,
            ping_seq: 0,
            unanswered: 0,
            counts: BTreeMap::new(),
            states: vec![],
            last_auth: None,
            last_progress: None,
        })
    }

    pub fn pid(&self) -> Option<u32> {
        self.child.id()
    }

    fn record(&mut self, f: &Frame) {
        if let Ok(s) = serde_json::to_string(f) {
            let _ = writeln!(self.events, "{s}");
        }
        let Frame::Evt { evt } = f else { return };
        let name = match evt {
            Event::PlaybackState { state, .. } => {
                self.states.push(*state);
                "playback_state"
            }
            Event::Progress {
                position_ms,
                duration_ms,
                ..
            } => {
                self.last_progress = Some((*position_ms, *duration_ms));
                "progress"
            }
            Event::TrackChanged { .. } => "track_changed",
            Event::QueueChanged { .. } => "queue_changed",
            Event::Volume { .. } => "volume",
            Event::Shuffle { .. } => "shuffle",
            Event::Repeat { .. } => "repeat",
            Event::Auth { state } => {
                self.last_auth = Some(*state);
                "auth"
            }
            Event::Error { .. } => "error",
        };
        *self.counts.entry(name).or_insert(0) += 1;
    }

    /// Pumps frames (answering heartbeats) until `f` matches. Err on timeout,
    /// EOF, or 3 unanswered pings.
    pub async fn wait_for<T>(
        &mut self,
        what: &str,
        within: Duration,
        mut f: impl FnMut(&Frame) -> Option<T>,
    ) -> R<T> {
        let mut tick = tokio::time::interval(HEARTBEAT_INTERVAL);
        let run = async {
            loop {
                tokio::select! {
                    _ = tick.tick() => {
                        if self.unanswered >= HEARTBEAT_MISSES {
                            return Err(format!("engine hung: {HEARTBEAT_MISSES} pings unanswered").into());
                        }
                        self.ping_seq += 1;
                        transport::send(&mut self.conn, &Frame::Ping { seq: self.ping_seq }).await?;
                        self.unanswered += 1;
                    }
                    r = transport::recv(&mut self.conn) => {
                        let Some(frame) = r? else {
                            return Err("engine closed the connection".into());
                        };
                        if let Frame::Pong { seq } = frame {
                            if seq == self.ping_seq {
                                self.unanswered = 0;
                            }
                            continue;
                        }
                        self.record(&frame);
                        if let Some(t) = f(&frame) {
                            return Ok(t);
                        }
                    }
                }
            }
        };
        match tokio::time::timeout(within, run).await {
            Ok(r) => r,
            Err(_) => Err(format!("timed out after {within:?} waiting for {what}").into()),
        }
    }

    async fn roundtrip(&mut self, frame: Frame, id: u64) -> R<Outcome> {
        let within = frame.kind().map(Kind::timeout).unwrap_or_default();
        transport::send(&mut self.conn, &frame).await?;
        self.wait_for("response", within, |f| match f {
            Frame::Res { id: rid, outcome } if *rid == id => Some(outcome.clone()),
            _ => None,
        })
        .await
    }

    pub async fn call(&mut self, cmd: Command) -> R<Outcome> {
        let id = self.next_id;
        self.next_id += 1;
        self.roundtrip(Frame::Cmd { id, cmd }, id).await
    }

    pub async fn get(&mut self, path: &str, query: &[(&str, &str)]) -> R<Outcome> {
        let id = self.next_id;
        self.next_id += 1;
        let mut req = ApiRequest::get(path);
        for (k, v) in query {
            req.query.insert((*k).into(), (*v).into());
        }
        self.roundtrip(Frame::Req { id, req }, id).await
    }

    /// Closes the connection (engine quits on EOF so Chromium flushes cookies),
    /// then waits 10 s before killing.
    pub async fn shutdown(self) -> R<std::process::ExitStatus> {
        let Session {
            conn, mut child, ..
        } = self;
        drop(conn);
        match tokio::time::timeout(Duration::from_secs(10), child.wait()).await {
            Ok(r) => Ok(r?),
            Err(_) => {
                child.kill().await?;
                Ok(child.wait().await?)
            }
        }
    }
}

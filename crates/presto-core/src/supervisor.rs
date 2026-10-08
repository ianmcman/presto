//! The supervisor actor: owns the engine child, its socket and the pending-request map.
use crate::auth::{AuthEffect, AuthMachine};
use crate::backoff::Backoff;
use crate::config::{CoreConfig, check_bridge};
use crate::mirror::{Snapshot, queue_key, snapshot};
use crate::paths::{Pidfile, new_log_file, sweep_stale, tail};
use crate::state::{BridgeInfo, CoreState, EngineStatus};
use nix::errno::Errno;
use nix::sys::signal::{Signal, killpg};
use nix::unistd::Pid;
use presto_ipc::transport::{self, Conn, TransportError};
use presto_ipc::{
    ApiRequest, PlayState, AuthState, Command, ErrorKind, Event, FaultSpec, Frame, Hello, IpcError, Kind, Outcome, PROTO,
    Role, caps,
};
use std::collections::{HashMap, VecDeque};
use std::io;
use std::process::Stdio;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, watch};
use tokio::time::{Instant, MissedTickBehavior, interval, sleep, sleep_until, timeout};

const SEEK_TOLERANCE_MS: u64 = 2000;
const SEEK_SETTLE: Duration = Duration::from_millis(1500);
const SEEK_TRIES: u32 = 3;
const STATE_SETTLE: Duration = Duration::from_millis(500);
const STATE_TRIES: u32 = 5;
const VERIFY_HOLD: Duration = Duration::from_millis(2000);
const VERIFY_DEADLINE: Duration = Duration::from_secs(20);

type Tx = oneshot::Sender<Outcome>;

pub(crate) enum Job {
    Cmd(Command),
    Req(ApiRequest),
    Mock(FaultSpec),
}

impl Job {
    fn frame(self, id: u64) -> Frame {
        match self {
            Job::Cmd(cmd) => Frame::Cmd { id, cmd },
            Job::Req(req) => Frame::Req { id, req },
            Job::Mock(fault) => Frame::Mock { id, fault },
        }
    }

    /// Sent whenever connected, never queued.
    fn immediate(&self) -> bool {
        matches!(self, Job::Mock(_) | Job::Cmd(Command::ShowWindow { .. }))
    }
}

pub(crate) enum Msg {
    Job(Job, Tx),
    Restart,
    Shutdown(oneshot::Sender<()>),
}

fn auth_err() -> Outcome {
    Outcome::Err { error: IpcError::new(ErrorKind::AuthExpired, "signed out") }
}

/// Queue and player to put back after a restart or re-auth (D-01, D-04, D-11).
struct Restore {
    snap: Snapshot,
    /// The page lost its queue, so load it again.
    full: bool,
    resume_allowed: bool,
    verify: Option<Verify>,
    /// Volume to put back when a load restore ends; the engine is muted while it converges.
    unmute: Option<f32>,
}

/// Post-load check that the seek landed and the play state is right (RESEARCH Pitfall 6).
struct Verify {
    target: u64,
    resume: bool,
    started: Instant,
    seek_tries: u32,
    seek_since: Option<Instant>,
    state_tries: u32,
    state_sent: Option<Instant>,
    ok_since: Option<Instant>,
}

enum VerifyStep {
    Done,
    Wait,
    Retry(VecDeque<Command>),
}

/// One engine connection's bookkeeping.
struct Sess {
    conn: Conn,
    pending: HashMap<u64, (Tx, Instant)>,
    queued: Vec<(Job, Tx)>,
    next_id: u64,
}

impl Sess {
    async fn send(&mut self, job: Job, tx: Tx) -> Result<u64, TransportError> {
        let id = self.next_id;
        dispatch(&mut self.conn, &mut self.pending, &mut self.next_id, job, tx).await?;
        Ok(id)
    }
}

fn unavail(msg: impl Into<String>) -> Outcome {
    Outcome::Err { error: IpcError::new(ErrorKind::Unavailable, msg) }
}

pub struct Core;

impl Core {
    pub async fn start(cfg: CoreConfig) -> io::Result<CoreHandle> {
        cfg.paths.prepare()?;
        let (pidfile, grace) = (cfg.paths.pidfile.clone(), cfg.timings.kill_grace);
        tokio::task::spawn_blocking(move || sweep_stale(&pidfile, grace)).await.map_err(io::Error::other)??;
        let (tx, rx) = mpsc::unbounded_channel();
        let (state_tx, state_rx) = watch::channel(CoreState::default());
        let t = &cfg.timings;
        let backoff = Backoff::new(t.backoff_base, t.backoff_cap, t.fast, t.stable);
        tokio::spawn(
            Actor {
                cfg,
                rx,
                state_tx,
                backoff,
                restore: None,
                same_queue_crashes: 0,
                last_crash_key: None,
                attempt_start: std::time::Instant::now(),
                auth: AuthMachine::default(),
                bridge_seen: false,
                auth_seen: false,
                announced: false,
                drift: false,
                steps: None,
                inflight: None,
            }
            .run(),
        );
        Ok(CoreHandle { tx, state: state_rx })
    }
}

#[derive(Clone)]
pub struct CoreHandle {
    tx: mpsc::UnboundedSender<Msg>,
    state: watch::Receiver<CoreState>,
}

impl CoreHandle {
    pub fn state(&self) -> watch::Receiver<CoreState> {
        self.state.clone()
    }

    async fn ask(&self, job: Job) -> Outcome {
        let (tx, rx) = oneshot::channel();
        if self.tx.send(Msg::Job(job, tx)).is_err() {
            return unavail("core stopped");
        }
        rx.await.unwrap_or_else(|_| unavail("core stopped"))
    }

    pub async fn command(&self, cmd: Command) -> Outcome {
        self.ask(Job::Cmd(cmd)).await
    }

    pub async fn request(&self, req: ApiRequest) -> Outcome {
        self.ask(Job::Req(req)).await
    }

    pub async fn show_sign_in(&self) -> Outcome {
        self.command(Command::ShowWindow { show: true }).await
    }

    /// Test/demo control.
    pub async fn mock(&self, fault: FaultSpec) -> Outcome {
        self.ask(Job::Mock(fault)).await
    }

    pub fn restart_engine(&self) {
        let _ = self.tx.send(Msg::Restart);
    }

    pub async fn shutdown(self) {
        let (tx, rx) = oneshot::channel();
        if self.tx.send(Msg::Shutdown(tx)).is_ok() {
            let _ = rx.await;
        }
    }
}

/// SIGKILLs the engine's process group if the actor is dropped mid-session.
struct Guard(Option<Pid>);

impl Drop for Guard {
    fn drop(&mut self) {
        if let Some(p) = self.0 {
            let _ = killpg(p, Signal::SIGKILL);
        }
    }
}

enum End {
    Failure(String),
    Restart,
    Shutdown(Option<oneshot::Sender<()>>),
}

enum Wake {
    Elapsed,
    Restart,
    Shutdown(Option<oneshot::Sender<()>>),
}


enum Ev {
    Tick,
    Frame(Result<Option<Frame>, TransportError>),
    Exit(io::Result<std::process::ExitStatus>),
    Msg(Option<Msg>),
    Drift,
}

struct Actor {
    cfg: CoreConfig,
    rx: mpsc::UnboundedReceiver<Msg>,
    state_tx: watch::Sender<CoreState>,
    backoff: Backoff,
    // Survive restarts.
    restore: Option<Restore>,
    same_queue_crashes: u32,
    last_crash_key: Option<u64>,
    // Reset per engine process.
    attempt_start: std::time::Instant,
    auth: AuthMachine,
    bridge_seen: bool,
    auth_seen: bool,
    announced: bool,
    drift: bool,
    steps: Option<VecDeque<Command>>,
    /// (request id, is the SetQueue load) of the restore step in flight.
    inflight: Option<(u64, Command)>,
}

impl Actor {
    fn set(&self, f: impl FnOnce(&mut CoreState)) {
        self.state_tx.send_modify(f);
    }

    fn status(&self, s: EngineStatus) {
        self.set(|c| c.engine = s);
    }

    fn log_path(&self) -> std::path::PathBuf {
        self.state_tx.borrow().log_path.clone().unwrap_or_default()
    }

    /// Every bridge_ready means the page reloaded: revisions restart and MusicKit's queue is gone.
    fn on_bridge_ready(&mut self) {
        self.set(|c| {
            c.queue.new_generation();
            c.player.new_generation();
        });
        self.auth_seen = false;
        if let Some(r) = &mut self.restore {
            r.full = true;
        }
    }

    fn take_snapshot(&self) -> Option<Snapshot> {
        let st = self.state_tx.borrow();
        snapshot(&st.queue, &st.player)
    }

    /// Auth left SignedIn: remember what to put back once it returns.
    fn auth_lost(&mut self) {
        if self.restore.is_none()
            && let Some(snap) = self.take_snapshot()
        {
            self.restore = Some(Restore { snap, full: false, resume_allowed: true, verify: None, unmute: None });
        }
    }

    fn publish_auth(&self) {
        let a = self.auth.state;
        self.set(|c| c.auth = a);
    }

    /// Mirrors and auth. Returns true when the sign-in window should open.
    fn on_event(&mut self, evt: Event) -> bool {
        match evt {
            Event::QueueChanged { rev, items, index } => {
                self.set(|c| {
                    c.queue.apply(rev, items, index);
                });
            }
            Event::Auth { state } => {
                let was = self.auth.state;
                self.auth_seen = true;
                let effect = self.auth.on_event(state);
                self.publish_auth();
                if was == Some(AuthState::SignedIn) && state != AuthState::SignedIn {
                    self.auth_lost();
                }
                return effect == AuthEffect::ShowWindow;
            }
            Event::Error { error } => eprintln!("presto-core: engine error: {error:?}"),
            Event::BridgeReady { .. } => {}
            other => self.set(|c| c.player.apply(&other)),
        }
        false
    }

    /// An auth_expired reply while signed in expires the session (D-13).
    fn on_outcome(&mut self, o: &Outcome) {
        if let Outcome::Err { error } = o
            && error.kind == ErrorKind::AuthExpired
        {
            let was = self.auth.state;
            self.auth.on_auth_expired_outcome();
            if self.auth.state != was {
                self.publish_auth();
                self.auth_lost();
            }
        }
    }

    fn usable(&self) -> bool {
        !self.drift && self.bridge_seen && self.auth_seen && self.restore.is_none()
    }

    /// A restore step answered (or timed out).
    fn step_result(&mut self, cmd: &Command, o: &Outcome) {
        self.inflight = None;
        let load = matches!(cmd, Command::SetQueue { .. });
        {
            let st = self.state_tx.borrow();
            let res = match o {
                Outcome::Ok { .. } => "ok".to_string(),
                Outcome::Err { error } => format!("err {error:?}"),
            };
            eprintln!("presto-core: restore: {cmd:?} -> {res}, state {:?} at {} ms, volume {}", st.player.state, st.player.position_ms, st.player.volume);
        }
        match o {
            Outcome::Ok { .. } => {
                let ids_match = self.restore.as_ref().is_none_or(|r| self.state_tx.borrow().queue.ids() == r.snap.ids);
                if load && !ids_match {
                    eprintln!("presto-core: restore: queue did not load as expected");
                    self.end_restore();
                }
            }
            Outcome::Err { error } => {
                eprintln!("presto-core: restore step failed: {error:?}");
                if error.kind == ErrorKind::AuthExpired {
                    // try again from the top when sign-in returns
                    self.steps = None;
                    if let Some(r) = &mut self.restore {
                        r.full = true;
                    }
                } else if load {
                    self.end_restore();
                }
                // other steps: verify retries the seek and the play state (03-13)
            }
        }
    }

    /// Ends a restore. A load restore puts the snapshot volume back; one that must end paused (D-01, D-04) sends Pause first and stays muted while the player is Playing or Loading.
    fn end_restore(&mut self) {
        let state = self.state_tx.borrow().player.state;
        let Some(r) = self.restore.as_mut() else { return };
        r.verify = None;
        let resume = r.snap.was_playing && r.resume_allowed;
        let Some(volume) = r.unmute.take() else {
            self.restore = None;
            self.steps = None;
            return;
        };
        let busy = matches!(state, PlayState::Playing | PlayState::Loading);
        let mut v = VecDeque::new();
        if !resume {
            v.push_back(Command::Pause);
        }
        if resume || !busy {
            v.push_back(Command::SetVolume { volume });
        } else {
            // ponytail: stays muted rather than risk audio on a paused restore; the next SetVolume from the user unmutes
            eprintln!("presto-core: restore: left muted, state {state:?}");
        }
        self.steps = Some(v);
    }

    fn begin_restore(&mut self) {
        let Some(r) = &self.restore else { return };
        let s = &r.snap;
        let playing = self.state_tx.borrow().player.state == PlayState::Playing;
        let load = r.full || self.state_tx.borrow().queue.ids() != s.ids;
        let resume = s.was_playing && r.resume_allowed;
        let mut v = VecDeque::new();
        if load {
            // MusicKit can play at 0 ms right after load, so stay muted until the restore is confirmed.
            v.push_back(Command::SetVolume { volume: 0.0 });
            v.push_back(Command::SetQueue { ids: s.ids.clone(), start: s.index, play: false });
            v.push_back(Command::SetVolume { volume: 0.0 });
            if s.position_ms > SEEK_TOLERANCE_MS {
                v.push_back(Command::Seek { ms: s.position_ms });
            }
            v.push_back(Command::SetShuffle { on: s.shuffle });
            v.push_back(Command::SetRepeat { mode: s.repeat });
            // MusicKit may autoplay after a seek on a fresh load, so always end explicitly (D-01, D-04).
            v.push_back(if resume { Command::Play } else { Command::Pause });
        } else if resume && !playing {
            v.push_back(Command::Play);
        } else if !resume && playing {
            v.push_back(Command::Pause);
        }
        let verify = load.then_some(Verify {
            target: s.position_ms,
            resume,
            started: Instant::now(),
            seek_tries: 0,
            seek_since: None,
            state_tries: 0,
            state_sent: None,
            ok_since: None,
        });
        if load {
            // a stale pre-crash position must not pass verification
            self.set(|c| c.player.position_ms = 0);
        }
        if let Some(r) = &mut self.restore {
            r.verify = verify;
            r.unmute = load.then_some(r.snap.volume);
        }
        self.steps = Some(v);
    }

    fn verify_step(&mut self) -> VerifyStep {
        let Some(v) = self.restore.as_mut().and_then(|r| r.verify.as_mut()) else { return VerifyStep::Done };
        let (pos, state) = {
            let st = self.state_tx.borrow();
            (st.player.position_ms, st.player.state)
        };
        let now = Instant::now();
        if v.started.elapsed() >= VERIFY_DEADLINE {
            eprintln!("presto-core: restore: not confirmed within 20 s (at {pos} ms, want {} ms, state {state:?})", v.target);
            return VerifyStep::Done;
        }
        // MusicKit drops Play/Pause/Seek-sensitive work while loading
        if state == PlayState::Loading {
            v.ok_since = None;
            v.seek_since = None;
            return VerifyStep::Wait;
        }
        // only a lost seek lands short; a playing restore runs ahead legitimately
        if pos + SEEK_TOLERANCE_MS < v.target {
            v.ok_since = None;
            let since = *v.seek_since.get_or_insert(now);
            if since.elapsed() < SEEK_SETTLE {
                return VerifyStep::Wait;
            }
            if v.seek_tries >= SEEK_TRIES {
                eprintln!("presto-core: restore: seek not confirmed after {SEEK_TRIES} tries (at {pos} ms, want {} ms), checking state only", v.target);
                v.target = 0;
                v.seek_since = None;
            } else {
                v.seek_tries += 1;
                v.seek_since = None;
                return VerifyStep::Retry(VecDeque::from([Command::Seek { ms: v.target }]));
            }
        }
        let want = if v.resume { PlayState::Playing } else { PlayState::Paused };
        if state != want {
            v.ok_since = None;
            if v.state_sent.is_some_and(|t| t.elapsed() < STATE_SETTLE) {
                return VerifyStep::Wait;
            }
            if v.state_tries >= STATE_TRIES {
                eprintln!("presto-core: restore: state not confirmed after 5 tries (state {state:?})");
                return VerifyStep::Done;
            }
            v.state_tries += 1;
            v.state_sent = Some(now);
            let cmd = if v.resume { Command::Play } else { Command::Pause };
            return VerifyStep::Retry(VecDeque::from([cmd]));
        }
        if now.duration_since(*v.ok_since.get_or_insert(now)) >= VERIFY_HOLD {
            VerifyStep::Done
        } else {
            VerifyStep::Wait
        }
    }

    /// Runs after every event: starts and advances the restore, announces Ready, flushes queued work.
    async fn pump(&mut self, s: &mut Sess) -> Result<(), TransportError> {
        if self.drift {
            return Ok(());
        }
        if self.auth.blocked() {
            for (_, tx) in s.queued.drain(..) {
                let _ = tx.send(auth_err());
            }
        }
        if self.restore.is_some() && self.steps.is_none() && self.bridge_seen && self.auth_seen && self.auth.allows_traffic() {
            self.begin_restore();
        }
        while self.inflight.is_none() {
            let Some(steps) = &mut self.steps else { break };
            match steps.pop_front() {
                Some(cmd) => {
                    let (tx, _rx) = oneshot::channel();
                    let id = s.send(Job::Cmd(cmd.clone()), tx).await?;
                    self.inflight = Some((id, cmd));
                }
                None => match self.verify_step() {
                    VerifyStep::Wait => break,
                    VerifyStep::Retry(v) => self.steps = Some(v),
                    VerifyStep::Done => self.end_restore(),
                },
            }
        }
        if self.usable() {
            if !self.announced {
                self.announced = true;
                self.status(EngineStatus::Ready);
            }
            for (job, tx) in std::mem::take(&mut s.queued) {
                s.send(job, tx).await?;
            }
        }
        Ok(())
    }

    /// Before backing off: remember the queue, and count same-queue crashes (D-04).
    fn note_failure(&mut self) {
        if self.attempt_start.elapsed() >= self.cfg.timings.stable {
            self.same_queue_crashes = 0;
            self.last_crash_key = None;
        }
        // a restore still pending holds a better snapshot than a half-restored mirror
        let snap = self.restore.take().map(|r| r.snap).or_else(|| self.take_snapshot());
        let Some(snap) = snap else { return };
        let k = queue_key(&snap.ids);
        self.same_queue_crashes = if Some(k) == self.last_crash_key { self.same_queue_crashes + 1 } else { 1 };
        self.last_crash_key = Some(k);
        let resume_allowed = self.same_queue_crashes < 2;
        self.restore = Some(Restore { snap, full: true, resume_allowed, verify: None, unmute: None });
    }

    async fn run(mut self) {
        let mut n = 0u32;
        loop {
            let end = self.attempt(n).await;
            n += 1;
            let wake = match end {
                End::Shutdown(tx) => return finish(tx),
                End::Restart => {
                    if self.restore.is_none()
                        && let Some(snap) = self.take_snapshot()
                    {
                        self.restore = Some(Restore { snap, full: true, resume_allowed: true, verify: None, unmute: None });
                    }
                    Wake::Restart
                }
                End::Failure(reason) => {
                    eprintln!("presto-core: engine failed: {reason}");
                    self.set(|c| c.restarts += 1);
                    self.note_failure();
                    let log_path = self.log_path();
                    match self.backoff.on_failure(std::time::Instant::now()) {
                        Some(d) => {
                            self.status(EngineStatus::Restarting { attempt: n, delay_ms: d.as_millis() as u64 });
                            self.pause(Some(d), "restarting").await
                        }
                        None => {
                            let log_tail = tail(&log_path, 40);
                            self.status(EngineStatus::Failed { reason, log_path, log_tail });
                            self.pause(None, "failed").await
                        }
                    }
                }
            };
            match wake {
                Wake::Elapsed => {}
                Wake::Restart => self.backoff.reset(),
                Wake::Shutdown(tx) => return finish(tx),
            }
        }
    }

    /// Waits `d` (forever when None) while answering messages.
    async fn pause(&mut self, d: Option<Duration>, why: &str) -> Wake {
        let nap = async {
            match d {
                Some(d) => sleep(d).await,
                None => std::future::pending().await,
            }
        };
        tokio::pin!(nap);
        loop {
            tokio::select! {
                _ = &mut nap => return Wake::Elapsed,
                m = self.rx.recv() => match m {
                    None => return Wake::Shutdown(None),
                    Some(Msg::Job(_, tx)) => { let _ = tx.send(unavail(format!("engine {why}"))); }
                    Some(Msg::Restart) => return Wake::Restart,
                    Some(Msg::Shutdown(tx)) => return Wake::Shutdown(Some(tx)),
                },
            }
        }
    }

    /// One engine lifetime: spawn, handshake, session, teardown.
    async fn attempt(&mut self, n: u32) -> End {
        self.status(EngineStatus::Starting);
        self.set(|c| {
            c.bridge = None;
            c.auth = None;
        });
        self.attempt_start = std::time::Instant::now();
        self.auth = AuthMachine::default();
        (self.bridge_seen, self.auth_seen, self.announced, self.drift) = (false, false, false, false);
        (self.steps, self.inflight) = (None, None);
        let socket = self.cfg.socket.clone();
        let listener = match transport::bind(&socket) {
            Ok(l) => l,
            Err(e) => return End::Failure(format!("cannot bind {}: {e}", socket.display())),
        };
        let (log_path, log) = match new_log_file(&self.cfg.paths.logs, n) {
            Ok(x) => x,
            Err(e) => return End::Failure(format!("cannot create engine log: {e}")),
        };
        self.set(|c| c.log_path = Some(log_path.clone()));
        let l = (self.cfg.launcher)(n);
        let log2 = match log.try_clone() {
            Ok(f) => f,
            Err(e) => return End::Failure(format!("cannot clone log handle: {e}")),
        };
        let mut child = match tokio::process::Command::new(&l.program)
            .args(&l.args)
            .arg("--socket")
            .arg(&socket)
            .arg("--profile")
            .arg(&self.cfg.paths.profile)
            .process_group(0)
            .stdin(Stdio::null())
            .stdout(log)
            .stderr(log2)
            .kill_on_drop(true)
            .spawn()
        {
            Ok(c) => c,
            Err(e) => return End::Failure(format!("cannot spawn {}: {e}", l.program.display())),
        };
        let Some(pid) = child.id().map(|p| p as i32) else {
            return End::Failure("engine exited before it could be tracked".into());
        };
        let pgid = Pid::from_raw(pid);
        let mut guard = Guard(Some(pgid));
        match Pidfile::for_pid(pid).and_then(|p| p.write(&self.cfg.paths.pidfile)) {
            Ok(()) => {}
            Err(e) => eprintln!("presto-core: cannot write pidfile: {e}"),
        }
        self.backoff.on_start(std::time::Instant::now());
        eprintln!("presto-core: spawned engine pid {pid} (attempt {n}), log {}", log_path.display());

        let t = self.cfg.timings.clone();
        // ponytail: messages sent during the handshake wait in the channel; Shutdown waits up to `connect`.
        let hs = tokio::select! {
            r = handshake(&listener, &t.connect) => r,
            s = child.wait() => Err(format!("engine exited before connecting: {s:?}")),
        };
        let (conn, hello) = match hs {
            Ok(x) => x,
            Err(reason) => {
                self.teardown(None, &mut child, pgid, None).await;
                guard.0 = None;
                return End::Failure(reason);
            }
        };

        let mut s = Sess { conn, pending: HashMap::new(), queued: Vec::new(), next_id: 1 };
        let (mut ping_seq, mut unanswered) = (0u64, 0u32);
        let mut drift_armed = true;
        let drift_at = Instant::now() + t.drift;
        let mut tick = interval(t.heartbeat);
        tick.set_missed_tick_behavior(MissedTickBehavior::Delay);

        let (end, graceful) = 'session: loop {
            let ev = tokio::select! {
                _ = tick.tick() => Ev::Tick,
                r = transport::recv(&mut s.conn) => Ev::Frame(r),
                c = child.wait() => Ev::Exit(c),
                m = self.rx.recv() => Ev::Msg(m),
                _ = sleep_until(drift_at), if drift_armed => Ev::Drift,
            };
            let closed = |e: &dyn std::fmt::Display| End::Failure(format!("engine closed the connection: {e}"));
            match ev {
                Ev::Tick => {
                    if unanswered >= t.misses {
                        break (End::Failure(format!("engine hung: {unanswered} pings unanswered")), false);
                    }
                    ping_seq += 1;
                    if let Err(e) = transport::send(&mut s.conn, &Frame::Ping { seq: ping_seq }).await {
                        break (closed(&e), false);
                    }
                    unanswered += 1;
                    let now = Instant::now();
                    let late: Vec<u64> = s.pending.iter().filter(|(_, (_, dl))| *dl <= now).map(|(id, _)| *id).collect();
                    for id in late {
                        if let Some((tx, _)) = s.pending.remove(&id) {
                            let o = Outcome::Err { error: IpcError::new(ErrorKind::Timeout, "engine did not answer in time") };
                            if let Some((_, cmd)) = self.inflight.take_if(|(i, _)| *i == id) {
                                self.step_result(&cmd, &o);
                            }
                            let _ = tx.send(o);
                        }
                    }
                }
                Ev::Frame(Err(e)) => break (closed(&e), false),
                Ev::Frame(Ok(None)) => break (closed(&"EOF"), false),
                Ev::Frame(Ok(Some(f))) => match f {
                    Frame::Pong { seq } => {
                        if seq == ping_seq {
                            unanswered = 0;
                        }
                    }
                    Frame::Res { id, outcome } => {
                        if let Some((_, cmd)) = self.inflight.take_if(|(i, _)| *i == id) {
                            self.step_result(&cmd, &outcome);
                        }
                        self.on_outcome(&outcome);
                        if let Some((tx, _)) = s.pending.remove(&id) {
                            let _ = tx.send(outcome);
                        }
                    }
                    Frame::Evt { evt: Event::BridgeReady { version, capabilities, musickit_build } } => {
                        drift_armed = false;
                        self.on_bridge_ready();
                        match check_bridge(&capabilities, musickit_build.as_deref()) {
                            Ok(()) => {
                                self.set(|c| c.bridge = Some(BridgeInfo { version, capabilities, musickit_build }));
                                self.bridge_seen = true;
                            }
                            Err(reason) => {
                                eprintln!("presto-core: drift: {reason}");
                                self.drift = true;
                                fail_queued(&mut s.queued, "engine drift");
                                self.set(|c| c.bridge = None);
                                self.status(EngineStatus::Drift { reason, log_path: log_path.clone() });
                            }
                        }
                    }
                    Frame::Evt { evt } => {
                        if self.on_event(evt) && hello.has(caps::WINDOW) {
                            let (tx, _rx) = oneshot::channel();
                            if let Err(e) = s.send(Job::Cmd(Command::ShowWindow { show: true }), tx).await {
                                break (closed(&e), false);
                            }
                        }
                    }
                    _ => {}
                },
                Ev::Exit(c) => break (End::Failure(format!("engine exited: {c:?}")), false),
                Ev::Drift => {
                    drift_armed = false;
                    let reason = format!("no bridge_ready within {:?}", t.drift);
                    eprintln!("presto-core: drift: {reason}");
                    self.drift = true;
                    fail_queued(&mut s.queued, "engine drift");
                    self.status(EngineStatus::Drift { reason, log_path: log_path.clone() });
                }
                Ev::Msg(None) => break (End::Shutdown(None), true),
                Ev::Msg(Some(Msg::Shutdown(tx))) => break (End::Shutdown(Some(tx)), true),
                Ev::Msg(Some(Msg::Restart)) => break (End::Restart, true),
                Ev::Msg(Some(Msg::Job(job, tx))) => {
                    if job.immediate() {
                        if matches!(job, Job::Cmd(_)) && !hello.has(caps::WINDOW) {
                            let _ = tx.send(unavail("engine cannot show its window"));
                        } else if let Err(e) = s.send(job, tx).await {
                            break (closed(&e), false);
                        }
                    } else if self.drift {
                        let _ = tx.send(unavail("engine drift"));
                    } else if self.auth.blocked() {
                        let _ = tx.send(auth_err());
                    } else if self.usable() {
                        if let Err(e) = s.send(job, tx).await {
                            break (closed(&e), false);
                        }
                    } else {
                        s.queued.push((job, tx));
                    }
                }
            }
            if let Err(e) = self.pump(&mut s).await {
                break 'session (closed(&e), false);
            }
        };

        for (tx, _) in s.pending.into_values() {
            let _ = tx.send(unavail("engine restarted"));
        }
        fail_queued(&mut s.queued, "engine restarted");
        let conn = s.conn;
        self.teardown(Some(conn), &mut child, pgid, graceful.then_some(t.quit_wait)).await;
        guard.0 = None;
        end
    }

    /// Close the socket (engine quits on EOF), optionally wait, then kill the group and drop the pidfile.
    async fn teardown(&self, conn: Option<Conn>, child: &mut tokio::process::Child, pgid: Pid, quit_wait: Option<Duration>) {
        drop(conn);
        if let Some(w) = quit_wait {
            let _ = timeout(w, child.wait()).await;
        }
        let _ = killpg(pgid, Signal::SIGTERM);
        if timeout(self.cfg.timings.kill_grace, child.wait()).await.is_err() {
            let _ = killpg(pgid, Signal::SIGKILL);
            let _ = child.wait().await;
        } else if !matches!(killpg(pgid, None), Err(Errno::ESRCH)) {
            let _ = killpg(pgid, Signal::SIGKILL);
        }
        let _ = std::fs::remove_file(&self.cfg.paths.pidfile);
    }
}

fn finish(tx: Option<oneshot::Sender<()>>) {
    if let Some(tx) = tx {
        let _ = tx.send(());
    }
}

fn fail_queued(q: &mut Vec<(Job, Tx)>, why: &str) {
    for (_, tx) in q.drain(..) {
        let _ = tx.send(unavail(why));
    }
}

async fn handshake(l: &tokio::net::UnixListener, connect: &Duration) -> Result<(Conn, Hello), String> {
    let (stream, _) = timeout(*connect, l.accept())
        .await
        .map_err(|_| format!("engine did not connect within {connect:?}"))?
        .map_err(|e| format!("accept failed: {e}"))?;
    let mut conn = transport::framed(stream);
    let hello = match timeout(Kind::Hello.timeout(), transport::recv(&mut conn)).await {
        Err(_) => return Err("engine hello timed out".into()),
        Ok(Ok(Some(Frame::Hello(h)))) => h,
        Ok(Ok(other)) => return Err(format!("expected engine hello, got {other:?}")),
        Ok(Err(e)) => return Err(format!("bad engine hello: {e}")),
    };
    hello.check(PROTO).map_err(|e| e.to_string())?;
    let ours = Hello::new(Role::Presto, &[], Some(format!("presto-core {}", env!("CARGO_PKG_VERSION"))));
    transport::send(&mut conn, &Frame::Hello(ours)).await.map_err(|e| format!("hello send failed: {e}"))?;
    Ok((conn, hello))
}

/// Registers the reply slot before sending so a send failure still fails the caller.
async fn dispatch(
    conn: &mut Conn,
    pending: &mut HashMap<u64, (Tx, Instant)>,
    next_id: &mut u64,
    job: Job,
    tx: Tx,
) -> Result<(), TransportError> {
    let id = *next_id;
    *next_id += 1;
    let frame = job.frame(id);
    let dl = Instant::now() + frame.kind().map_or(Kind::Command.timeout(), Kind::timeout);
    pending.insert(id, (tx, dl));
    transport::send(conn, &frame).await
}

//! The supervisor actor: owns the engine child, its socket and the pending-request map.
use crate::backoff::Backoff;
use crate::config::{CoreConfig, check_bridge};
use crate::paths::{Pidfile, new_log_file, sweep_stale, tail};
use crate::state::{BridgeInfo, CoreState, EngineStatus};
use nix::errno::Errno;
use nix::sys::signal::{Signal, killpg};
use nix::unistd::Pid;
use presto_ipc::transport::{self, Conn, TransportError};
use presto_ipc::{
    ApiRequest, Command, ErrorKind, Event, FaultSpec, Frame, Hello, IpcError, Kind, Outcome, PROTO,
    Role, caps,
};
use std::collections::HashMap;
use std::io;
use std::process::Stdio;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, watch};
use tokio::time::{Instant, MissedTickBehavior, interval, sleep, sleep_until, timeout};

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
        tokio::spawn(Actor { cfg, rx, state_tx, backoff }.run());
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

#[derive(PartialEq, Clone, Copy)]
enum Phase {
    Starting,
    Ready,
    Drift,
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

    /// Extension point for 03-05: every bridge_ready means the page reloaded.
    fn on_bridge_ready(&mut self) {}

    /// Extension point for 03-05: non-handshake events.
    fn on_event(&mut self, _evt: &Event) {}

    async fn run(mut self) {
        let mut n = 0u32;
        loop {
            let end = self.attempt(n).await;
            n += 1;
            let wake = match end {
                End::Shutdown(tx) => return finish(tx),
                End::Restart => Wake::Restart,
                End::Failure(reason) => {
                    eprintln!("presto-core: engine failed: {reason}");
                    self.set(|c| c.restarts += 1);
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
        self.set(|c| c.bridge = None);
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
        let (mut conn, hello) = match hs {
            Ok(x) => x,
            Err(reason) => {
                self.teardown(None, &mut child, pgid, None).await;
                guard.0 = None;
                return End::Failure(reason);
            }
        };

        let mut pending: HashMap<u64, (Tx, Instant)> = HashMap::new();
        let mut queued: Vec<(Job, Tx)> = Vec::new();
        let mut next_id = 1u64;
        let (mut ping_seq, mut unanswered) = (0u64, 0u32);
        let mut phase = Phase::Starting;
        let mut drift_armed = true;
        let drift_at = Instant::now() + t.drift;
        let mut tick = interval(t.heartbeat);
        tick.set_missed_tick_behavior(MissedTickBehavior::Delay);

        let (end, graceful) = 'session: loop {
            let ev = tokio::select! {
                _ = tick.tick() => Ev::Tick,
                r = transport::recv(&mut conn) => Ev::Frame(r),
                s = child.wait() => Ev::Exit(s),
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
                    if let Err(e) = transport::send(&mut conn, &Frame::Ping { seq: ping_seq }).await {
                        break (closed(&e), false);
                    }
                    unanswered += 1;
                    let now = Instant::now();
                    let late: Vec<u64> = pending.iter().filter(|(_, (_, dl))| *dl <= now).map(|(id, _)| *id).collect();
                    for id in late {
                        if let Some((tx, _)) = pending.remove(&id) {
                            let _ = tx.send(Outcome::Err { error: IpcError::new(ErrorKind::Timeout, "engine did not answer in time") });
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
                        if let Some((tx, _)) = pending.remove(&id) {
                            let _ = tx.send(outcome);
                        }
                    }
                    Frame::Evt { evt: Event::BridgeReady { version, capabilities, musickit_build } } => {
                        drift_armed = false;
                        self.on_bridge_ready();
                        match check_bridge(&capabilities, musickit_build.as_deref()) {
                            Ok(()) => {
                                self.set(|c| c.bridge = Some(BridgeInfo { version, capabilities, musickit_build }));
                                phase = Phase::Ready;
                                self.status(EngineStatus::Ready);
                                for (job, tx) in std::mem::take(&mut queued) {
                                    if let Err(e) = dispatch(&mut conn, &mut pending, &mut next_id, job, tx).await {
                                        break 'session (closed(&e), false);
                                    }
                                }
                            }
                            Err(reason) => {
                                eprintln!("presto-core: drift: {reason}");
                                phase = Phase::Drift;
                                fail_queued(&mut queued, "engine drift");
                                self.set(|c| c.bridge = None);
                                self.status(EngineStatus::Drift { reason, log_path: log_path.clone() });
                            }
                        }
                    }
                    Frame::Evt { evt } => self.on_event(&evt),
                    _ => {}
                },
                Ev::Exit(s) => break (End::Failure(format!("engine exited: {s:?}")), false),
                Ev::Drift => {
                    drift_armed = false;
                    let reason = format!("no bridge_ready within {:?}", t.drift);
                    eprintln!("presto-core: drift: {reason}");
                    phase = Phase::Drift;
                    fail_queued(&mut queued, "engine drift");
                    self.status(EngineStatus::Drift { reason, log_path: log_path.clone() });
                }
                Ev::Msg(None) => break (End::Shutdown(None), true),
                Ev::Msg(Some(Msg::Shutdown(tx))) => break (End::Shutdown(Some(tx)), true),
                Ev::Msg(Some(Msg::Restart)) => break (End::Restart, true),
                Ev::Msg(Some(Msg::Job(job, tx))) => {
                    if job.immediate() {
                        if matches!(job, Job::Cmd(_)) && !hello.has(caps::WINDOW) {
                            let _ = tx.send(unavail("engine cannot show its window"));
                        } else if let Err(e) = dispatch(&mut conn, &mut pending, &mut next_id, job, tx).await {
                            break (closed(&e), false);
                        }
                    } else {
                        match phase {
                            Phase::Ready => {
                                if let Err(e) = dispatch(&mut conn, &mut pending, &mut next_id, job, tx).await {
                                    break (closed(&e), false);
                                }
                            }
                            Phase::Starting => queued.push((job, tx)),
                            Phase::Drift => {
                                let _ = tx.send(unavail("engine drift"));
                            }
                        }
                    }
                }
            }
        };

        for (tx, _) in pending.into_values() {
            let _ = tx.send(unavail("engine restarted"));
        }
        fail_queued(&mut queued, "engine restarted");
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

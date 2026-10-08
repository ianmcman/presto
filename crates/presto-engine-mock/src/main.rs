mod catalog;
mod fault;
mod player;

use clap::Parser;
use fault::Faults;
use futures_util::StreamExt;
use player::Player;
use presto_ipc::transport::{self, TransportError};
use presto_ipc::{
    AuthState, Command, ErrorKind, Event, FaultSpec, Frame, Hello, IpcError, Kind, Outcome, PROTO,
    Role, caps,
};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::time::MissedTickBehavior;

#[derive(Parser)]
struct Args {
    #[arg(long, env = "PRESTO_SOCKET")]
    socket: PathBuf,
    /// Accepted for contract parity (D-02); unused by the mock.
    #[arg(long, env = "PRESTO_PROFILE")]
    #[allow(dead_code)]
    profile: Option<PathBuf>,
    /// Startup fault, repeatable: none, hang, crash[@ms], auth_expired, slow[=ms].
    #[arg(long = "fault", value_parser = |s: &str| s.parse::<FaultSpec>())]
    faults: Vec<FaultSpec>,
    /// Initial auth state.
    #[arg(long, default_value = "signed_in", value_parser = ["signed_in", "signed_out"])]
    auth: String,
    /// Never send bridge_ready or the initial auth event.
    #[arg(long)]
    bridge_missing: bool,
    /// Capabilities advertised in bridge_ready.
    #[arg(long, value_delimiter = ',', default_value = "playback,queue,api")]
    bridge_caps: Vec<String>,
    /// Test only: mimic MusicKit load (drops Play/Pause and first seek while loading, autoplays after load and seek).
    #[arg(long)]
    restore_quirks: bool,
}

struct Engine {
    player: Player,
    faults: Faults,
}

fn evts(events: Vec<Event>) -> Vec<Frame> {
    events.into_iter().map(|evt| Frame::Evt { evt }).collect()
}

fn crash(after_ms: Option<u64>) {
    match after_ms {
        None => {
            eprintln!("mock: crash fault");
            std::process::exit(101);
        }
        Some(ms) => {
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(ms)).await;
                eprintln!("mock: crash fault");
                std::process::exit(101);
            });
        }
    }
}

fn ok() -> Outcome {
    Outcome::Ok {
        data: serde_json::Value::Null,
    }
}

impl Engine {
    fn apply_fault(&mut self, spec: &FaultSpec) -> Vec<Frame> {
        if let FaultSpec::Crash { after_ms } = spec {
            crash(*after_ms);
        }
        evts(self.faults.apply(spec))
    }

    fn handle(&mut self, frame: Frame, now: Instant) -> Vec<Frame> {
        match frame {
            Frame::Ping { seq } => vec![Frame::Pong { seq }],
            Frame::Mock { id, fault } => {
                let mut out = self.apply_fault(&fault);
                out.push(Frame::Res { id, outcome: ok() });
                out
            }
            // answered by the engine host, regardless of auth or bridge state
            Frame::Cmd {
                id,
                cmd: Command::ShowWindow { .. },
            } => vec![Frame::Res { id, outcome: ok() }],
            Frame::Cmd { id, .. } | Frame::Req { id, .. }
                if self.faults.auth_expired || self.faults.signed_out =>
            {
                vec![Frame::Res {
                    id,
                    outcome: Outcome::Err {
                        error: IpcError::new(ErrorKind::AuthExpired, "mock: session expired"),
                    },
                }]
            }
            Frame::Cmd { id, cmd } => match self.audible_log(now, |p| p.apply(&cmd, now)) {
                Ok(events) => {
                    let mut out = evts(events);
                    out.push(Frame::Res {
                        id,
                        outcome: Outcome::Ok {
                            data: serde_json::Value::Null,
                        },
                    });
                    out
                }
                Err(error) => vec![Frame::Res {
                    id,
                    outcome: Outcome::Err { error },
                }],
            },
            Frame::Req { id, req } => vec![Frame::Res {
                id,
                outcome: catalog::handle(&req),
            }],
            other => {
                eprintln!("mock: ignoring unexpected frame {other:?}");
                vec![]
            }
        }
    }

    fn tick(&mut self, now: Instant) -> Vec<Frame> {
        evts(self.audible_log(now, |p| p.tick(now)))
    }

    /// Logs when the player becomes audible, so tests can assert restores are silent.
    fn audible_log<R>(&mut self, now: Instant, f: impl FnOnce(&mut Player) -> R) -> R {
        let was = self.player.audible();
        let r = f(&mut self.player);
        if !was && self.player.audible() {
            eprintln!("mock: audible at {} ms", self.player.position(now));
        }
        r
    }
}

/// A peer that hangs up mid-send is a normal shutdown.
fn send_failed(e: TransportError) -> ! {
    // ponytail: string match; LinesCodecError is not nameable without a tokio-util dep
    let msg = e.to_string();
    if msg.contains("Broken pipe") || msg.contains("Connection reset") {
        std::process::exit(0);
    }
    eprintln!("mock: send failed: {msg}");
    std::process::exit(1);
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let mut conn = match transport::connect(&args.socket).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("mock: cannot connect to {}: {e}", args.socket.display());
            std::process::exit(1);
        }
    };
    let me = Hello::new(
        Role::Engine,
        &[
            caps::PLAYBACK,
            caps::QUEUE,
            caps::API,
            caps::WINDOW,
            caps::MOCK,
        ],
        Some(format!("presto-engine-mock {}", env!("CARGO_PKG_VERSION"))),
    );
    if let Err(e) = transport::send(&mut conn, &Frame::Hello(me)).await {
        eprintln!("mock: hello send failed: {e}");
        std::process::exit(1);
    }
    match tokio::time::timeout(Kind::Hello.timeout(), transport::recv(&mut conn)).await {
        Ok(Ok(Some(Frame::Hello(h)))) => {
            if let Err(e) = h.check(PROTO) {
                eprintln!("{e}");
                std::process::exit(2);
            }
        }
        other => {
            eprintln!("mock: no valid hello from presto: {other:?}");
            std::process::exit(2);
        }
    }
    let mut engine = Engine {
        player: Player::new(Instant::now()),
        faults: Faults::default(),
    };
    engine.player.restore_quirks = args.restore_quirks;
    let signed_out = args.auth == "signed_out";
    engine.faults.signed_out = signed_out;
    let mut startup = vec![];
    if !args.bridge_missing {
        startup.extend(evts(vec![Event::BridgeReady {
            version: "mock-1".into(),
            capabilities: args.bridge_caps.clone(),
            musickit_build: Some("mock".into()),
        }]));
    }
    let mut fault_frames = vec![];
    for spec in &args.faults {
        fault_frames.extend(engine.apply_fault(spec));
    }
    if fault_frames.is_empty() && !args.bridge_missing {
        let state = if signed_out {
            AuthState::SignedOut
        } else {
            AuthState::SignedIn
        };
        fault_frames = evts(vec![Event::Auth { state }]);
    }
    startup.extend(fault_frames);
    let mut ticker = tokio::time::interval(Duration::from_millis(500));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let (mut sink, mut stream) = conn.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Frame>();
    // (frames, forced): forced frames (mock acks) bypass hang gating and slow delay
    let mut out = (startup, false);
    loop {
        let (frames, forced) = std::mem::take(&mut out);
        for f in frames {
            if engine.faults.hang && !forced {
                continue;
            }
            if let (Frame::Res { .. }, Some(d), false) = (&f, engine.faults.slow, forced) {
                let tx = tx.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(d).await;
                    let _ = tx.send(f);
                });
                continue;
            }
            if let Err(e) = transport::send(&mut sink, &f).await {
                send_failed(e);
            }
        }
        out = tokio::select! {
            r = transport::recv(&mut stream) => match r {
                Ok(Some(f)) => {
                    let forced = matches!(f, Frame::Mock { .. });
                    (engine.handle(f, Instant::now()), forced)
                }
                Ok(None) => std::process::exit(0),
                Err(TransportError::Json(e)) => {
                    eprintln!("mock: bad frame: {e}");
                    continue;
                }
                Err(e) => {
                    eprintln!("mock: transport error: {e}");
                    std::process::exit(1);
                }
            },
            _ = ticker.tick() => (engine.tick(Instant::now()), false),
            Some(f) = rx.recv() => {
                // delayed res: already slowed, still subject to hang
                if !engine.faults.hang
                    && let Err(e) = transport::send(&mut sink, &f).await
                {
                    send_failed(e);
                }
                continue;
            }
        };
    }
}

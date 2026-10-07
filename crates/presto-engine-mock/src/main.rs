mod catalog;
mod fault;
mod player;

use clap::Parser;
use fault::Faults;
use futures_util::StreamExt;
use player::Player;
use presto_ipc::transport::{self, TransportError};
use presto_ipc::{
    AuthState, ErrorKind, Event, FaultSpec, Frame, Hello, IpcError, Kind, Outcome, PROTO, Role, caps,
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
            Frame::Cmd { id, .. } | Frame::Req { id, .. } if self.faults.auth_expired => {
                vec![Frame::Res {
                    id,
                    outcome: Outcome::Err {
                        error: IpcError::new(ErrorKind::AuthExpired, "mock: session expired"),
                    },
                }]
            }
            Frame::Cmd { id, cmd } => match self.player.apply(&cmd, now) {
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
        evts(self.player.tick(now))
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
        &[caps::PLAYBACK, caps::QUEUE, caps::API, caps::MOCK],
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
    let mut startup = vec![];
    for spec in &args.faults {
        startup.extend(engine.apply_fault(spec));
    }
    if startup.is_empty() {
        startup = evts(vec![Event::Auth {
            state: AuthState::SignedIn,
        }]);
    }
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

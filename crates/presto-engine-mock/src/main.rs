mod catalog;
mod player;

use clap::Parser;
use futures_util::StreamExt;
use player::Player;
use presto_ipc::transport::{self, TransportError};
use presto_ipc::{AuthState, Event, Frame, Hello, Kind, Outcome, PROTO, Role, caps};
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
}

struct Engine {
    player: Player,
}

fn evts(events: Vec<Event>) -> Vec<Frame> {
    events.into_iter().map(|evt| Frame::Evt { evt }).collect()
}

impl Engine {
    fn handle(&mut self, frame: Frame, now: Instant) -> Vec<Frame> {
        match frame {
            Frame::Ping { seq } => vec![Frame::Pong { seq }],
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
            Frame::Mock { id, .. } => vec![Frame::Res {
                id,
                outcome: Outcome::Err {
                    error: presto_ipc::IpcError::new(
                        presto_ipc::ErrorKind::Internal,
                        "fault injection not implemented",
                    ),
                },
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
    let auth = Frame::Evt {
        evt: Event::Auth {
            state: AuthState::SignedIn,
        },
    };
    if let Err(e) = transport::send(&mut conn, &auth).await {
        send_failed(e);
    }

    let mut engine = Engine {
        player: Player::new(Instant::now()),
    };
    let mut ticker = tokio::time::interval(Duration::from_millis(500));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let (mut sink, mut stream) = conn.split();
    loop {
        let out = tokio::select! {
            r = transport::recv(&mut stream) => match r {
                Ok(Some(f)) => engine.handle(f, Instant::now()),
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
            _ = ticker.tick() => engine.tick(Instant::now()),
        };
        for f in out {
            if let Err(e) = transport::send(&mut sink, &f).await {
                send_failed(e);
            }
        }
    }
}

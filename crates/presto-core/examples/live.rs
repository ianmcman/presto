//! Live driver: runs presto-core against the real Electron engine.
//!
//! Prerequisite, once: `cd engine && npm install --allow-git=root && node node_modules/electron/install.js`
//!
//! cargo run -p presto-core --example live -- [--engine-dir DIR] [--state DIR] [--show]
//!
//! Stdin commands: play, pause, next, prev, seek <ms>, queue <id,id,...> [start],
//! vol <0..1>, get <path[?k=v&...]>, signin, restart, pids, quit.
//! CoreState is printed as one JSON line per change; command outcomes are prefixed `> `.
use presto_core::paths::Paths;
use presto_core::{Core, CoreConfig, CoreHandle, Launch, Timings};
use presto_ipc::{ApiRequest, Command, Outcome, transport};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned()
}

fn show(o: &Outcome, max: usize) {
    let mut s = serde_json::to_string(o).unwrap_or_default();
    if s.len() > max {
        s = s.chars().take(max).collect();
    }
    println!("> {s}");
}

async fn run(core: &CoreHandle, pidfile: &std::path::Path, line: &str) -> bool {
    let mut it = line.split_whitespace();
    let (Some(cmd), arg, arg2) = (it.next(), it.next(), it.next()) else { return true };
    let out = match cmd {
        "play" => core.command(Command::Play).await,
        "pause" => core.command(Command::Pause).await,
        "next" => core.command(Command::Next).await,
        "prev" => core.command(Command::Prev).await,
        "seek" => match arg.and_then(|a| a.parse().ok()) {
            Some(ms) => core.command(Command::Seek { ms }).await,
            None => return usage(),
        },
        "vol" => match arg.and_then(|a| a.parse().ok()) {
            Some(volume) => core.command(Command::SetVolume { volume }).await,
            None => return usage(),
        },
        "queue" => match arg {
            Some(ids) => {
                let ids = ids.split(',').map(String::from).collect();
                let start = arg2.and_then(|s| s.parse().ok()).unwrap_or(0);
                core.command(Command::SetQueue { ids, start, play: true }).await
            }
            None => return usage(),
        },
        "get" => match arg {
            Some(p) => {
                let (path, q) = p.split_once('?').unwrap_or((p, ""));
                let mut req = ApiRequest::get(path);
                for kv in q.split('&').filter(|s| !s.is_empty()) {
                    let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
                    req.query.insert(k.into(), v.into());
                }
                let out = core.request(req).await;
                show(&out, 400);
                return true;
            }
            None => return usage(),
        },
        "signin" => core.show_sign_in().await,
        "restart" => {
            core.restart_engine();
            println!("> restart requested");
            return true;
        }
        "pids" => {
            println!("> {}", std::fs::read_to_string(pidfile).unwrap_or_else(|e| e.to_string()));
            return true;
        }
        "quit" => return false,
        _ => return usage(),
    };
    show(&out, usize::MAX);
    true
}

fn usage() -> bool {
    println!("> unknown or malformed command; see the doc comment in examples/live.rs");
    true
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let engine_dir = PathBuf::from(flag(&args, "--engine-dir").unwrap_or_else(|| "engine".into()));
    let paths = flag(&args, "--state").map_or_else(Paths::from_env, |d| Paths::under(d.into()));
    let pidfile = paths.pidfile.clone();
    let extra: Vec<&str> = if args.iter().any(|a| a == "--show") { vec!["--show"] } else { vec![] };
    let launch = Launch::electron(&engine_dir, &extra)?;
    let socket = transport::socket_path().map_err(std::io::Error::other)?;
    let core = Core::start(CoreConfig {
        launcher: Arc::new(move |_| launch.clone()),
        socket,
        paths,
        timings: Timings::default(),
    })
    .await?;

    let mut rx = core.state();
    tokio::spawn(async move {
        let mut last_pos = 0u64;
        let mut last = String::new();
        while rx.changed().await.is_ok() {
            let s = rx.borrow().clone();
            let pos = s.player.position_ms;
            let mut j = s.clone();
            j.player.position_ms = 0;
            let key = serde_json::to_string(&j).unwrap_or_default();
            // Skip updates where only the position moved by less than 5 s.
            if key == last && pos.abs_diff(last_pos) < 5000 {
                continue;
            }
            last = key;
            last_pos = pos;
            println!("{}", serde_json::to_string(&s).unwrap_or_default());
        }
    });

    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        if !run(&core, &pidfile, &line).await {
            break;
        }
    }
    core.shutdown().await;
    Ok(())
}

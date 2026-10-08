//! Live driver: runs presto-core against the real Electron engine.
//!
//! Prerequisite, once: `cd engine && npm install --allow-git=root && node node_modules/electron/install.js`
//!
//! cargo run -p presto-core --example live -- [--engine-dir DIR] [--state DIR] [--show]
//!
//! `get` output is cut at 400 chars; set LIVE_MAX to raise it.
//!
//! Stdin commands: play, pause, next, prev, seek <ms>, queue <id,id,...> [start],
//! vol <0..1>, get <path[?k=v&...]>, probe (D-08 live API probe), signin, restart, pids, quit.
//! CoreState is printed as one JSON line per change; command outcomes are prefixed `> `.
use presto_core::paths::Paths;
use presto_core::{Core, CoreConfig, CoreHandle, EngineStatus, Launch, Timings};
use presto_ipc::{ApiRequest, AuthState, Command, Outcome, transport};
use serde_json::{Value, json};
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
                show(&out, std::env::var("LIVE_MAX").ok().and_then(|v| v.parse().ok()).unwrap_or(400));
                return true;
            }
            None => return usage(),
        },
        "probe" => {
            probe(core).await;
            return true;
        }
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

/// Copy of `v` with arrays cut to their first element and long strings cut to 24 chars.
fn shape(v: &Value) -> Value {
    match v {
        Value::Array(a) => Value::Array(a.first().map(shape).into_iter().collect()),
        Value::Object(m) => Value::Object(m.iter().map(|(k, v)| (k.clone(), shape(v))).collect()),
        Value::String(s) if s.chars().count() > 24 => Value::String(s.chars().take(24).collect()),
        _ => v.clone(),
    }
}

fn summarize(label: &str, o: &Outcome) -> String {
    let r = match o {
        Outcome::Err { error } => serde_json::to_value(error).unwrap_or_default(),
        Outcome::Ok { data } => {
            let items = data.get("data").and_then(Value::as_array).cloned().unwrap_or_default();
            let first5 = |f: &dyn Fn(&Value) -> Option<Value>| -> Vec<Value> {
                items.iter().filter_map(f).take(5).collect()
            };
            let names = first5(&|i| {
                let a = i.get("attributes")?;
                a.get("name").or_else(|| a.pointer("/title/stringForDisplay")).cloned()
            });
            let ids = first5(&|i| i.get("id").cloned());
            let mut types: Vec<Value> = items.iter().filter_map(|i| i.get("type").cloned()).collect();
            types.dedup();
            let mut keys = json!(data.as_object().map(|m| m.keys().collect::<Vec<_>>()));
            let mut len = json!(items.len());
            if let Some(res) = data.get("results").and_then(Value::as_object) {
                let m: serde_json::Map<String, Value> = res
                    .iter()
                    .map(|(k, v)| {
                        let n = v.get("data").and_then(Value::as_array).map_or(0, Vec::len);
                        (k.clone(), json!(n))
                    })
                    .collect();
                keys = json!(res.keys().collect::<Vec<_>>());
                len = Value::Object(m);
            }
            let mut sh = shape(data).to_string();
            sh = sh.chars().take(1500).collect();
            json!({
                "keys": keys, "names": names, "ids": ids, "types": types,
                "next": data.get("next"), "total": data.pointer("/meta/total"),
                "len": len, "shape": sh,
            })
        }
    };
    format!("probe {label} {r}")
}

async fn probe(core: &CoreHandle) {
    let mut rx = core.state();
    let ready = tokio::time::timeout(std::time::Duration::from_secs(90), async {
        loop {
            {
                let s = rx.borrow();
                if s.engine == EngineStatus::Ready && s.auth == Some(AuthState::SignedIn) {
                    return;
                }
            }
            if rx.changed().await.is_err() {
                std::future::pending::<()>().await;
            }
        }
    })
    .await;
    if ready.is_err() {
        println!("probe: not ready");
        return;
    }
    async fn go(core: &CoreHandle, label: &str, path: &str, q: &[(&str, &str)]) -> Outcome {
        let mut req = ApiRequest::get(path);
        for (k, v) in q {
            req.query.insert((*k).into(), (*v).into());
        }
        let o = core.request(req).await;
        println!("{}", summarize(label, &o));
        o
    }
    let sf = match go(core, "sf", "/v1/me/storefront", &[]).await {
        Outcome::Ok { data } => data.pointer("/data/0/id").and_then(Value::as_str).map(String::from),
        _ => None,
    }
    .unwrap_or_else(|| "us".into());
    for ty in ["songs", "albums", "artists", "playlists"] {
        for sort in ["none", "name", "-name", "dateAdded", "-dateAdded"] {
            let mut q = vec![("limit", "5")];
            if sort != "none" {
                q.push(("sort", sort));
            }
            go(core, &format!("lib-{ty}-sort={sort}"), &format!("/v1/me/library/{ty}"), &q).await;
        }
    }
    let songs = "/v1/me/library/songs";
    go(core, "lib-songs-limit100", songs, &[("limit", "100")]).await;
    go(core, "lib-songs-limit101", songs, &[("limit", "101")]).await;
    let rp = "/v1/me/recent/played";
    go(core, "recent-10", rp, &[("limit", "10")]).await;
    go(core, "recent-11", rp, &[("limit", "11")]).await;
    go(core, "recent-off10", rp, &[("limit", "10"), ("offset", "10")]).await;
    go(core, "recent-tracks-30", "/v1/me/recent/played/tracks", &[("limit", "30")]).await;
    let rid = match go(core, "recs", "/v1/me/recommendations", &[("limit", "10")]).await {
        Outcome::Ok { data } => data.pointer("/data/0/id").and_then(Value::as_str).map(String::from),
        _ => None,
    };
    if let Some(rid) = rid {
        go(core, "recs-id", &format!("/v1/me/recommendations/{rid}"), &[]).await;
        go(core, "recs-contents", &format!("/v1/me/recommendations/{rid}/contents"), &[("limit", "10")]).await;
    }
    let types = "songs,albums,artists,playlists";
    go(core, "search", &format!("/v1/catalog/{sf}/search"),
        &[("term", "love"), ("types", types), ("limit", "25"), ("with", "topResults")]).await;
    go(core, "hints", &format!("/v1/catalog/{sf}/search/hints"), &[("term", "lov"), ("limit", "10")]).await;
    go(core, "libsearch", "/v1/me/library/search", &[
        ("term", "a"),
        ("types", "library-songs,library-albums,library-artists,library-playlists"),
        ("limit", "25"),
    ])
    .await;
    println!("probe: done");
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

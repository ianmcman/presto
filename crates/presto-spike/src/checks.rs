//! The spike checklist (SPIKE-01..06), signin and measure.

use crate::session::{R, Session, unix_ts};
use crate::{Ctx, SongArgs, rss};
use presto_ipc::{AuthState, Command, Event, Frame, Outcome, PlayState, caps};
use std::collections::BTreeMap;
use std::io::Write;
use std::time::Duration;

pub struct Row {
    pub name: &'static str,
    pub pass: bool,
    pub detail: String,
}

fn secs(n: u64) -> Duration {
    Duration::from_secs(n)
}

fn err_text(o: &Outcome) -> String {
    match o {
        Outcome::Err { error } => format!("{:?}: {}", error.kind, error.message),
        Outcome::Ok { .. } => "ok".into(),
    }
}

fn ok_data(o: Outcome) -> Result<serde_json::Value, String> {
    match o {
        Outcome::Ok { data } => Ok(data),
        e => Err(err_text(&e)),
    }
}

fn hello_row(s: &Session, allow_mock: bool) -> Row {
    let h = &s.engine_hello;
    let ok = h.has(caps::PLAYBACK)
        && h.has(caps::QUEUE)
        && h.has(caps::API)
        && (allow_mock || !h.has(caps::MOCK));
    Row {
        name: "hello",
        pass: ok,
        detail: format!("engine={:?} caps={:?}", h.engine, h.capabilities),
    }
}

async fn progress_at(s: &mut Session, what: &str, within: Duration, pos: u64) -> R<()> {
    if matches!(s.last_progress, Some((p, _)) if p >= pos) {
        return Ok(());
    }
    s.wait_for(what, within, |f| match f {
        Frame::Evt {
            evt: Event::Progress { position_ms, .. },
        } if *position_ms >= pos => Some(()),
        _ => None,
    })
    .await
}

/// Waits for `state`, counting states already seen since index `from`.
async fn state_seen(s: &mut Session, from: usize, state: PlayState, within: Duration) -> R<()> {
    if s.states[from..].contains(&state) {
        return Ok(());
    }
    s.wait_for(&format!("{state:?}"), within, |f| match f {
        Frame::Evt {
            evt: Event::PlaybackState { state: st, .. },
        } if *st == state => Some(()),
        _ => None,
    })
    .await
}

async fn wait_auth(s: &mut Session, within: Duration) -> R<AuthState> {
    if let Some(a) = s.last_auth {
        return Ok(a);
    }
    s.wait_for("auth event", within, |f| match f {
        Frame::Evt {
            evt: Event::Auth { state },
        } => Some(*state),
        _ => None,
    })
    .await
}

async fn select_song(s: &mut Session, song: &SongArgs) -> R<String> {
    if let Some(id) = &song.song {
        return Ok(id.clone());
    }
    let sf = ok_data(s.get("/v1/me/storefront", &[]).await?)
        .ok()
        .and_then(|d| d["data"][0]["id"].as_str().map(String::from))
        .unwrap_or_else(|| "us".into());
    let q = [
        ("term", song.search_term.as_str()),
        ("types", "songs"),
        ("limit", "5"),
    ];
    let d = ok_data(s.get(&format!("/v1/catalog/{sf}/search"), &q).await?)
        .map_err(|e| format!("search failed: {e}"))?;
    let songs = d["results"]["songs"]["data"]
        .as_array()
        .filter(|a| !a.is_empty())
        .ok_or("search returned no songs")?;
    let pick = songs
        .iter()
        .find(|x| x["attributes"]["durationInMillis"].as_u64().unwrap_or(0) > 180_000)
        .unwrap_or(&songs[0]);
    let id = pick["id"].as_str().ok_or("song without id")?.to_string();
    println!(
        "song {id} {}",
        pick["attributes"]["name"].as_str().unwrap_or("?")
    );
    Ok(id)
}

async fn check_api(s: &mut Session) -> R<Row> {
    let o = s
        .get("/v1/me/library/playlists", &[("limit", "25")])
        .await?;
    let (pass, detail) = match ok_data(o) {
        Ok(d) if d["data"].is_array() => (
            true,
            format!(
                "{} playlists, next={}",
                d["data"].as_array().map_or(0, Vec::len),
                d.get("next").is_some()
            ),
        ),
        Ok(_) => (false, "response has no data array".into()),
        Err(e) => (false, e),
    };
    Ok(Row {
        name: "api",
        pass,
        detail,
    })
}

async fn check_playback(s: &mut Session, song: &SongArgs, min_play: u64, seek: u64) -> R<Row> {
    let fail = |detail: String| Row {
        name: "playback",
        pass: false,
        detail,
    };
    let id = select_song(s, song).await?;
    let from = s.states.len();
    let o = s
        .call(Command::SetQueue {
            ids: vec![id.clone()],
            start: 0,
            play: true,
        })
        .await?;
    if let Err(e) = ok_data(o) {
        return Ok(fail(format!("set_queue: {e}")));
    }
    let mut note = "";
    if state_seen(s, from, PlayState::Playing, secs(10))
        .await
        .is_err()
    {
        // autoplay policy finding: engine needed an explicit play
        note = " needed explicit play;";
        let o = s.call(Command::Play).await?;
        if let Err(e) = ok_data(o) {
            return Ok(fail(format!("play: {e}")));
        }
        state_seen(s, from, PlayState::Playing, secs(30)).await?;
    }
    progress_at(
        s,
        "progress past min-play",
        secs(min_play + 30),
        min_play * 1000,
    )
    .await?;
    let (_, dur) = s.last_progress.unwrap_or((0, 0));
    let mut seek_ms = seek * 1000;
    if seek_ms >= dur {
        seek_ms = dur.saturating_sub(20_000);
    }
    if let Err(e) = ok_data(s.call(Command::Seek { ms: seek_ms }).await?) {
        return Ok(fail(format!("seek: {e}")));
    }
    progress_at(s, "progress at seek target", secs(20), seek_ms).await?;
    let c = min_play.min(5) * 1000;
    progress_at(s, "progress after seek", secs(20), seek_ms + c).await?;
    let from = s.states.len();
    if let Err(e) = ok_data(s.call(Command::Pause).await?) {
        return Ok(fail(format!("pause: {e}")));
    }
    state_seen(s, from, PlayState::Paused, secs(10)).await?;
    let reached = s.last_progress.map_or(0, |p| p.0);
    let mut detail =
        format!("song={id} duration_ms={dur} reached={reached}ms seek={seek_ms}{note}");
    let pass = dur > 30_000;
    if !pass {
        detail.push_str(" preview-length duration (Pitfall 8)");
    }
    Ok(Row {
        name: "playback",
        pass,
        detail,
    })
}

fn check_events(s: &Session) -> Row {
    let c = |k| s.counts.get(k).copied().unwrap_or(0);
    let pass = s.states.contains(&PlayState::Playing)
        && s.states.contains(&PlayState::Paused)
        && c("progress") >= 3
        && c("track_changed") >= 1;
    Row {
        name: "events",
        pass,
        detail: format!("{:?}", s.counts),
    }
}

async fn run_one(
    s: &mut Session,
    name: &str,
    cx: &Ctx,
    song: &SongArgs,
    min_play: u64,
    seek: u64,
    auth_wait: u64,
) -> R<Row> {
    Ok(match name {
        "hello" => hello_row(s, cx.allow_mock),
        "musickit" => {
            let a = wait_auth(s, secs(auth_wait)).await?;
            Row {
                name: "musickit",
                pass: true,
                detail: format!("auth={a:?}"),
            }
        }
        "session" => {
            let a = wait_auth(s, secs(auth_wait)).await?;
            let pass = a == AuthState::SignedIn;
            let detail = if pass {
                "signed_in".into()
            } else {
                format!("{a:?}: run `presto-spike signin` first")
            };
            Row {
                name: "session",
                pass,
                detail,
            }
        }
        "api" => check_api(s).await?,
        "playback" => check_playback(s, song, min_play, seek).await?,
        "events" => check_events(s),
        other => return Err(format!("unknown check {other}").into()),
    })
}

fn static_name(n: &str) -> &'static str {
    ["hello", "musickit", "session", "api", "playback", "events"]
        .into_iter()
        .find(|x| *x == n)
        .unwrap_or("check")
}

pub async fn run_checks(
    cx: &Ctx,
    name: &str,
    song: &SongArgs,
    min_play: u64,
    seek: u64,
    auth_wait: u64,
) -> R<i32> {
    let names: Vec<&str> = if name == "all" {
        vec!["hello", "musickit", "session", "api", "playback", "events"]
    } else {
        vec![name]
    };
    let mut s = Session::start(&cx.eo).await?;
    let (hello_caps, hello_engine) = (
        s.engine_hello.capabilities.clone(),
        s.engine_hello.engine.clone(),
    );
    let mut rows = vec![];
    let mut broken: Option<String> = None;
    for n in names {
        if let Some(e) = &broken {
            rows.push(Row {
                name: static_name(n),
                pass: false,
                detail: e.clone(),
            });
            continue;
        }
        match run_one(&mut s, n, cx, song, min_play, seek, auth_wait).await {
            Ok(r) => rows.push(r),
            Err(e) => {
                broken = Some(e.to_string());
                rows.push(Row {
                    name: static_name(n),
                    pass: false,
                    detail: e.to_string(),
                });
            }
        }
    }
    s.shutdown().await?;
    let mut md = format!(
        "# presto-spike checklist\n- label: {}\n- unix_time: {}\n- engine: {hello_engine:?} caps={hello_caps:?}\n- engine_args: {:?}\n\n| check | result | detail |\n|---|---|---|\n",
        cx.eo.label,
        unix_ts(),
        cx.eo.extra
    );
    for r in &rows {
        let res = if r.pass { "PASS" } else { "FAIL" };
        println!("{res}  {}  {}", r.name, r.detail);
        md.push_str(&format!(
            "| {} | {res} | {} |\n",
            r.name,
            r.detail.replace('|', "/")
        ));
    }
    let path = cx
        .eo
        .log_dir
        .join(format!("checklist-{}-{}.md", cx.eo.label, unix_ts()));
    std::fs::write(path, md)?;
    Ok(if rows.iter().all(|r| r.pass) { 0 } else { 1 })
}

pub async fn signin(cx: &mut Ctx, wait: u64) -> R<i32> {
    cx.eo.extra.push("--show".into());
    let mut s = Session::start(&cx.eo).await?;
    let r = s
        .wait_for("sign-in", secs(wait), |f| match f {
            Frame::Evt {
                evt: Event::Auth {
                    state: AuthState::SignedIn,
                },
            } => Some(()),
            _ => None,
        })
        .await;
    let code = match r {
        Ok(()) => {
            println!("signed in; quitting engine cleanly so cookies flush");
            0
        }
        Err(e) => {
            println!("FAIL  signin  {e}");
            1
        }
    };
    s.shutdown().await?;
    Ok(code)
}

/// Pumps frames for `d`; a timeout is the expected outcome.
async fn pump(s: &mut Session, d: Duration) -> R<()> {
    match s.wait_for("pump", d, |_| None::<()>).await {
        Err(e) if !e.to_string().starts_with("timed out") => Err(e),
        _ => Ok(()),
    }
}

pub async fn measure(cx: &Ctx, song: &SongArgs, play: u64, interval: u64, settle: u64) -> R<i32> {
    let mut s = Session::start(&cx.eo).await?;
    let pid = s.pid().ok_or("engine has no pid")?;
    let path = cx
        .eo
        .log_dir
        .join(format!("rss-{}-{}.csv", cx.eo.label, unix_ts()));
    let mut csv = std::fs::File::create(path)?;
    writeln!(csv, "date,phase,rss_mib,pss_mib")?;
    let mut stats: BTreeMap<&str, Vec<(f64, f64)>> = BTreeMap::new();
    let mut take = |s: &Session, phase: &'static str| -> R<()> {
        let (r, p) = rss::sample(s.pid().unwrap_or(pid));
        writeln!(csv, "{},{phase},{r:.1},{p:.1}", unix_ts())?;
        stats.entry(phase).or_default().push((r, p));
        Ok(())
    };
    take(&s, "idle_after_hello")?;
    wait_auth(&mut s, secs(90)).await?;
    pump(&mut s, secs(settle)).await?;
    take(&s, "signed_in_idle")?;
    let id = select_song(&mut s, song).await?;
    let from = s.states.len();
    ok_data(
        s.call(Command::SetQueue {
            ids: vec![id],
            start: 0,
            play: true,
        })
        .await?,
    )?;
    state_seen(&mut s, from, PlayState::Playing, secs(30)).await?;
    for _ in 0..(play / interval.max(1)) {
        pump(&mut s, secs(interval)).await?;
        take(&s, "playing")?;
    }
    s.shutdown().await?;
    for (phase, v) in &stats {
        let rss = v.iter().map(|x| x.0);
        let min = rss.clone().fold(f64::MAX, f64::min);
        let max = rss.fold(0.0, f64::max);
        let last = v.last().map_or(0.0, |x| x.0);
        println!("{phase}: rss_mib min={min:.1} max={max:.1} last={last:.1}");
    }
    Ok(0)
}

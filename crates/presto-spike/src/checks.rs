use crate::session::{R, Session};
use crate::{Ctx, SongArgs};
use presto_ipc::caps;

pub struct Row {
    pub name: &'static str,
    pub pass: bool,
    pub detail: String,
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

pub async fn run_checks(
    cx: &Ctx,
    name: &str,
    _song: &SongArgs,
    _min_play: u64,
    _seek: u64,
    _auth_wait: u64,
) -> R<i32> {
    let s = Session::start(&cx.eo).await?;
    let mut rows = vec![hello_row(&s, cx.allow_mock)];
    if name != "hello" {
        rows.push(Row {
            name: "todo",
            pass: false,
            detail: "not implemented".into(),
        });
    }
    s.shutdown().await?;
    for r in &rows {
        println!("{}  {}  {}", if r.pass { "PASS" } else { "FAIL" }, r.name, r.detail);
    }
    Ok(if rows.iter().all(|r| r.pass) { 0 } else { 1 })
}

pub async fn signin(_cx: &mut Ctx, _wait: u64) -> R<i32> {
    println!("FAIL  signin  not implemented");
    Ok(1)
}

pub async fn measure(_cx: &Ctx, _song: &SongArgs, _p: u64, _i: u64, _s: u64) -> R<i32> {
    println!("FAIL  measure  not implemented");
    Ok(1)
}

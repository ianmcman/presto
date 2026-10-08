mod common;
use common::*;
use presto_core::EngineStatus;
use presto_ipc::{Command, ErrorKind, FaultSpec, Outcome, PlayState, RepeatMode};
use std::time::Duration;

const T: Duration = Duration::from_secs(15);

fn ready(s: &presto_core::CoreState) -> bool {
    s.engine == EngineStatus::Ready
}

fn crashing(n: u32) -> Vec<String> {
    let _ = n;
    vec!["--fault".into(), "crash".into()]
}

#[tokio::test]
async fn crash_restarts() {
    let r = rig(|_| vec![]).await;
    let mut rx = r.core.state();
    wait_for(&mut rx, T, ready).await;
    r.core.mock(FaultSpec::Crash { after_ms: None }).await;
    wait_for(&mut rx, T, |s| s.restarts == 1).await;
    let s = wait_for(&mut rx, T, ready).await;
    assert_eq!(s.restarts, 1);
    assert_eq!(r.calls(), 2);
    r.core.shutdown().await;
}

#[tokio::test]
async fn hang_restarts() {
    let r = rig(|_| vec![]).await;
    let mut rx = r.core.state();
    wait_for(&mut rx, T, ready).await;
    r.core.mock(FaultSpec::Hang).await;
    wait_for(&mut rx, T, |s| s.restarts == 1).await;
    let s = wait_for(&mut rx, T, ready).await;
    assert_eq!(s.restarts, 1);
    r.core.shutdown().await;
}

#[tokio::test]
async fn gives_up_after_five_fast_failures() {
    let r = rig(crashing).await;
    let mut rx = r.core.state();
    let s = wait_for(&mut rx, T, |s| matches!(s.engine, EngineStatus::Failed { .. })).await;
    let EngineStatus::Failed { log_path, .. } = s.engine else { unreachable!() };
    assert!(!log_path.as_os_str().is_empty());
    assert_eq!(r.calls(), 5);
    let Outcome::Err { error } = r.core.command(Command::Play).await else { panic!("expected err") };
    assert_eq!(error.kind, ErrorKind::Unavailable);
    r.core.shutdown().await;
}

#[tokio::test]
async fn restart_engine_after_failed() {
    let r = rig(|n| if n < 5 { crashing(n) } else { vec![] }).await;
    let mut rx = r.core.state();
    wait_for(&mut rx, T, |s| matches!(s.engine, EngineStatus::Failed { .. })).await;
    r.core.restart_engine();
    wait_for(&mut rx, T, ready).await;
    r.core.shutdown().await;
}

#[tokio::test]
async fn shutdown_reaps_engine_and_pidfile() {
    let r = rig(|_| vec![]).await;
    let mut rx = r.core.state();
    wait_for(&mut rx, T, ready).await;
    let pidfile = r.dir.path().join("state/engine.pid");
    let pid = presto_core::paths::Pidfile::read(&pidfile).unwrap().expect("pidfile").pid;
    r.core.shutdown().await;
    assert!(!pidfile.exists());
    assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
}

fn queue(ids: &[&str], start: u32) -> Command {
    Command::SetQueue { ids: ids.iter().map(|s| s.to_string()).collect(), start, play: true }
}

fn ids(s: &presto_core::CoreState) -> Vec<String> {
    s.queue.ids()
}

/// Rig with [s1,s2,s3] loaded at index 1 and the position at 30 s.
async fn playing_rig() -> (Rig, tokio::sync::watch::Receiver<presto_core::CoreState>) {
    playing_rig_with(|_| vec![]).await
}

/// First launch normal, every restarted engine drops the first seek and autoplays.
fn quirky(n: u32) -> Vec<String> {
    if n == 0 { vec![] } else { vec!["--restore-quirks".into()] }
}

async fn playing_rig_with(
    argv: impl Fn(u32) -> Vec<String> + Send + Sync + 'static,
) -> (Rig, tokio::sync::watch::Receiver<presto_core::CoreState>) {
    let r = rig(argv).await;
    let mut rx = r.core.state();
    wait_for(&mut rx, T, ready).await;
    assert!(matches!(r.core.command(queue(&["s1", "s2", "s3"], 1)).await, Outcome::Ok { .. }));
    assert!(matches!(r.core.command(Command::Seek { ms: 30000 }).await, Outcome::Ok { .. }));
    wait_for(&mut rx, T, |s| s.player.position_ms >= 30000).await;
    (r, rx)
}

async fn crash(r: &Rig, rx: &mut tokio::sync::watch::Receiver<presto_core::CoreState>, n: u32) -> presto_core::CoreState {
    r.core.mock(FaultSpec::Crash { after_ms: None }).await;
    wait_for(rx, T, |s| s.restarts == n && ready(s)).await
}

#[tokio::test]
async fn crash_restores_playing() {
    let (r, mut rx) = playing_rig().await;
    let s = crash(&r, &mut rx, 1).await;
    assert_eq!(ids(&s), ["s1", "s2", "s3"]);
    assert_eq!(s.queue.index, Some(1));
    assert_eq!(s.player.state, PlayState::Playing);
    assert!((30000..=34000).contains(&s.player.position_ms), "{}", s.player.position_ms);
    r.core.shutdown().await;
}

#[tokio::test]
async fn crash_restores_paused_when_paused() {
    let (r, mut rx) = playing_rig().await;
    r.core.command(Command::Pause).await;
    wait_for(&mut rx, T, |s| s.player.state == PlayState::Paused).await;
    let s = crash(&r, &mut rx, 1).await;
    assert_eq!(s.queue.index, Some(1));
    assert_eq!(s.player.state, PlayState::Paused);
    assert!((30000..=31000).contains(&s.player.position_ms), "{}", s.player.position_ms);
    r.core.shutdown().await;
}

#[tokio::test]
async fn second_crash_same_queue_restores_paused() {
    let (r, mut rx) = playing_rig().await;
    let s = crash(&r, &mut rx, 1).await;
    assert_eq!(s.player.state, PlayState::Playing);
    let s = crash(&r, &mut rx, 2).await;
    assert_eq!(ids(&s), ["s1", "s2", "s3"]);
    assert_eq!(s.player.state, PlayState::Paused);
    r.core.shutdown().await;
}

#[tokio::test]
async fn hang_restores_queue() {
    let (r, mut rx) = playing_rig().await;
    r.core.mock(FaultSpec::Hang).await;
    let s = wait_for(&mut rx, T, |s| s.restarts == 1 && ready(s)).await;
    assert_eq!(ids(&s), ["s1", "s2", "s3"]);
    assert_eq!(s.queue.index, Some(1));
    r.core.shutdown().await;
}

#[tokio::test]
async fn restore_keeps_shuffle_repeat_volume() {
    let (r, mut rx) = playing_rig().await;
    r.core.command(Command::SetShuffle { on: true }).await;
    r.core.command(Command::SetRepeat { mode: RepeatMode::All }).await;
    r.core.command(Command::SetVolume { volume: 0.3 }).await;
    wait_for(&mut rx, T, |s| s.player.shuffle && s.player.repeat == RepeatMode::All && s.player.volume == 0.3).await;
    // reset the mirror view by crashing, then require the values from the new engine process
    let s = crash(&r, &mut rx, 1).await;
    assert!(s.player.shuffle);
    assert_eq!(s.player.repeat, RepeatMode::All);
    assert!((s.player.volume - 0.3).abs() < 1e-6);
    r.core.shutdown().await;
}

#[tokio::test]
async fn quirky_crash_restores_position() {
    let (r, mut rx) = playing_rig_with(quirky).await;
    let s = crash(&r, &mut rx, 1).await;
    assert_eq!(s.queue.index, Some(1));
    assert_eq!(s.player.state, PlayState::Playing);
    assert!((30000..=36000).contains(&s.player.position_ms), "{}", s.player.position_ms);
    r.core.shutdown().await;
}

#[tokio::test]
async fn quirky_crash_restores_paused_when_paused() {
    let (r, mut rx) = playing_rig_with(quirky).await;
    r.core.command(Command::Pause).await;
    wait_for(&mut rx, T, |s| s.player.state == PlayState::Paused).await;
    let s = crash(&r, &mut rx, 1).await;
    assert_eq!(s.player.state, PlayState::Paused);
    tokio::time::sleep(Duration::from_millis(1500)).await;
    let s = rx.borrow().clone();
    assert_eq!(s.player.state, PlayState::Paused);
    assert!((30000..=31000).contains(&s.player.position_ms), "{}", s.player.position_ms);
    r.core.shutdown().await;
}

#[tokio::test]
async fn quirky_second_crash_restores_paused() {
    let (r, mut rx) = playing_rig_with(quirky).await;
    let s = crash(&r, &mut rx, 1).await;
    assert_eq!(s.player.state, PlayState::Playing);
    let s = crash(&r, &mut rx, 2).await;
    assert_eq!(s.player.state, PlayState::Paused);
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert_eq!(rx.borrow().player.state, PlayState::Paused);
    r.core.shutdown().await;
}

mod common;
use common::*;
use presto::app::App;
use presto::model::Action;
use presto_ipc::{Command, PlayState, RepeatMode};
use std::time::Duration;

const T: Duration = Duration::from_secs(10);

fn play(app: &mut App, ctx: &egui::Context, ids: &[&str]) {
    frames_until(app, ctx, T, |a, _| a.ready());
    app.dispatch(Action::PlayList { ids: ids.iter().map(|s| s.to_string()).collect(), start: 0, shuffle: false });
    frames_until(app, ctx, T, |a, _| a.state.player.track.is_some() && a.state.queue.items.len() == ids.len());
}

#[test]
fn bar_now_playing() {
    let (mut app, ctx, _tmp) = demo_app(&[]);
    play(&mut app, &ctx, &["s1", "s2", "s3"]);
    let t = frames_until(&mut app, &ctx, T, |_, t| has(t, "Neon Static"));
    assert!(has(&t, "Mock Artist One"));
    assert!(t.iter().any(|s| s.starts_with("0:0")), "{t:?}");
}

#[test]
fn transport_actions() {
    let (mut app, ctx, _tmp) = demo_app(&[]);
    play(&mut app, &ctx, &["s1", "s2", "s3"]);
    frames_until(&mut app, &ctx, T, |a, _| a.state.player.state == PlayState::Playing);
    app.dispatch(Action::TogglePlay);
    frames_until(&mut app, &ctx, T, |a, _| a.state.player.state == PlayState::Paused);
    app.dispatch(Action::TogglePlay);
    frames_until(&mut app, &ctx, T, |a, _| a.state.player.state == PlayState::Playing);
    app.dispatch(Action::CycleRepeat);
    frames_until(&mut app, &ctx, T, |a, _| a.state.player.repeat == RepeatMode::All);
    app.dispatch(Action::ToggleShuffle);
    frames_until(&mut app, &ctx, T, |a, _| a.state.player.shuffle);
    app.dispatch(Action::SetVolume(0.2));
    frames_until(&mut app, &ctx, T, |a, _| (a.state.player.volume - 0.2).abs() < 1e-3);
    app.dispatch(Action::ToggleMute);
    frames_until(&mut app, &ctx, T, |a, _| a.state.player.volume == 0.0);
    app.dispatch(Action::ToggleMute);
    frames_until(&mut app, &ctx, T, |a, _| (a.state.player.volume - 0.2).abs() < 1e-3);
}

#[test]
fn seek_once() {
    let (mut app, ctx, _tmp) = demo_app(&[]);
    play(&mut app, &ctx, &["s1", "s2", "s3"]);
    app.seek.drag(60_000);
    let seq = app.state.player.seq;
    let id = app.state.player.track.as_ref().unwrap().id.clone();
    let cmd = app.seek.release(app.now(), seq, &id);
    assert!(matches!(cmd, Some(Command::Seek { ms: 60_000 })), "{cmd:?}");
    assert!(app.seek.release(app.now(), seq, &id).is_none());
    app.backend.command(cmd.unwrap());
    frames_until(&mut app, &ctx, T, |a, _| a.state.player.position_ms >= 60_000);
}

fn open_queue(app: &mut App) {
    app.dispatch(Action::ToggleQueuePanel);
}

#[test]
fn queue_panel() {
    let (mut app, ctx, _tmp) = demo_app(&[]);
    play(&mut app, &ctx, &["s1", "s2", "s3"]);
    open_queue(&mut app);
    let t = frames_until(&mut app, &ctx, T, |_, t| has(t, "Up Next"));
    for s in ["Queue", "Now Playing", "Low Battery", "Cache Miss"] {
        assert!(has(&t, s), "missing {s}: {t:?}");
    }
}

#[test]
fn queue_empty() {
    let (mut app, ctx, _tmp) = demo_app(&[]);
    frames_until(&mut app, &ctx, T, |a, _| a.ready());
    open_queue(&mut app);
    let t = frames_until(&mut app, &ctx, T, |_, t| has(t, "Queue is empty"));
    assert!(has(&t, "Play a song or album to fill it."));
}

#[test]
fn queue_click_plays() {
    let (mut app, ctx, _tmp) = demo_app(&[]);
    play(&mut app, &ctx, &["s1", "s2", "s3"]);
    app.dispatch(Action::QueuePlayFrom(2));
    frames_until(&mut app, &ctx, T, |a, _| a.state.player.track.as_ref().is_some_and(|t| t.id == "s3"));
}

#[test]
fn queue_unavailable_row() {
    let (mut app, ctx, _tmp) = demo_app(&[]);
    play(&mut app, &ctx, &["s1", "s7", "s2"]);
    open_queue(&mut app);
    frames_until(&mut app, &ctx, T, |_, t| has(t, "Unavailable"));
}

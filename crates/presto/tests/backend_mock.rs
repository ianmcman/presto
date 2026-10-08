mod common;
use common::*;
use presto::model::{Event, Page};
use presto::queue_ops::QueueOp;
use presto_ipc::{Command, PlayState, RepeatMode};
use std::time::{Duration, Instant};

const T: Duration = Duration::from_secs(10);

fn ids(s: &presto_core::CoreState) -> Vec<String> {
    s.queue.ids()
}

fn toast_texts(b: &presto::backend::Backend, within: Duration) -> Vec<String> {
    let end = Instant::now() + within;
    let mut out = vec![];
    while Instant::now() < end && out.is_empty() {
        for e in b.poll() {
            if let Event::Toast(t) = e {
                out.push(t.text);
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    out
}

fn playing(b: &presto::backend::Backend) -> presto_core::CoreState {
    ready(b);
    b.play_list(vec!["s1".into(), "s2".into(), "s3".into()], 0, false);
    wait_state(b, T, |s| s.player.state == PlayState::Playing && ids(s).len() == 3)
}

#[test]
fn play_list() {
    let (b, _t) = demo_backend(&[]);
    ready(&b);
    b.play_list(vec!["s1".into(), "s2".into(), "s3".into()], 1, false);
    let s = wait_state(&b, T, |s| s.player.state == PlayState::Playing && ids(s).len() == 3);
    assert_eq!(ids(&s), ["s1", "s2", "s3"]);
    assert_eq!(s.queue.index, Some(1));
    b.shutdown();
}

#[test]
fn commands() {
    let (b, _t) = demo_backend(&[]);
    playing(&b);
    b.command(Command::Pause);
    wait_state(&b, T, |s| s.player.state == PlayState::Paused);
    b.command(Command::SetVolume { volume: 0.3 });
    wait_state(&b, T, |s| (s.player.volume - 0.3).abs() < 0.01);
    b.shutdown();
}

#[test]
fn commands_mode_and_seek() {
    let (b, _t) = demo_backend(&[]);
    playing(&b);
    b.command(Command::SetShuffle { on: true });
    wait_state(&b, T, |s| s.player.shuffle);
    b.command(Command::SetRepeat { mode: RepeatMode::All });
    wait_state(&b, T, |s| s.player.repeat == RepeatMode::All);
    b.command(Command::Seek { ms: 20000 });
    wait_state(&b, T, |s| s.player.position_ms >= 20000);
    b.shutdown();
}

#[test]
fn edit_keeps_position() {
    let (b, _t) = demo_backend(&[]);
    playing(&b);
    b.command(Command::Seek { ms: 30000 });
    let s = wait_state(&b, T, |s| s.player.position_ms >= 30000);
    b.edit_queue(QueueOp::PlayNext("s4".into()), s.queue.rev);
    let s = wait_state(&b, T, |s| ids(s) == ["s1", "s4", "s2", "s3"] && s.player.position_ms >= 30000);
    assert_eq!(s.player.track.as_ref().unwrap().id, "s1");
    assert!(toast_texts(&b, T).contains(&"Added to Up Next".to_string()));
    b.shutdown();
}

#[test]
fn edit_add() {
    let (b, _t) = demo_backend(&[]);
    let s = playing(&b);
    b.edit_queue(QueueOp::Add("s5".into()), s.queue.rev);
    wait_state(&b, T, |s| ids(s) == ["s1", "s2", "s3", "s5"]);
    assert!(toast_texts(&b, T).contains(&"Added to queue".to_string()));
    b.shutdown();
}

#[test]
fn edit_stale_rev() {
    let (b, _t) = demo_backend(&[]);
    let s = playing(&b);
    b.edit_queue(QueueOp::Remove(1), s.queue.rev - 1);
    let t = toast_texts(&b, T);
    assert!(t.iter().any(|x| x.contains("Queue changed")), "{t:?}");
    assert_eq!(ids(&b.state().borrow()), ["s1", "s2", "s3"]);
    b.shutdown();
}

#[test]
fn remove_only_item() {
    let (b, _t) = demo_backend(&[]);
    ready(&b);
    b.play_list(vec!["s1".into()], 0, false);
    let s = wait_state(&b, T, |s| s.player.state == PlayState::Playing && ids(s).len() == 1);
    b.edit_queue(QueueOp::Remove(0), s.queue.rev);
    wait_state(&b, T, |s| s.player.state == PlayState::Paused);
    b.shutdown();
}

#[test]
fn open_artist_named() {
    let (b, _t) = demo_backend(&[]);
    ready(&b);
    b.open_artist_named("Mock Artist One".into());
    let end = Instant::now() + T;
    loop {
        if let Some(Event::Open(Page::Artist(a))) = b.poll().into_iter().next() {
            assert_eq!(a.id, "a1");
            break;
        }
        assert!(Instant::now() < end, "no Open event");
        std::thread::sleep(Duration::from_millis(20));
    }
    b.shutdown();
}

#[test]
fn art_placeholder() {
    let (b, _t) = demo_backend(&[]);
    use presto::ui::widgets::Art;
    assert!(matches!(b.art(Some("https://example.invalid/artwork/x/{w}x{h}.jpg"), 160), Art::Placeholder));
    assert!(matches!(b.art(None, 160), Art::Placeholder));
    b.shutdown();
}

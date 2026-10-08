mod common;
use common::*;
use presto_core::EngineStatus;
use presto_ipc::{Command, FaultSpec, Outcome};
use std::time::Duration;

const T: Duration = Duration::from_secs(5);

fn q(ids: &[&str], start: u32) -> Command {
    Command::SetQueue { ids: ids.iter().map(|s| s.to_string()).collect(), start, play: true }
}

#[tokio::test]
async fn mirror_matches_after_rapid_changes() {
    let r = rig(|_| vec![]).await;
    let mut rx = r.core.state();
    wait_for(&mut rx, T, |s| s.engine == EngineStatus::Ready).await;
    let cmds = [
        q(&["s1", "s2", "s3", "s4", "s5"], 0),
        Command::Next,
        Command::Next,
        Command::Next,
        q(&["s3", "s4"], 1),
        Command::Next, // last item, repeat off: ends, no rev bump
        Command::Prev, // past 3 s since Ended, or first item: restarts, no rev bump
    ];
    for c in cmds {
        assert!(matches!(r.core.command(c).await, Outcome::Ok { .. }));
    }
    tokio::time::sleep(Duration::from_millis(300)).await;
    let s = rx.borrow().clone();
    assert_eq!(s.queue.ids(), ["s3", "s4"]);
    assert_eq!(s.queue.index, Some(1));
    // SetQueue x2 + three Next moves
    assert_eq!(s.queue.rev, 5);
    assert_eq!(s.player.track.as_ref().map(|t| t.id.clone()), Some(s.queue.items[1].id.clone()));
    r.core.shutdown().await;
}

#[tokio::test]
async fn mirror_generation_bumps_on_restart() {
    let r = rig(|_| vec![]).await;
    let mut rx = r.core.state();
    wait_for(&mut rx, T, |s| s.engine == EngineStatus::Ready).await;
    assert_eq!(rx.borrow().queue.generation, 1);
    r.core.command(q(&["s1", "s2"], 0)).await;
    r.core.mock(FaultSpec::Crash { after_ms: None }).await;
    let s = wait_for(&mut rx, T, |s| s.restarts == 1 && s.engine == EngineStatus::Ready).await;
    assert_eq!(s.queue.generation, 2);
    assert!(s.queue.rev >= 1);
    assert_eq!(s.queue.ids(), ["s1", "s2"]);
    r.core.shutdown().await;
}

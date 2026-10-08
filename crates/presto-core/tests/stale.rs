use presto_core::paths::{Pidfile, sweep_stale};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command};
use std::time::Duration;

fn spawn() -> Child {
    Command::new("sleep").arg("30").process_group(0).spawn().unwrap()
}

#[test]
fn kills_matching_stale_engine() {
    let t = tempfile::tempdir().unwrap();
    let f = t.path().join("engine.pid");
    let mut c = spawn();
    let pid = c.id() as i32;
    Pidfile::for_pid(pid).unwrap().write(&f).unwrap();
    assert_eq!(sweep_stale(&f, Duration::from_secs(1)).unwrap(), Some(pid));
    c.wait().unwrap();
    assert!(!f.exists());
}

#[test]
fn leaves_mismatching_pid_alone() {
    let t = tempfile::tempdir().unwrap();
    let f = t.path().join("engine.pid");
    let mut c = spawn();
    let mut pf = Pidfile::for_pid(c.id() as i32).unwrap();
    pf.start_ticks += 1;
    pf.write(&f).unwrap();
    assert_eq!(sweep_stale(&f, Duration::from_secs(1)).unwrap(), None);
    assert!(c.try_wait().unwrap().is_none());
    assert!(!f.exists());
    c.kill().unwrap();
    c.wait().unwrap();
}

#[test]
fn missing_pidfile_is_noop() {
    let t = tempfile::tempdir().unwrap();
    assert_eq!(sweep_stale(&t.path().join("none"), Duration::from_secs(1)).unwrap(), None);
}

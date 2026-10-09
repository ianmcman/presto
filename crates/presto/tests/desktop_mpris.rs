//! Integration test for MPRIS over a private session bus.

mod common;

use presto::desktop;
use std::process::Command;
use std::thread;
use std::time::Duration;

#[test]
fn mpris_private_bus() {
    // Check if a session bus is available
    if std::env::var("DBUS_SESSION_BUS_ADDRESS").is_err() {
        eprintln!("skipped: no session bus");
        return;
    }

    // Start the demo backend and wait for readiness
    let (backend, _tmp) = common::demo_backend(&[]);
    let _ready_state = common::ready(&backend);

    // Create a control interface
    let ctx = egui::Context::default();
    let ctl = backend.control(ctx);

    // Start the MPRIS service with a unique test name
    let mut test_app = desktop::np::App::new("presto-test", "Presto Test");
    test_app.desktop_entry = "presto".into();
    desktop::start(&backend, ctl, test_app);

    // Wait a bit for the service to register
    thread::sleep(Duration::from_millis(500));

    // Check that exactly one MPRIS player is registered
    let output = Command::new("busctl")
        .args(&["--user", "list"])
        .output()
        .expect("busctl failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mpris_players: Vec<&str> = stdout
        .lines()
        .filter(|line| line.contains("org.mpris.MediaPlayer2."))
        .collect();
    assert_eq!(mpris_players.len(), 1, "expected 1 MPRIS player, found {}", mpris_players.len());

    // Verify initial state is Stopped before any playback
    let status = Command::new("busctl")
        .args(&[
            "--user",
            "get-property",
            "org.mpris.MediaPlayer2.presto-test",
            "/org/mpris/MediaPlayer2",
            "org.mpris.MediaPlayer2.Player",
            "PlaybackStatus",
        ])
        .output()
        .expect("busctl get-property failed");
    let status_str = String::from_utf8_lossy(&status.stdout);
    assert!(status_str.contains("Stopped"), "initial PlaybackStatus should be Stopped, got: {}", status_str);

    // Check Rate property is d 1 (even at Stopped state)
    let rate = Command::new("busctl")
        .args(&[
            "--user",
            "get-property",
            "org.mpris.MediaPlayer2.presto-test",
            "/org/mpris/MediaPlayer2",
            "org.mpris.MediaPlayer2.Player",
            "Rate",
        ])
        .output()
        .expect("busctl get-property failed");
    let rate_str = String::from_utf8_lossy(&rate.stdout);
    assert!(rate_str.contains("1"), "Rate should be 1, got: {}", rate_str);

    // Check CanSeek property is b false (no track yet)
    let can_seek = Command::new("busctl")
        .args(&[
            "--user",
            "get-property",
            "org.mpris.MediaPlayer2.presto-test",
            "/org/mpris/MediaPlayer2",
            "org.mpris.MediaPlayer2.Player",
            "CanSeek",
        ])
        .output()
        .expect("busctl get-property failed");
    let can_seek_str = String::from_utf8_lossy(&can_seek.stdout);
    assert!(can_seek_str.contains("false"), "CanSeek should be false before playback, got: {}", can_seek_str);
}

#![allow(dead_code)]
use presto::backend::Backend;
use presto::launch::{demo_config, demo_extra};
use presto_core::CoreState;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tempfile::TempDir;

pub fn mock_bin() -> PathBuf {
    let p = std::env::current_exe().unwrap().parent().unwrap().parent().unwrap().join("presto-engine-mock");
    assert!(p.exists(), "run cargo build -p presto-engine-mock first");
    p
}

pub fn demo_backend(faults: &[&str]) -> (Backend, TempDir) {
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempfile::Builder::new().prefix("pb").tempdir_in("/tmp").unwrap();
    let rt = tmp.path().join("run");
    std::fs::create_dir(&rt).unwrap();
    std::fs::set_permissions(&rt, std::fs::Permissions::from_mode(0o700)).unwrap();
    let faults: Vec<String> = faults.iter().map(|s| s.to_string()).collect();
    let cfg = demo_config(&tmp.path().join("state"), &rt, mock_bin(), demo_extra(&faults)).unwrap();
    (Backend::start(cfg).unwrap(), tmp)
}

pub fn wait_state(b: &Backend, within: Duration, pred: impl Fn(&CoreState) -> bool) -> CoreState {
    let rx = b.state();
    let end = Instant::now() + within;
    loop {
        let s = rx.borrow().clone();
        if pred(&s) {
            return s;
        }
        assert!(Instant::now() < end, "timed out; last state: {s:?}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

pub fn ready(b: &Backend) -> CoreState {
    use presto_core::EngineStatus;
    wait_state(b, Duration::from_secs(10), |s| s.engine == EngineStatus::Ready)
}

use presto::app::App;
use presto::ui::widgets;

pub fn demo_app(extra: &[&str]) -> (App, egui::Context, TempDir) {
    let (backend, tmp) = demo_backend(extra);
    let ctx = egui::Context::default();
    let app = App::new(backend, true, &ctx);
    (app, ctx, tmp)
}

pub fn frame(app: &mut App, ctx: &egui::Context, events: Vec<egui::Event>) -> Vec<String> {
    let raw = egui::RawInput {
        events,
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 800.0))),
        ..Default::default()
    };
    let mut out = ctx.run_ui(raw, |ui| {
        app.logic(&ui.ctx().clone());
        app.frame_ui(ui);
    });
    out.textures_delta.clear();
    widgets::texts(&out)
}

/// Runs frames every 50 ms until `pred` holds; panics after `within`.
pub fn frames_until(app: &mut App, ctx: &egui::Context, within: Duration, pred: impl Fn(&App, &[String]) -> bool) -> Vec<String> {
    let end = Instant::now() + within;
    loop {
        let t = frame(app, ctx, vec![]);
        if pred(app, &t) {
            return t;
        }
        assert!(Instant::now() < end, "timed out; last texts: {t:?}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

pub fn has(t: &[String], s: &str) -> bool {
    t.iter().any(|x| x == s)
}

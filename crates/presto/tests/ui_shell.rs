mod common;
use common::*;
use presto::model::{Action, Page};
use presto::app::ViewData;
use presto_core::data::view::LibKind;
use std::time::Duration;

const T: Duration = Duration::from_secs(10);

fn ready(app: &mut presto::app::App, ctx: &egui::Context) -> Vec<String> {
    frames_until(app, ctx, T, |a, t| a.ready() && has(t, "Songs"))
}

fn toast_has(app: &presto::app::App, s: &str) -> bool {
    app.toasts.texts().iter().any(|t| t.contains(s))
}

#[test]
fn demo_shell() {
    let (mut app, ctx, _tmp) = demo_app(&[]);
    let t = ready(&mut app, &ctx);
    for s in ["DEMO", "Home", "Search", "Playlists", "Albums", "Artists", "Songs", "Settings"] {
        assert!(has(&t, s), "missing {s}: {t:?}");
    }
    assert!(!has(&t, "Sign in to Apple Music in the window"));
}

#[test]
fn navigation() {
    let (mut app, ctx, _tmp) = demo_app(&[]);
    ready(&mut app, &ctx);
    app.dispatch(Action::Open(Page::Library(LibKind::Songs)));
    app.dispatch(Action::Open(Page::Settings));
    app.dispatch(Action::Back);
    assert_eq!(app.page, Page::Library(LibKind::Songs));
    assert!(matches!(app.view, ViewData::List { .. }));
    app.dispatch(Action::Forward);
    assert_eq!(app.page, Page::Settings);
}

#[test]
fn signed_out_panel() {
    let (mut app, ctx, _tmp) = demo_app(&["signed_out"]);
    frames_until(&mut app, &ctx, T, |_, t| has(t, "Sign in to Apple Music in the window"));
}

#[test]
fn expired_banner() {
    let (mut app, ctx, _tmp) = demo_app(&["auth_expired"]);
    frames_until(&mut app, &ctx, T, |_, t| has(t, "Re-authenticate"));
}

#[test]
fn guard_all_failed() {
    let (mut app, ctx, _tmp) = demo_app(&[]);
    ready(&mut app, &ctx);
    app.dispatch(Action::PlayList { ids: vec!["s8".into()], start: 0, shuffle: false });
    let end = std::time::Instant::now() + Duration::from_secs(5);
    while !toast_has(&app, "Nothing in the queue can be played.") {
        assert!(std::time::Instant::now() < end, "toasts: {:?}", app.toasts.texts());
        frame(&mut app, &ctx, vec![]);
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn guard_skip() {
    let (mut app, ctx, _tmp) = demo_app(&[]);
    ready(&mut app, &ctx);
    app.dispatch(Action::PlayList { ids: vec!["s7".into(), "s1".into()], start: 0, shuffle: false });
    let end = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        frame(&mut app, &ctx, vec![]);
        let cur = app.state.player.track.as_ref().map(|t| t.id.clone());
        if cur.as_deref() == Some("s1") && (toast_has(&app, "Skipped") || toast_has(&app, "Couldn't play")) {
            break;
        }
        assert!(std::time::Instant::now() < end, "toasts: {:?} cur {cur:?}", app.toasts.texts());
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(toast_has(&app, "Region Locked"));
}

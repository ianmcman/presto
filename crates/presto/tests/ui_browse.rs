mod common;
use common::*;
use presto::app::{App, ViewData};
use presto::model::{Action, Page};
use presto_core::data::view::LibKind;
use std::time::Duration;

const T: Duration = Duration::from_secs(15);

/// Like `frame` on a tall screen, so shelves below the fold still paint.
fn tall(app: &mut App, ctx: &egui::Context) -> Vec<String> {
    let raw = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 3000.0))),
        ..Default::default()
    };
    let mut out = ctx.run_ui(raw, |ui| {
        app.logic(&ui.ctx().clone());
        app.frame_ui(ui);
    });
    out.textures_delta.clear();
    presto::ui::widgets::texts(&out)
}

fn until_tall(app: &mut App, ctx: &egui::Context, pred: impl Fn(&[String]) -> bool) {
    let end = std::time::Instant::now() + T;
    loop {
        let t = tall(app, ctx);
        if pred(&t) {
            return;
        }
        assert!(std::time::Instant::now() < end, "timed out; last texts: {t:?}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn start(extra: &[&str]) -> (App, egui::Context, tempfile::TempDir) {
    let (mut app, ctx, tmp) = demo_app(extra);
    frames_until(&mut app, &ctx, T, |a, t| a.ready() && has(t, "Songs"));
    (app, ctx, tmp)
}

#[test]
fn home_shelves() {
    let (mut app, ctx, _t) = start(&[]);
    let names = ["Recently Played", "Made for You", "Albums You Might Like", "New Releases", "Night Shift"];
    until_tall(&mut app, &ctx, |t| names.iter().all(|n| has(t, n)));
}

#[test]
fn library_songs_paging() {
    let (mut app, ctx, _t) = start(&[]);
    app.dispatch(Action::Open(Page::Library(LibKind::Songs)));
    frames_until(&mut app, &ctx, T, |_, t| has(t, "Song 00000") && has(t, "Unavailable"));
    let (key, mut rx) = match &app.view {
        ViewData::List { key, rx } => (key.clone(), rx.clone()),
        _ => panic!("not a list"),
    };
    app.dispatch(Action::LoadMore(key));
    let end = std::time::Instant::now() + T;
    while rx.borrow_and_update().items.len() < 200 {
        assert!(std::time::Instant::now() < end, "items: {}", rx.borrow().items.len());
        frame(&mut app, &ctx, vec![]);
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn library_albums() {
    let (mut app, ctx, _t) = start(&[]);
    app.dispatch(Action::Open(Page::Library(LibKind::Albums)));
    until_tall(&mut app, &ctx, |t| has(t, "Night Shift") && has(t, "Long Form"));
}

#[test]
fn search_results() {
    let (mut app, ctx, _t) = start(&[]);
    app.dispatch(Action::Open(Page::Search));
    app.dispatch(Action::SearchInput("neon".into()));
    app.dispatch(Action::SearchSubmit);
    frames_until(&mut app, &ctx, T, |_, t| has(t, "Neon Static"));
}

#[test]
fn search_empty() {
    let (mut app, ctx, _t) = start(&[]);
    app.dispatch(Action::Open(Page::Search));
    app.dispatch(Action::SearchInput("zzzz".into()));
    app.dispatch(Action::SearchSubmit);
    frames_until(&mut app, &ctx, T, |_, t| {
        has(t, "No results for \"zzzz\"") && has(t, "Check the spelling or try a different search.")
    });
}

#[test]
fn settings_confirm() {
    let (mut app, ctx, _t) = start(&[]);
    app.dispatch(Action::Open(Page::Settings));
    let t = frame(&mut app, &ctx, vec![]);
    assert!(has(&t, "Clear cache"));
    app.confirm_clear = true;
    frame(&mut app, &ctx, vec![]);
    let t = frame(&mut app, &ctx, vec![]);
    assert!(has(&t, "Clear cached library data and artwork? It will be downloaded again as you browse."), "{t:?}");
    assert!(has(&t, "Keep cache"));
}

#[test]
fn rate_limited_banner() {
    let (mut app, ctx, _t) = start(&["rate_limited=100"]);
    app.dispatch(Action::Open(Page::Library(LibKind::Songs)));
    frames_until(&mut app, &ctx, T, |_, t| has(t, "Apple Music is busy. Retrying shortly.") || has(t, "Try again"));
}


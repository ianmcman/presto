mod common;
use common::*;
use presto::app::{App, ViewData};
use presto::model::{Action, Page};
use presto_core::data::models::Item;
use presto_core::data::view::{LibKind, Sort, ViewKey};
use std::time::{Duration, Instant};

const T: Duration = Duration::from_secs(10);

/// Like `common::frames_until` with a chosen screen size.
fn frames_sized(app: &mut App, ctx: &egui::Context, size: egui::Vec2, pred: impl Fn(&App, &[String]) -> bool) {
    let end = Instant::now() + T;
    loop {
        let raw = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)), ..Default::default() };
        let mut out = ctx.run_ui(raw, |ui| {
            app.logic(&ui.ctx().clone());
            app.frame_ui(ui);
        });
        out.textures_delta.clear();
        let t = presto::ui::widgets::texts(&out);
        if pred(app, &t) {
            return;
        }
        assert!(Instant::now() < end, "timed out; last texts: {t:?}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

const TALL: egui::Vec2 = egui::vec2(1200.0, 1600.0);

fn any(t: &[String], s: &str) -> bool {
    t.iter().any(|x| x.contains(s))
}

fn ready(app: &mut App, ctx: &egui::Context) {
    frames_until(app, ctx, T, |a, t| a.ready() && has(t, "Songs"));
}

/// Library item by name, from the demo data layer.
fn find(app: &mut App, ctx: &egui::Context, kind: LibKind, name: &str) -> Item {
    let rx = app.backend.data().list(ViewKey::Library { kind, sort: Sort::Default });
    let end = Instant::now() + T;
    loop {
        frame(app, ctx, vec![]);
        if let Some(i) = rx.borrow().items.iter().find(|i| i.name == name) {
            return i.clone();
        }
        assert!(Instant::now() < end, "no {name} in {kind:?}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn toast_has(app: &App, s: &str) -> bool {
    app.toasts.texts().iter().any(|t| t.contains(s))
}

fn tracks_rx(app: &App) -> presto_core::data::view::ListState<Item> {
    let ViewData::Detail { tracks: Some((_, rx)), .. } = &app.view else { panic!("no tracks") };
    let s = rx.borrow().clone();
    s
}

#[test]
fn album_page() {
    let (mut app, ctx, _tmp) = demo_app(&[]);
    ready(&mut app, &ctx);
    let al = find(&mut app, &ctx, LibKind::Albums, "Night Shift");
    app.dispatch(Action::Open(Page::Album(al)));
    let t = frames_until(&mut app, &ctx, T, |_, t| any(t, "Afterglow Protocol"));
    assert!(has(&t, "Night Shift") && has(&t, "Album") && has(&t, "Play") && has(&t, "Shuffle"), "{t:?}");
    assert!(any(&t, "2023") && any(&t, "3 songs"), "{t:?}");
}

#[test]
fn album_play_skips_failure() {
    let (mut app, ctx, _tmp) = demo_app(&[]);
    ready(&mut app, &ctx);
    app.dispatch(Action::PlayList { ids: vec!["s8".into(), "s9".into(), "s10".into()], start: 0, shuffle: false });
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        frame(&mut app, &ctx, vec![]);
        let cur = app.state.player.track.as_ref().map(|t| t.id.clone());
        if cur.as_deref() == Some("s9") && toast_has(&app, "Couldn't play \"Dropped Signal\"") {
            break;
        }
        assert!(Instant::now() < end, "toasts {:?} cur {cur:?}", app.toasts.texts());
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn playlist_unavailable() {
    let (mut app, ctx, _tmp) = demo_app(&[]);
    ready(&mut app, &ctx);
    let p = find(&mut app, &ctx, LibKind::Playlists, "Unavailable Mix");
    app.dispatch(Action::Open(Page::Playlist(p)));
    let t = frames_until(&mut app, &ctx, T, |_, t| has(t, "Unavailable"));
    assert!(has(&t, "Unavailable Mix"), "{t:?}");
}

#[test]
fn playlist_paging() {
    let (mut app, ctx, _tmp) = demo_app(&[]);
    ready(&mut app, &ctx);
    let p = find(&mut app, &ctx, LibKind::Playlists, "Long Playlist");
    app.dispatch(Action::Open(Page::Playlist(p)));
    frames_until(&mut app, &ctx, T, |a, _| tracks_rx(a).items.len() == 100);
    let ViewData::Detail { tracks: Some((k, _)), .. } = &app.view else { panic!() };
    let k = k.clone();
    app.dispatch(Action::LoadMore(k));
    frames_until(&mut app, &ctx, T, |a, _| tracks_rx(a).items.len() == 150);
}

#[test]
fn long_title() {
    let (mut app, ctx, _tmp) = demo_app(&[]);
    ready(&mut app, &ctx);
    let al = find(&mut app, &ctx, LibKind::Albums, "Long Form");
    app.dispatch(Action::Open(Page::Album(al)));
    frames_sized(&mut app, &ctx, egui::vec2(700.0, 800.0), |a, t| {
        tracks_rx(a).items.len() == 2 && t.iter().any(|s| s.ends_with('…'))
    });
}

#[test]
fn artist_page() {
    let (mut app, ctx, _tmp) = demo_app(&[]);
    ready(&mut app, &ctx);
    let a = find(&mut app, &ctx, LibKind::Artists, "Mock Artist One");
    app.dispatch(Action::Open(Page::Artist(a)));
    frames_sized(&mut app, &ctx, TALL, |_, t| {
        ["Mock Artist One", "Top Songs", "Albums", "Singles & EPs", "Quiet Hours", "See all"].iter().all(|s| has(t, s))
    });
}

#[test]
fn artist_all() {
    let (mut app, ctx, _tmp) = demo_app(&[]);
    ready(&mut app, &ctx);
    let a = find(&mut app, &ctx, LibKind::Artists, "Mock Artist One");
    app.dispatch(Action::Open(Page::Artist(a)));
    frames_sized(&mut app, &ctx, TALL, |_, t| has(t, "Singles & EPs"));
    let Page::Artist(artist) = app.page.clone() else { panic!("{:?}", app.page) };
    app.dispatch(Action::Open(Page::ArtistAll { artist, singles: false }));
    frames_sized(&mut app, &ctx, TALL, |_, t| has(t, "Neon Static") && has(t, "Night Shift"));
}

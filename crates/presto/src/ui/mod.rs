//! Layout frame and page dispatch.
use crate::app::App;
use crate::i18n::tr;
use crate::model::Page;
use crate::theme;
use egui::{Margin, Ui};

pub mod artist;
pub mod detail;
pub mod home;
pub mod keys;
pub mod library;
pub mod player_bar;
pub mod queue;
pub mod search;
pub mod settings;
pub mod sidebar;
pub mod status;
pub mod toasts;
pub mod widgets;

pub fn show(app: &mut App, ui: &mut Ui) {
    egui::Panel::bottom("player").exact_size(theme::PLAYER_H).show(ui, |ui| player_bar::show(app, ui));
    if app.show_sidebar {
        egui::Panel::left("sidebar").exact_size(theme::SIDEBAR_W).show(ui, |ui| sidebar::show(app, ui));
    }
    if app.show_queue {
        egui::Panel::right("queue")
            .default_size(theme::QUEUE_W)
            .size_range(theme::QUEUE_MIN_W..=600.0)
            .resizable(true)
            .show(ui, |ui| queue::show(app, ui));
    }
    let frame = egui::Frame::new()
        .fill(app.palette.window)
        .inner_margin(Margin { left: theme::MD as i8, right: theme::MD as i8, top: theme::LG as i8, bottom: 0 });
    egui::CentralPanel::default().frame(frame).show(ui, |ui| {
        status::banners(app, ui);
        if status::blocking(app, ui) {
            return;
        }
        match app.page {
            Page::Home => home::show(app, ui),
            Page::Search => search::show(app, ui),
            Page::Library(_) => library::show(app, ui),
            Page::Album(_) | Page::Playlist(_) => detail::show(app, ui),
            Page::Artist(_) | Page::ArtistAll { .. } => artist::show(app, ui),
            Page::Settings => settings::show(app, ui),
        }
    });
    let (ctx, pal, now) = (ui.ctx().clone(), app.palette, app.now());
    app.toasts.show(&ctx, &pal, now);
    if app.show_shortcuts {
        shortcuts(app, &ctx);
    }
}

pub const SHORTCUTS: &[(&str, &str)] = &[
    ("Space", "Play / pause"),
    ("N / Ctrl+Right", "Next track"),
    ("P / Ctrl+Left", "Previous track"),
    ("Shift+Left / Shift+Right", "Seek 10 s"),
    ("Ctrl+Up / Ctrl+Down", "Volume"),
    ("M", "Mute"),
    ("S", "Shuffle"),
    ("R", "Repeat"),
    ("Q / Ctrl+Shift+Q", "Queue panel"),
    ("Ctrl+F or /", "Search"),
    ("Ctrl+B", "Sidebar"),
    ("Ctrl+H", "Home"),
    ("Ctrl+,", "Settings"),
    ("Alt+Left / Alt+Right", "Back / forward"),
    ("Ctrl+Shift+A", "Go to artist"),
    ("Ctrl+/ or ?", "This list"),
    ("Ctrl+Q / Ctrl+W", "Quit"),
];

fn shortcuts(app: &mut App, ctx: &egui::Context) {
    let pal = app.palette;
    let r = egui::Modal::new(egui::Id::new("shortcuts")).show(ctx, |ui| {
        ui.set_width(380.0);
        ui.label(egui::RichText::new(tr("Keyboard shortcuts")).font(theme::heading()).color(pal.text));
        ui.add_space(theme::SM);
        for (k, d) in SHORTCUTS {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(tr(k)).font(theme::mono()).color(pal.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(tr(d)).color(pal.secondary));
                });
            });
        }
    });
    if r.should_close() {
        app.show_shortcuts = false;
    }
}

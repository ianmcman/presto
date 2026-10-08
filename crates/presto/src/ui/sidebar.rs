//! Left navigation and engine status area.
use crate::app::App;
use crate::i18n::tr;
use crate::model::{Action, Page};
use crate::theme::{self, Icon};
use crate::ui::widgets::demo_chip;
use egui::{Rect, RichText, Sense, Ui, vec2};
use presto_core::EngineStatus;
use presto_core::data::view::LibKind;
use presto_ipc::AuthState;

fn item(ui: &mut Ui, app: &App, icon: Icon, label: &str, target: Page) -> Option<Action> {
    let pal = app.palette;
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::click());
    let active = app.page == target;
    if active || resp.hovered() {
        let fill = if active { pal.surface_active } else { pal.surface_hover };
        ui.painter().rect_filled(rect, 6, fill);
    }
    if active {
        ui.painter().rect_filled(Rect::from_min_size(rect.min + vec2(0.0, 6.0), vec2(3.0, rect.height() - 12.0)), 1, pal.accent);
    }
    let color = if active { pal.text } else { pal.secondary };
    theme::paint_icon(ui, icon, Rect::from_min_size(rect.min + vec2(8.0, 0.0), vec2(20.0, rect.height())), 18.0, color);
    ui.painter().text(rect.min + vec2(40.0, rect.height() / 2.0), egui::Align2::LEFT_CENTER, label, theme::body(), color);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    theme::focus_ring(ui, &resp);
    resp.clicked().then_some(Action::Open(target))
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let pal = app.palette;
    ui.add_space(theme::MD);
    let mut act = None;
    let items = [
        (Icon::House, tr("Home"), Page::Home),
        (Icon::Search, tr("Search"), Page::Search),
    ];
    for (i, l, p) in items {
        act = item(ui, app, i, &l, p).or(act);
    }
    ui.add_space(theme::MD);
    ui.label(RichText::new(tr("Library")).font(theme::label_strong()).color(pal.dim));
    let lib = [
        (Icon::ListMusic, tr("Playlists"), LibKind::Playlists),
        (Icon::Disc, tr("Albums"), LibKind::Albums),
        (Icon::Music, tr("Artists"), LibKind::Artists),
        (Icon::AudioLines, tr("Songs"), LibKind::Songs),
    ];
    for (i, l, k) in lib {
        act = item(ui, app, i, &l, Page::Library(k)).or(act);
    }
    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
        ui.add_space(theme::SM);
        act = item(ui, app, Icon::Settings, &tr("Settings"), Page::Settings).or(act.take());
        status(app, ui, &mut act);
    });
    if let Some(a) = act {
        app.act(a);
    }
}

fn status(app: &App, ui: &mut Ui, act: &mut Option<Action>) {
    let pal = app.palette;
    let dot = |ui: &mut Ui, color, text: String| {
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
            ui.painter().circle_filled(r.center(), 4.0, color);
            ui.label(RichText::new(text).font(theme::label()).color(pal.secondary));
        });
    };
    if matches!(app.state.auth, Some(AuthState::SignedOut | AuthState::Expired))
        && ui.button(tr("Sign in")).clicked()
    {
        *act = Some(Action::SignIn);
    }
    match &app.state.engine {
        EngineStatus::Starting => dot(ui, pal.dim, tr("Connecting…")),
        EngineStatus::Restarting { .. } => dot(ui, pal.warning, tr("Reconnecting…")),
        EngineStatus::Drift { .. } | EngineStatus::Failed { .. } => dot(ui, pal.danger, tr("Engine stopped")),
        EngineStatus::Ready => {}
    }
    if app.demo {
        demo_chip(ui, &pal);
    }
}

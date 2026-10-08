//! Blocking panels and banners for auth and engine state.
use crate::app::App;
use crate::i18n::tr;
use crate::model::Action;
use crate::theme;
use crate::ui::widgets::{BannerKind, banner};
use egui::{RichText, Ui};
use presto_core::EngineStatus;
use presto_ipc::AuthState;

fn panel(ui: &mut Ui, app: &App, title: &str, lines: &[String], mono: &[String], button: Option<&str>) -> bool {
    let pal = app.palette;
    let mut clicked = false;
    ui.vertical_centered(|ui| {
        ui.add_space(theme::XXL);
        ui.label(RichText::new(title).font(theme::heading()).color(pal.text));
        ui.add_space(theme::SM);
        for l in lines {
            ui.label(RichText::new(l).font(theme::body()).color(pal.secondary));
        }
        for l in mono {
            ui.label(RichText::new(l).font(theme::mono()).color(pal.dim));
        }
        if let Some(b) = button {
            ui.add_space(theme::MD);
            clicked = ui.button(b).clicked();
        }
    });
    clicked
}

/// Draws a full-page panel and returns true when the page content must not render.
pub fn blocking(app: &mut App, ui: &mut Ui) -> bool {
    let mut act = None;
    let blocked = match (&app.state.engine, app.state.auth) {
        (EngineStatus::Drift { reason, log_path }, _) => {
            let lines = [reason.clone(), format!("{}: {}", tr("Log"), log_path.display())];
            panel(ui, app, &tr("Apple's web player changed; update bridge.js"), &lines, &[], None);
            true
        }
        (EngineStatus::Failed { reason, log_path, log_tail }, _) => {
            let lines = [reason.clone(), format!("{}: {}", tr("Log"), log_path.display())];
            let tail: Vec<String> = log_tail.iter().rev().take(8).rev().cloned().collect();
            if panel(ui, app, &tr("The engine stopped."), &lines, &tail, Some(&tr("Restart engine"))) {
                act = Some(Action::RestartEngine);
            }
            true
        }
        (EngineStatus::Starting, _) => {
            ui.vertical_centered(|ui| {
                ui.add_space(theme::XXL);
                ui.add(egui::Spinner::new());
                ui.label(RichText::new(tr("Starting the engine…")).color(app.palette.secondary));
            });
            true
        }
        (_, Some(AuthState::SignedOut)) => {
            let lines = [tr("Sign in to the Apple Music window that opened, then come back here.")];
            if panel(ui, app, &tr("Sign in to Apple Music in the window"), &lines, &[], Some(&tr("Bring to front"))) {
                act = Some(Action::SignIn);
            }
            true
        }
        _ => false,
    };
    if let Some(a) = act {
        app.act(a);
    }
    blocked
}

pub fn banners(app: &mut App, ui: &mut Ui) {
    let pal = app.palette;
    if app.state.auth == Some(AuthState::Expired)
        && banner(ui, &pal, BannerKind::Warning, &tr("Session expired. Sign in again."), Some(&tr("Re-authenticate")))
    {
        app.act(Action::SignIn);
    }
    if matches!(app.state.engine, EngineStatus::Restarting { .. }) {
        banner(ui, &pal, BannerKind::Warning, &tr("Reconnecting to the engine…"), None);
    }
}

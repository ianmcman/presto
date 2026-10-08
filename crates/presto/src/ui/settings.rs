//! Settings: cache clearing and read-only diagnostics.
use crate::app::App;
use crate::i18n::tr;
use crate::model::Action;
use crate::theme;
use egui::{RichText, Ui};

pub fn show(app: &mut App, ui: &mut Ui) {
    let pal = app.palette;
    ui.label(RichText::new(tr("Settings")).font(theme::heading()).color(pal.text));
    ui.add_space(theme::MD);
    if ui.button(RichText::new(tr("Clear cache")).color(pal.danger)).clicked() {
        app.confirm_clear = true;
    }
    ui.add_space(theme::LG);
    let mut info = Vec::new();
    if let Some(b) = &app.state.bridge {
        info.push(format!("{} {}", tr("Bridge"), b.version));
        info.push(format!("{}: {}", tr("Capabilities"), b.capabilities.join(", ")));
    }
    if let Some(p) = &app.state.log_path {
        info.push(format!("{}: {}", tr("Log"), p.display()));
    }
    if app.demo {
        info.push(tr("Demo mode: mock engine"));
    }
    for l in info {
        ui.label(RichText::new(l).font(theme::label()).color(pal.secondary));
    }
    if app.confirm_clear {
        let r = egui::Modal::new(egui::Id::new("confirm_clear")).show(ui.ctx(), |ui| {
            ui.set_width(380.0);
            ui.label(tr("Clear cached library data and artwork? It will be downloaded again as you browse."));
            ui.add_space(theme::MD);
            ui.horizontal(|ui| {
                if ui.button(RichText::new(tr("Clear cache")).color(pal.danger)).clicked() {
                    app.act(Action::ClearCache);
                    app.confirm_clear = false;
                }
                if ui.button(tr("Keep cache")).clicked() {
                    app.confirm_clear = false;
                }
            });
        });
        if r.should_close() {
            app.confirm_clear = false;
        }
    }
}

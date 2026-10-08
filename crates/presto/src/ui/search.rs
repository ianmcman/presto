//! Search: field, scope toggle, hints, history and grouped results.
use crate::app::App;
use crate::i18n::tr;
use crate::model::Action;
use crate::theme;
use crate::ui::library::{error_copy, item_card, open_item, song_rows};
use crate::ui::widgets::{BannerKind, banner, empty_state, shelf, spinner_row};
use egui::{RichText, Ui};
use presto_core::data::models::Item;
use presto_core::data::search::Scope;

fn cards(app: &mut App, ui: &mut Ui, title: &str, items: &[Item]) {
    if items.is_empty() {
        return;
    }
    let pal = app.palette;
    shelf(ui, &pal, title, false, |ui| {
        for it in items {
            if item_card(app, ui, it).clicked() {
                open_item(app, it);
            }
        }
    });
    ui.add_space(theme::XL);
}

fn link(ui: &mut Ui, app: &App, text: &str) -> bool {
    ui.add(egui::Button::new(RichText::new(text).font(theme::body()).color(app.palette.text)).frame(false)).clicked()
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let pal = app.palette;
    let st = app.backend.search().state().borrow().clone();
    let id = egui::Id::new("search_field");
    let mut text = ui.data(|d| d.get_temp::<String>(id)).unwrap_or_else(|| st.term.clone());
    let resp = ui.add(egui::TextEdit::singleline(&mut text).id(id.with("edit")).hint_text(tr("Search")).desired_width(f32::INFINITY));
    if app.focus_search || (st.term.is_empty() && ui.memory(|m| m.focused().is_none())) {
        resp.request_focus();
        app.focus_search = false;
    }
    if resp.changed() {
        app.act(Action::SearchInput(text.clone()));
    }
    if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        app.act(Action::SearchSubmit);
    }
    ui.data_mut(|d| d.insert_temp(id, text.clone()));
    ui.horizontal(|ui| {
        for (label, scope) in [("Apple Music", Scope::Catalog), ("Library", Scope::Library)] {
            if ui.selectable_label(st.scope == scope, tr(label)).clicked() && st.scope != scope {
                app.act(Action::SearchScope(scope));
            }
        }
    });
    ui.add_space(theme::MD);
    if let Some(e) = &st.error {
        banner(ui, &pal, BannerKind::Error, &error_copy(&e.kind), None);
        ui.add_space(theme::SM);
    }
    if text.trim().is_empty() {
        for h in &st.history {
            if link(ui, app, h) {
                ui.data_mut(|d| d.insert_temp(id, h.clone()));
                app.act(Action::SearchInput(h.clone()));
                app.act(Action::SearchSubmit);
            }
        }
        return;
    }
    if st.results.is_none() {
        for h in &st.hints {
            if link(ui, app, h) {
                ui.data_mut(|d| d.insert_temp(id, h.clone()));
                app.act(Action::SearchInput(h.clone()));
                app.act(Action::SearchSubmit);
            }
        }
    }
    if st.searching {
        spinner_row(ui, &pal);
    }
    let Some(r) = &st.results else { return };
    if r.songs.is_empty() && r.albums.is_empty() && r.artists.is_empty() && r.playlists.is_empty() && r.top.is_empty() {
        if !st.searching && st.error.is_none() {
            let title = tr("No results for \"{query}\"").replace("{query}", &r.term);
            empty_state(ui, &pal, &title, &tr("Check the spelling or try a different search."));
        }
        return;
    }
    egui::ScrollArea::vertical().id_salt("search_results").auto_shrink(false).show(ui, |ui| {
        cards(app, ui, &tr("Top Results"), &r.top);
        if !r.songs.is_empty() {
            ui.label(RichText::new(tr("Songs")).font(theme::heading()).color(pal.text));
            ui.add_space(theme::SM);
            // song_rows scrolls itself; cap its height so the shelves below stay reachable
            ui.allocate_ui(egui::vec2(ui.available_width(), theme::ROW_H * r.songs.len().min(5) as f32), |ui| {
                song_rows(app, ui, &r.songs, "search_songs");
            });
            ui.add_space(theme::XL);
        }
        cards(app, ui, &tr("Albums"), &r.albums);
        cards(app, ui, &tr("Artists"), &r.artists);
        cards(app, ui, &tr("Playlists"), &r.playlists);
    });
}

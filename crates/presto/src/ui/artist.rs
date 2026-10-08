//! Artist page and its "See all" grid (D-10).
use crate::app::{App, ViewData};
use crate::i18n::tr;
use crate::model::{Action, Page, page_for};
use crate::theme;
use crate::ui::detail::{blocked, ids, play_buttons, track_rows, year};
use crate::ui::widgets::{self, card, shelf};
use egui::{Id, RichText, Ui};
use presto_core::data::models::Item;

fn cards(app: &mut App, ui: &mut Ui, items: &[Item]) {
    let pal = app.palette;
    for it in items {
        let art = app.backend.art(it.artwork.as_ref().map(|a| a.url.as_str()), 320);
        if card(ui, &pal, &art, &it.id, &it.name, year(it), false).clicked()
            && let Some(p) = page_for(it)
        {
            app.act(Action::Open(p));
        }
    }
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let pal = app.palette;
    let (head, all) = match &app.page {
        Page::Artist(i) => (i.clone(), None),
        Page::ArtistAll { artist, singles } => (artist.clone(), Some(*singles)),
        _ => return,
    };
    let ViewData::Detail { key, rx, .. } = &app.view else { return };
    let key = key.clone();
    let st = rx.borrow().clone();
    if blocked(app, ui, &key, &st) {
        return;
    }
    let Some(d) = st.items.first() else { return };
    // Library artist: swap to the catalog artist, which carries the top songs and albums.
    if head.library
        && let Some(cid) = &d.catalog_id
    {
        let item = Item { id: cid.clone(), library: false, ..d.head.clone() };
        let page = if let Some(singles) = all { Page::ArtistAll { artist: item, singles } } else { Page::Artist(item) };
        app.act(Action::Replace(page));
        return;
    }

    egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
        if let Some(singles) = all {
            let title = if singles { tr("Singles & EPs") } else { tr("Albums") };
            ui.label(RichText::new(format!("{}: {title}", d.head.name)).font(theme::heading()).color(pal.text));
            ui.add_space(theme::MD);
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Min).with_main_wrap(true), |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(theme::MD, theme::MD);
                cards(app, ui, if singles { &d.singles } else { &d.albums });
            });
            return;
        }
        ui.add_space(theme::XXXL - theme::LG);
        ui.horizontal(|ui| {
            let art = app.backend.art(d.head.artwork.as_ref().map(|a| a.url.as_str()), 400);
            widgets::artwork(ui, &pal, &art, &d.head.id, theme::HERO_ART, true);
            ui.add_space(theme::MD);
            ui.vertical(|ui| {
                ui.label(RichText::new(tr("Artist")).font(theme::label()).color(pal.secondary));
                widgets::ellipsized(ui, &d.head.name, theme::display(), pal.text, (ui.available_width() - theme::MD).max(100.0));
                ui.add_space(theme::MD);
                play_buttons(app, ui, ids(&d.top_songs));
            });
        });
        ui.add_space(theme::LG);

        if !d.top_songs.is_empty() {
            ui.label(RichText::new(tr("Top Songs")).font(theme::heading()).color(pal.text));
            ui.add_space(theme::SM);
            let more_id = Id::new("artist_top_more");
            let expanded = ui.data(|m| m.get_temp::<bool>(more_id)).unwrap_or(false);
            let n = if expanded { d.top_songs.len() } else { d.top_songs.len().min(5) };
            ui.spacing_mut().item_spacing.y = 0.0;
            track_rows(app, ui, &d.top_songs, 0..n, false, "top");
            ui.spacing_mut().item_spacing.y = theme::SM;
            if d.top_songs.len() > 5 {
                let label = if expanded { tr("Show less") } else { tr("Show more") };
                if ui.add(egui::Button::new(RichText::new(label).color(pal.accent)).frame(false)).clicked() {
                    ui.data_mut(|m| m.insert_temp(more_id, !expanded));
                }
            }
            ui.add_space(theme::LG);
        }
        for (title, list, singles) in [(tr("Albums"), &d.albums, false), (tr("Singles & EPs"), &d.singles, true)] {
            if list.is_empty() {
                continue;
            }
            if shelf(ui, &pal, &title, true, |ui| cards(app, ui, list)) {
                app.act(Action::Open(Page::ArtistAll { artist: d.head.clone(), singles }));
            }
            ui.add_space(theme::LG);
        }
    });
}

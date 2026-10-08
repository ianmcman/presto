//! Home: recently played and Apple's recommendation shelves.
use crate::app::{App, ViewData};
use crate::i18n::tr;
use crate::theme;
use crate::ui::library::{item_card, list_states, open_item};
use crate::ui::widgets::shelf;
use egui::Ui;
use presto_core::data::models::Item;
use presto_core::data::view::ViewKey;

fn cards(app: &mut App, ui: &mut Ui, title: &str, items: &[Item]) {
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

pub fn show(app: &mut App, ui: &mut Ui) {
    let ViewData::Home { recent, shelves } = &app.view else { return };
    let (recent, shelves) = (recent.clone(), shelves.clone());
    let (r, s) = (recent.borrow(), shelves.borrow());
    let empty = ("Nothing to show yet", "Play some music and your recent activity will appear here.");
    egui::ScrollArea::vertical().id_salt("home").auto_shrink(false).show(ui, |ui| {
        let a = list_states(app, ui, &r, &ViewKey::RecentlyPlayed, empty);
        let b = list_states(app, ui, &s, &ViewKey::Recommendations, empty);
        if a {
            cards(app, ui, &tr("Recently Played"), &r.items);
        }
        if b {
            for sh in &s.items {
                cards(app, ui, &sh.title, &sh.items);
            }
        }
    });
}

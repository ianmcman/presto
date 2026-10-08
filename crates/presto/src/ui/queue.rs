//! Queue side panel: Now Playing, then Up Next.
use crate::app::App;
use crate::i18n::tr;
use crate::model::Action;
use crate::theme::{self, Icon};
use crate::ui::widgets::{RowEvent, RowMenu, TrackRow, empty_state, icon_button, track_row};
use egui::{Id, RichText};

const MENU: &[RowMenu] = &[RowMenu::PlayFromHere, RowMenu::PlayNext, RowMenu::AddToQueue, RowMenu::Remove];

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let pal = app.palette;
    let mut acts = Vec::new();
    ui.horizontal(|ui| {
        ui.label(RichText::new(tr("Queue")).font(theme::heading()).color(pal.text));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if icon_button(ui, &pal, Icon::X, 16.0, false, &tr("Close queue")).clicked() {
                acts.push(Action::ToggleQueuePanel);
            }
        });
    });
    let items = &app.state.queue.items;
    if items.is_empty() {
        empty_state(ui, &pal, &tr("Queue is empty"), &tr("Play a song or album to fill it."));
    } else {
        let cur = app.state.queue.index.map(|i| i as usize).filter(|i| *i < items.len());
        let mut row = |ui: &mut egui::Ui, i: usize| {
            let item = &items[i];
            let art = app.backend.art(item.artwork_url.as_deref(), 160);
            let r = TrackRow {
                number: None,
                title: &item.title,
                subtitle: Some(&item.artist),
                duration_ms: Some(item.duration_ms),
                art: Some((&art, &item.id)),
                playing: Some(i) == cur,
                unavailable: app.guard.unavailable(&item.id, item.playable),
                menu: MENU,
            };
            match track_row(ui, &pal, Id::new(("queue_row", i)), &r) {
                RowEvent::Click | RowEvent::DoubleClick | RowEvent::Menu(RowMenu::PlayFromHere) => acts.push(Action::QueuePlayFrom(i as u32)),
                RowEvent::Menu(RowMenu::PlayNext) => acts.push(Action::QueuePlayNext(item.id.clone())),
                RowEvent::Menu(RowMenu::AddToQueue) => acts.push(Action::QueueAdd(item.id.clone())),
                RowEvent::Menu(RowMenu::Remove) => acts.push(Action::QueueRemove(i as u32)),
                RowEvent::None => {}
            }
        };
        ui.label(RichText::new(tr("Now Playing")).font(theme::label()).color(pal.secondary));
        if let Some(c) = cur {
            row(ui, c);
        }
        ui.add_space(theme::SM);
        ui.label(RichText::new(tr("Up Next")).font(theme::label()).color(pal.secondary));
        let first = cur.map_or(0, |c| c + 1);
        let rest = items.len() - first;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(ui, theme::ROW_H, rest, |ui, range| {
            for k in range {
                row(ui, first + k);
            }
        });
    }
    for a in acts {
        app.act(a);
    }
}

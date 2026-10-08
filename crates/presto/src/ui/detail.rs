//! Album and playlist page (D-09), plus helpers shared with the artist page.
use crate::app::{App, ViewData};
use crate::i18n::tr;
use crate::model::{Action, Page, queue_id};
use crate::theme;
use crate::ui::widgets::{self, BannerKind, RowEvent, RowMenu, TrackRow, banner, loading_rows, spinner_row};
use egui::{Id, RichText, Ui};
use presto_core::data::models::{Item, ItemKind};
use presto_core::data::view::{ListState, Phase, ViewKey};

const MENU: [RowMenu; 2] = [RowMenu::PlayNext, RowMenu::AddToQueue];

pub fn ids(items: &[Item]) -> Vec<String> {
    items.iter().map(queue_id).collect()
}

pub fn year(i: &Item) -> Option<&str> {
    i.release_date.as_deref().and_then(|d| d.get(..4))
}

fn plural(n: usize) -> String {
    if n == 1 { tr("1 song") } else { format!("{n} {}", tr("songs")) }
}

fn length(ms: u64) -> String {
    let m = ms / 60_000;
    if m >= 60 { format!("{} hr {} min", m / 60, m % 60) } else { format!("{m} min") }
}

/// Play (accent) and Shuffle (outlined) over `ids`.
pub fn play_buttons(app: &mut App, ui: &mut Ui, ids: Vec<String>) {
    let pal = app.palette;
    ui.horizontal(|ui| {
        let play = egui::Button::new(RichText::new(tr("Play")).font(theme::body_strong()).color(pal.on_accent))
            .fill(pal.accent)
            .min_size(egui::vec2(96.0, theme::HIT));
        let shuffle = egui::Button::new(RichText::new(tr("Shuffle")).font(theme::body_strong()).color(pal.text))
            .fill(egui::Color32::TRANSPARENT)
            .stroke(egui::Stroke::new(1.0, pal.outline))
            .min_size(egui::vec2(96.0, theme::HIT));
        let enabled = !ids.is_empty();
        if ui.add_enabled(enabled, play).clicked() {
            app.act(Action::PlayList { ids: ids.clone(), start: 0, shuffle: false });
        }
        if ui.add_enabled(enabled, shuffle).clicked() {
            app.act(Action::PlayList { ids, start: 0, shuffle: true });
        }
    });
}

/// Rows `range` of `items`. Double-click plays the whole of `items` from that row.
pub fn track_rows(app: &mut App, ui: &mut Ui, items: &[Item], range: std::ops::Range<usize>, with_artist: bool, salt: &str) {
    let pal = app.palette;
    let cur = app.state.player.track.as_ref().map(|t| t.id.clone());
    for i in range {
        let it = &items[i];
        let qid = queue_id(it);
        let row = TrackRow {
            number: Some(i + 1),
            title: &it.name,
            subtitle: with_artist.then_some(it.subtitle.as_deref()).flatten(),
            duration_ms: it.duration_ms,
            art: None,
            playing: cur.as_deref() == Some(qid.as_str()),
            unavailable: app.guard.unavailable(&qid, it.playable),
            menu: &MENU,
        };
        match widgets::track_row(ui, &pal, Id::new((salt, &it.id, i)), &row) {
            RowEvent::DoubleClick => app.act(Action::PlayList { ids: ids(items), start: i as u32, shuffle: false }),
            RowEvent::Menu(RowMenu::PlayNext) => app.act(Action::QueuePlayNext(qid)),
            RowEvent::Menu(RowMenu::AddToQueue) => app.act(Action::QueueAdd(qid)),
            _ => {}
        }
    }
}

/// Error or loading placeholder for a state with nothing to show. True when it drew something.
pub fn blocked<T>(app: &mut App, ui: &mut Ui, key: &ViewKey, st: &ListState<T>) -> bool {
    let pal = app.palette;
    if st.error.is_some() && st.items.is_empty()
    {
        if banner(ui, &pal, BannerKind::Error, &tr("Couldn't load this page."), Some(&tr("Try again"))) {
            app.act(Action::Retry(key.clone()));
        }
        return true;
    }
    if st.items.is_empty() && st.phase != Phase::Idle {
        loading_rows(ui, &pal, 8);
        return true;
    }
    false
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let pal = app.palette;
    let (head, is_playlist) = match &app.page {
        Page::Album(i) => (i.clone(), false),
        Page::Playlist(i) => (i.clone(), true),
        _ => return,
    };
    let ViewData::Detail { key, rx, tracks } = &app.view else {
        widgets::empty_state(ui, &pal, &tr("Nothing here yet"), "");
        return;
    };
    let key = key.clone();
    let detail = rx.borrow().clone();
    let Some((tkey, trx)) = tracks else { return };
    let tkey = tkey.clone();
    let tl = trx.borrow().clone();
    // The tracks list is the source; a detail error with nothing loaded is a full-view error.
    if detail.items.is_empty() && blocked(app, ui, &key, &detail) {
        return;
    }
    let head = detail.items.first().map_or(head, |d| d.head.clone());
    let items = tl.items.clone();

    egui::ScrollArea::vertical().auto_shrink([false; 2]).show_viewport(ui, |ui, vp| {
        let top = ui.min_rect().top();
        ui.add_space(theme::XXXL - theme::LG);
        ui.horizontal(|ui| {
            let art = app.backend.art(head.artwork.as_ref().map(|a| a.url.as_str()), 400);
            widgets::artwork(ui, &pal, &art, &head.id, theme::HERO_ART, false);
            ui.add_space(theme::MD);
            ui.vertical(|ui| {
                let w = (ui.available_width() - theme::MD).max(100.0);
                let kind = if is_playlist || head.kind == ItemKind::Playlist { "Playlist" } else { "Album" };
                ui.label(RichText::new(tr(kind)).font(theme::label()).color(pal.secondary));
                widgets::ellipsized(ui, &head.name, theme::display(), pal.text, w);
                let n = if tl.has_more { head.track_count.map(|n| n as usize) } else { None }.unwrap_or(items.len());
                let total: u64 = items.iter().filter_map(|i| i.duration_ms).sum();
                let mut parts: Vec<String> = head.subtitle.iter().cloned().collect();
                parts.extend(year(&head).map(str::to_owned));
                if n > 0 {
                    parts.push(plural(n));
                }
                if total > 0 {
                    parts.push(length(total));
                }
                widgets::ellipsized(ui, &parts.join(" · "), theme::label(), pal.secondary, w);
                ui.add_space(theme::MD);
                play_buttons(app, ui, ids(&items));
            });
        });
        ui.add_space(theme::LG);

        if items.is_empty() {
            if tl.error.is_some() {
                if banner(ui, &pal, BannerKind::Error, &tr("Couldn't load the tracks."), Some(&tr("Try again"))) {
                    app.act(Action::Retry(tkey.clone()));
                }
            } else if tl.phase != Phase::Idle {
                loading_rows(ui, &pal, 8);
            } else {
                widgets::empty_state(ui, &pal, &tr("Nothing here yet"), "");
            }
            return;
        }
        // Only rows in the viewport are drawn.
        let off = ui.cursor().top() - top;
        let first = (((vp.min.y - off) / theme::ROW_H).floor().max(0.0) as usize).min(items.len());
        let last = (((vp.max.y - off) / theme::ROW_H).ceil().max(0.0) as usize + 1).min(items.len());
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.add_space(first as f32 * theme::ROW_H);
        track_rows(app, ui, &items, first..last, is_playlist, "dtl");
        ui.add_space((items.len() - last) as f32 * theme::ROW_H);
        if tl.phase == Phase::LoadingMore {
            spinner_row(ui, &pal);
        } else if tl.has_more && tl.phase == Phase::Idle && last + 10 >= items.len() {
            app.act(Action::LoadMore(tkey.clone()));
        }
    });
}

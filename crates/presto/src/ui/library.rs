//! Library tabs, plus the list-state and card helpers shared by the other browse views.
use crate::app::{App, ViewData};
use crate::i18n::tr;
use crate::model::{Action, Page, page_for, queue_id};
use crate::theme;
use crate::ui::widgets::{BannerKind, RowEvent, RowMenu, TrackRow, banner, card, empty_state, loading_rows, spinner_row, track_row};
use egui::{RichText, Ui};
use presto_core::data::error::UiErrorKind;
use presto_core::data::models::{Item, ItemKind};
use presto_core::data::view::{ErrorDisplay, LibKind, ListState, Phase, Sort, ViewKey, sorts};

pub(crate) fn error_copy(kind: &UiErrorKind) -> String {
    match kind {
        UiErrorKind::RateLimited { .. } => "Apple Music is busy. Retrying shortly.".into(),
        UiErrorKind::Offline | UiErrorKind::Timeout | UiErrorKind::Unavailable | UiErrorKind::Upstream { .. } => {
            "Couldn't load this. Check your connection and try again.".into()
        }
        k => k.message(),
    }
}

/// Renders loading, error, offline and empty states. Returns true when the caller should render items.
pub(crate) fn list_states<T>(app: &mut App, ui: &mut Ui, s: &ListState<T>, key: &ViewKey, empty: (&'static str, &'static str)) -> bool {
    let pal = app.palette;
    if s.items.is_empty() && matches!(s.phase, Phase::Loading) {
        loading_rows(ui, &pal, 8);
        return false;
    }
    if let Some(err) = &s.error {
        // The session-expired banner already says it.
        if matches!(err.kind, UiErrorKind::AuthExpired) {
            return false;
        }
        let copy = error_copy(&err.kind);
        let retry = err.kind.retryable().then(|| tr("Try again"));
        match s.error_display() {
            Some(ErrorDisplay::FullView) => {
                empty_state(ui, &pal, &copy, "");
                if retry.is_some() {
                    ui.vertical_centered(|ui| {
                        if ui.button(tr("Try again")).clicked() {
                            app.act(Action::Retry(key.clone()));
                        }
                    });
                }
                return false;
            }
            _ => {
                let kind = if matches!(err.kind, UiErrorKind::RateLimited { .. }) { BannerKind::Warning } else { BannerKind::Error };
                if banner(ui, &pal, kind, &copy, retry.as_deref()) {
                    app.act(Action::Retry(key.clone()));
                }
                ui.add_space(theme::SM);
            }
        }
    }
    if s.from_cache && !app.ready() {
        banner(ui, &pal, BannerKind::Info, &tr("Engine offline. Showing cached data."), None);
        ui.add_space(theme::SM);
    }
    if s.items.is_empty() {
        if s.error.is_none() && s.phase == Phase::Idle {
            empty_state(ui, &pal, &tr(empty.0), &tr(empty.1));
        }
        return false;
    }
    true
}

/// Artwork card for `item`. Returns the click response.
pub(crate) fn item_card(app: &App, ui: &mut Ui, item: &Item) -> egui::Response {
    let art = app.backend.art(item.artwork.as_ref().map(|a| a.url.as_str()), 320);
    card(ui, &app.palette, &art, &item.id, &item.name, item.subtitle.as_deref(), item.kind == ItemKind::Artist)
}

/// Opens the item's page; songs play on their own.
pub(crate) fn open_item(app: &mut App, item: &Item) {
    match page_for(item) {
        Some(p) => app.act(Action::Open(p)),
        None if item.kind == ItemKind::Song => {
            app.act(Action::PlayList { ids: vec![queue_id(item)], start: 0, shuffle: false })
        }
        None => {}
    }
}

fn sort_label(s: Sort) -> &'static str {
    match s {
        Sort::Default => "Default",
        Sort::NameAsc => "Name A-Z",
        Sort::NameDesc => "Name Z-A",
        Sort::AddedNewest => "Recently added",
        Sort::AddedOldest => "Oldest added",
    }
}

/// Song rows from `items`; double-click plays the whole list from the row. Returns the last visible row range end.
pub(crate) fn song_rows(app: &mut App, ui: &mut Ui, items: &[Item], id: &str) -> usize {
    let pal = app.palette;
    let mut last = 0;
    let menu = [RowMenu::PlayNext, RowMenu::AddToQueue];
    egui::ScrollArea::vertical().id_salt(id).auto_shrink(false).show_rows(ui, theme::ROW_H, items.len(), |ui, range| {
        last = range.end;
        for i in range {
            let it = &items[i];
            let art = app.backend.art(it.artwork.as_ref().map(|a| a.url.as_str()), 80);
            let row = TrackRow {
                number: Some(i + 1),
                title: &it.name,
                subtitle: it.subtitle.as_deref(),
                duration_ms: it.duration_ms,
                art: Some((&art, &it.id)),
                playing: false,
                unavailable: app.guard.unavailable(&it.id, it.playable),
                menu: &menu,
            };
            match track_row(ui, &pal, egui::Id::new((id, i)), &row) {
                RowEvent::DoubleClick => app.act(Action::PlayList {
                    ids: items.iter().map(queue_id).collect(),
                    start: i as u32,
                    shuffle: false,
                }),
                RowEvent::Menu(RowMenu::PlayNext) => app.act(Action::QueuePlayNext(queue_id(it))),
                RowEvent::Menu(RowMenu::AddToQueue) => app.act(Action::QueueAdd(queue_id(it))),
                _ => {}
            }
        }
    });
    last
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let pal = app.palette;
    let Page::Library(kind) = app.page else { return };
    let ViewData::List { key, rx } = &app.view else { return };
    let (key, rx) = (key.clone(), rx.clone());
    let title = match kind {
        LibKind::Playlists => "Playlists",
        LibKind::Albums => "Albums",
        LibKind::Artists => "Artists",
        LibKind::Songs => "Songs",
    };
    ui.horizontal(|ui| {
        ui.label(RichText::new(tr(title)).font(theme::heading()).color(pal.text));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let cur = app.lib_sort;
            egui::ComboBox::from_id_salt("lib_sort").selected_text(tr(sort_label(cur))).show_ui(ui, |ui| {
                for s in sorts(kind) {
                    if ui.selectable_label(s == cur, tr(sort_label(s))).clicked() && s != cur {
                        app.act(Action::SetSort(s));
                    }
                }
            });
        });
    });
    ui.add_space(theme::SM);
    let s = rx.borrow();
    if !list_states(app, ui, &s, &key, ("Nothing here yet", "Songs you add to your Apple Music library will appear here.")) {
        return;
    }
    let paged = s.has_more && s.phase == Phase::Idle;
    let last = if kind == LibKind::Songs {
        let last = song_rows(app, ui, &s.items, "lib_songs");
        if s.phase == Phase::LoadingMore {
            spinner_row(ui, &pal);
        }
        last
    } else {
        let mut last = 0;
        egui::ScrollArea::vertical().id_salt(title).auto_shrink(false).show(ui, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Min).with_main_wrap(true), |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(theme::MD, theme::MD);
                for (i, it) in s.items.iter().enumerate() {
                    let r = item_card(app, ui, it);
                    if r.clicked() {
                        open_item(app, it);
                    }
                    if ui.is_rect_visible(r.rect) {
                        last = i;
                    }
                }
            });
            if s.phase == Phase::LoadingMore {
                spinner_row(ui, &pal);
            }
        });
        // cards fill rows of unknown width: treat the last painted card as the scroll position
        last + 1
    };
    if paged && last + 20 >= s.items.len() {
        app.act(Action::LoadMore(key));
    }
}

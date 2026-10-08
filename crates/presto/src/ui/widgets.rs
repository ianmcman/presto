//! Shared widgets. Plain data in, events out; no app state.

use std::path::PathBuf;
use std::time::Duration;

use egui::{Color32, FontId, Id, Margin, Response, Sense, Shape, Ui, vec2};

use crate::i18n::tr;
use crate::theme::{self, Icon, Palette};

pub enum Art {
    Ready(PathBuf),
    Pending,
    Placeholder,
}

const PLACEHOLDERS: [(u8, u8, u8); 8] = [
    (0x3A, 0x3A, 0x4C),
    (0x4C, 0x3A, 0x3A),
    (0x3A, 0x4C, 0x40),
    (0x4C, 0x46, 0x3A),
    (0x3A, 0x46, 0x4C),
    (0x46, 0x3A, 0x4C),
    (0x3A, 0x4C, 0x4C),
    (0x4C, 0x3A, 0x46),
];

/// FNV-1a of `seed` picks one of eight muted colours.
pub fn placeholder_color(seed: &str) -> Color32 {
    let mut h: u32 = 0x811c_9dc5;
    for b in seed.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    let (r, g, b) = PLACEHOLDERS[(h % 8) as usize];
    Color32::from_rgb(r, g, b)
}

pub fn artwork(ui: &mut Ui, pal: &Palette, art: &Art, seed: &str, size: f32, round: bool) -> Response {
    let radius = if round { (size / 2.0) as u8 } else { 4 };
    if let Art::Ready(p) = art {
        let img = egui::Image::new(format!("file://{}", p.display()))
            .fit_to_exact_size(vec2(size, size))
            .corner_radius(radius);
        return ui.add(img);
    }
    if matches!(art, Art::Pending) {
        ui.ctx().request_repaint_after(Duration::from_millis(100));
    }
    let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(rect, radius, placeholder_color(seed));
        theme::paint_icon(ui, Icon::Music, rect, size * 0.4, pal.secondary);
    }
    resp
}

pub fn fmt_duration(ms: u64) -> String {
    let s = ms / 1000;
    format!("{}:{:02}", s / 60, s % 60)
}

/// `text` cut to fit `max_w`, ending in "…" when cut.
fn fit(ui: &Ui, text: &str, font: &FontId, max_w: f32) -> (String, bool) {
    let width = |s: &str| ui.painter().layout_no_wrap(s.to_owned(), font.clone(), Color32::WHITE).size().x;
    if width(text) <= max_w {
        return (text.to_owned(), false);
    }
    let chars: Vec<char> = text.chars().collect();
    let (mut lo, mut hi) = (0, chars.len());
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let s: String = chars[..mid].iter().collect::<String>() + "…";
        if width(&s) <= max_w { lo = mid } else { hi = mid - 1 }
    }
    (chars[..lo].iter().collect::<String>() + "…", true)
}

pub fn ellipsized(ui: &mut Ui, text: &str, font: FontId, color: Color32, max_w: f32) -> Response {
    let (s, cut) = fit(ui, text, &font, max_w);
    let resp = ui.add(egui::Label::new(egui::RichText::new(s).font(font).color(color)).wrap_mode(egui::TextWrapMode::Extend));
    if cut { resp.on_hover_text(text) } else { resp }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowMenu {
    PlayFromHere,
    PlayNext,
    AddToQueue,
    Remove,
}

impl RowMenu {
    fn label(self) -> String {
        match self {
            RowMenu::PlayFromHere => tr("Play from here"),
            RowMenu::PlayNext => tr("Play next"),
            RowMenu::AddToQueue => tr("Add to queue"),
            RowMenu::Remove => tr("Remove from queue"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum RowEvent {
    None,
    Click,
    DoubleClick,
    Menu(RowMenu),
}

pub struct TrackRow<'a> {
    pub number: Option<usize>,
    pub title: &'a str,
    pub subtitle: Option<&'a str>,
    pub duration_ms: Option<u64>,
    pub art: Option<(&'a Art, &'a str)>,
    pub playing: bool,
    pub unavailable: bool,
    pub menu: &'a [RowMenu],
}

fn menu_items(ui: &mut Ui, items: &[RowMenu], ev: &mut RowEvent) {
    for m in items {
        if ui.button(m.label()).clicked() {
            *ev = RowEvent::Menu(*m);
            ui.close();
        }
    }
}

pub fn track_row(ui: &mut Ui, pal: &Palette, id: Id, row: &TrackRow) -> RowEvent {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, theme::ROW_H), Sense::click());
    let mut ev = RowEvent::None;
    let hovered = resp.hovered();
    if row.unavailable {
        if resp.clicked() {
            ui.data_mut(|d| {
                let v = d.get_temp_mut_or_default::<bool>(id);
                *v = !*v;
            });
        }
        if resp.clicked() || resp.double_clicked() {
            ev = RowEvent::Click;
        }
    } else if resp.double_clicked() {
        ev = RowEvent::DoubleClick;
    } else if resp.clicked() {
        ev = RowEvent::Click;
    }
    let a = if row.unavailable { 0.4 } else { 1.0 };
    let dim = |c: Color32| c.gamma_multiply(a);

    let p = ui.painter().clone();
    if hovered {
        p.rect_filled(rect, 4, pal.surface_hover);
    }
    let mut x = rect.left() + theme::SM;
    let cy = rect.center().y;
    if row.number.is_some() || row.playing {
        let nrect = egui::Rect::from_center_size(egui::pos2(x + 12.0, cy), vec2(24.0, 24.0));
        if row.playing || (hovered && !row.unavailable) {
            let color = if row.playing { pal.accent } else { pal.text };
            let ic = if row.playing { Icon::PauseFilled } else { Icon::PlayFilled };
            theme::paint_icon(ui, ic, nrect, 14.0, dim(color));
        } else if let Some(n) = row.number {
            p.text(nrect.center(), egui::Align2::CENTER_CENTER, n.to_string(), theme::mono(), dim(pal.secondary));
        }
        x += 32.0;
    }
    if let Some((art, seed)) = row.art {
        let r = egui::Rect::from_min_size(egui::pos2(x, cy - theme::ROW_ART / 2.0), vec2(theme::ROW_ART, theme::ROW_ART));
        // new_child, not scope_builder: the latter moves the parent cursor and shortens the row.
        let mut c = ui.new_child(egui::UiBuilder::new().max_rect(r));
        c.multiply_opacity(a);
        artwork(&mut c, pal, art, seed, theme::ROW_ART, false);
        x += theme::ROW_ART + theme::SM;
    }
    let mut right = rect.right() - theme::SM;
    let menu_rect = egui::Rect::from_center_size(egui::pos2(right - theme::HIT / 2.0, cy), vec2(theme::HIT, theme::HIT));
    let has_menu = !row.menu.is_empty();
    if has_menu {
        right -= theme::HIT;
    }
    if let Some(ms) = row.duration_ms {
        let g = p.layout_no_wrap(fmt_duration(ms), theme::mono(), dim(pal.secondary));
        let wd = g.size().x;
        p.galley(egui::pos2(right - wd, cy - g.size().y / 2.0), g, Color32::WHITE);
        right -= wd + theme::SM;
    }
    let mut badge_w = 0.0;
    if row.unavailable {
        let label = tr("Unavailable");
        badge_w = p.layout_no_wrap(label, theme::label_strong(), pal.warning).size().x + 2.0 * theme::SM + theme::SM;
    }
    let text_w = (right - x - badge_w).max(0.0);
    let title_color = if row.playing { pal.accent } else { pal.text };
    let (title, _) = fit(ui, row.title, &theme::body(), text_w);
    let tg = p.layout_no_wrap(title, theme::body(), dim(title_color));
    let sg = row.subtitle.map(|s| {
        let (s, _) = fit(ui, s, &theme::label(), text_w + badge_w);
        p.layout_no_wrap(s, theme::label(), dim(pal.secondary))
    });
    let total_h = tg.size().y + sg.as_ref().map_or(0.0, |g| g.size().y);
    let top = cy - total_h / 2.0;
    let title_w = tg.size().x;
    let title_h = tg.size().y;
    p.galley(egui::pos2(x, top), tg, Color32::WHITE);
    if let Some(g) = sg {
        p.galley(egui::pos2(x, top + title_h), g, Color32::WHITE);
    }
    if row.unavailable {
        let br = egui::Rect::from_min_size(egui::pos2(x + title_w + theme::SM, cy - 10.0), vec2(badge_w - theme::SM, 20.0));
        let mut c = ui.new_child(egui::UiBuilder::new().max_rect(br));
        badge(&mut c, pal, &tr("Unavailable")).on_hover_text(tr(REASON));
    }
    if has_menu && (hovered || ui.rect_contains_pointer(menu_rect)) {
        let b = ui.interact(menu_rect, id.with("menu"), Sense::click());
        theme::paint_icon(ui, Icon::Ellipsis, menu_rect, 16.0, if b.hovered() { pal.text } else { pal.secondary });
        egui::Popup::menu(&b).show(|ui| menu_items(ui, row.menu, &mut ev));
    }
    if has_menu {
        resp.context_menu(|ui| menu_items(ui, row.menu, &mut ev));
    }
    if row.unavailable && ui.data(|d| d.get_temp::<bool>(id)).unwrap_or(false) {
        ui.horizontal(|ui| {
            ui.add_space(x - rect.left());
            ui.label(egui::RichText::new(tr(REASON)).font(theme::label()).color(pal.secondary));
        });
    }
    ev
}

const REASON: &str = "This track can't be played right now. It may not be available in your region or on your plan.";

pub fn card(ui: &mut Ui, pal: &Palette, art: &Art, seed: &str, title: &str, subtitle: Option<&str>, round: bool) -> Response {
    let inner = ui.allocate_ui_with_layout(
        vec2(theme::CARD_ART, 0.0),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            artwork(ui, pal, art, seed, theme::CARD_ART, round);
            ui.add_space(theme::XS);
            ellipsized(ui, title, theme::body(), pal.text, theme::CARD_ART);
            if let Some(s) = subtitle {
                ellipsized(ui, s, theme::label(), pal.secondary, theme::CARD_ART);
            }
        },
    );
    let resp = ui.interact(inner.response.rect, inner.response.id.with("card"), Sense::click());
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

pub fn shelf(ui: &mut Ui, pal: &Palette, title: &str, see_all: bool, add: impl FnOnce(&mut Ui)) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(title).font(theme::heading()).color(pal.text));
        if see_all {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                clicked = ui
                    .add(egui::Button::new(egui::RichText::new(tr("See all")).font(theme::body()).color(pal.accent)).frame(false))
                    .clicked();
            });
        }
    });
    ui.add_space(theme::SM);
    egui::ScrollArea::horizontal().id_salt(title).show(ui, |ui| {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = theme::MD;
            add(ui);
        });
    });
    clicked
}

pub enum BannerKind {
    Error,
    Warning,
    Info,
}

pub fn banner(ui: &mut Ui, pal: &Palette, kind: BannerKind, text: &str, action: Option<&str>) -> bool {
    let mut clicked = false;
    let r = egui::Frame::new().fill(pal.surface).inner_margin(Margin::same(theme::MD as i8)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(text).font(theme::body()).color(pal.text));
            if let Some(a) = action {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    clicked = ui.button(a).clicked();
                });
            }
        });
    });
    let color = match kind {
        BannerKind::Error => Some(pal.danger),
        BannerKind::Warning => Some(pal.warning),
        BannerKind::Info => None,
    };
    if let Some(c) = color {
        let rc = r.response.rect;
        ui.painter().rect_filled(egui::Rect::from_min_size(rc.min, vec2(4.0, rc.height())), 0, c);
    }
    clicked
}

pub fn empty_state(ui: &mut Ui, pal: &Palette, title: &str, body: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(theme::XXL);
        ui.label(egui::RichText::new(title).font(theme::heading()).color(pal.text));
        ui.add_space(theme::XS);
        ui.label(egui::RichText::new(body).font(theme::body()).color(pal.secondary));
    });
}

pub fn loading_rows(ui: &mut Ui, pal: &Palette, n: usize) {
    for _ in 0..n {
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), theme::ROW_H), Sense::hover());
        let p = ui.painter();
        let art = egui::Rect::from_min_size(rect.min + vec2(theme::SM, 4.0), vec2(theme::ROW_ART, theme::ROW_ART));
        p.rect_filled(art, 4, pal.surface_hover);
        let x = art.right() + theme::SM;
        p.rect_filled(egui::Rect::from_min_size(egui::pos2(x, rect.top() + 12.0), vec2(160.0, 10.0)), 4, pal.surface_hover);
        p.rect_filled(egui::Rect::from_min_size(egui::pos2(x, rect.top() + 28.0), vec2(100.0, 8.0)), 4, pal.surface_hover);
    }
}

pub fn spinner_row(ui: &mut Ui, _pal: &Palette) {
    ui.vertical_centered(|ui| {
        ui.add_space(theme::MD);
        ui.add(egui::Spinner::new());
    });
}

#[derive(Debug, PartialEq)]
pub enum SliderEvent {
    None,
    Drag(f32),
    Release(f32),
}

const WHEEL_STEP: f32 = 0.05;

pub fn thin_slider(ui: &mut Ui, pal: &Palette, id: Id, value: f32, width: f32, label: &str) -> SliderEvent {
    let (_, rect) = ui.allocate_space(vec2(width, 16.0));
    let resp = ui.interact(rect, id, Sense::click_and_drag());
    resp.widget_info(|| egui::WidgetInfo::slider(ui.is_enabled(), value as f64, label));
    let pointer = resp.interact_pointer_pos().map(|p| ((p.x - rect.left()) / rect.width()).clamp(0.0, 1.0));
    let mut ev = SliderEvent::None;
    if (resp.is_pointer_button_down_on() || resp.dragged()) && let Some(v) = pointer {
        ui.data_mut(|d| d.insert_temp(id, v));
        ev = SliderEvent::Drag(v);
    }
    if resp.drag_stopped() {
        let v = ui.data_mut(|d| d.remove_temp::<f32>(id)).or(pointer).unwrap_or(value);
        ev = SliderEvent::Release(v);
    } else if resp.clicked() && let Some(v) = pointer {
        ui.data_mut(|d| d.remove::<f32>(id));
        ev = SliderEvent::Release(v);
    }
    let mut step = 0.0;
    if resp.hovered() {
        // smooth delta is spread over frames; ~50 px per notch
        step += ui.input(|i| i.smooth_scroll_delta.y) / 50.0 * WHEEL_STEP;
    }
    if resp.has_focus() {
        step += ui.input(|i| i.num_presses(egui::Key::ArrowRight) as f32 - i.num_presses(egui::Key::ArrowLeft) as f32) * 0.01;
    }
    if step != 0.0 {
        ev = SliderEvent::Release((value + step).clamp(0.0, 1.0));
    }
    let shown = ui.data(|d| d.get_temp::<f32>(id)).unwrap_or(value).clamp(0.0, 1.0);
    let h = if resp.hovered() || resp.dragged() { theme::SEEK_HOVER_H } else { theme::SEEK_H };
    let track = egui::Rect::from_center_size(rect.center(), vec2(rect.width(), h));
    let p = ui.painter();
    p.rect_filled(track, h / 2.0, pal.outline);
    let mut fill = track;
    fill.set_width(track.width() * shown);
    p.rect_filled(fill, h / 2.0, pal.accent);
    theme::focus_ring(ui, &resp);
    ev
}

pub fn icon_button(ui: &mut Ui, pal: &Palette, icon: Icon, size: f32, active: bool, label: &str) -> Response {
    let hit = (size + 12.0).max(theme::HIT);
    let (rect, resp) = ui.allocate_exact_size(vec2(hit, hit), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label));
    let color = if active {
        pal.accent
    } else if resp.hovered() || resp.has_focus() {
        pal.text
    } else {
        pal.secondary
    };
    theme::paint_icon(ui, icon, rect, size, color);
    theme::focus_ring(ui, &resp);
    resp.on_hover_text(label)
}

pub fn demo_chip(ui: &mut Ui, pal: &Palette) -> Response {
    egui::Frame::new()
        .stroke(egui::Stroke::new(1.0, pal.warning))
        .corner_radius(8)
        .inner_margin(Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(tr("DEMO")).font(theme::label_strong()).color(pal.warning));
        })
        .response
        .on_hover_text(tr("Running against the mock engine. No account needed."))
}

pub fn badge(ui: &mut Ui, pal: &Palette, text: &str) -> Response {
    egui::Frame::new()
        .fill(Color32::from_rgb(0x2C, 0x2C, 0x2E))
        .corner_radius(4)
        .inner_margin(Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).font(theme::label_strong()).color(pal.warning));
        })
        .response
}

fn collect(shape: &Shape, out: &mut Vec<String>) {
    match shape {
        Shape::Text(t) => out.push(t.galley.text().to_owned()),
        Shape::Vec(v) => v.iter().for_each(|s| collect(s, out)),
        _ => {}
    }
}

/// Every rendered text string in `out`.
pub fn texts(out: &egui::FullOutput) -> Vec<String> {
    let mut v = Vec::new();
    out.shapes.iter().for_each(|c| collect(&c.shape, &mut v));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> egui::Context {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        theme::apply(&ctx, &Palette::dark());
        run(&ctx, vec![], |_| {}); // fonts apply on the next frame
        ctx
    }

    fn row<'a>(title: &'a str, unavailable: bool) -> TrackRow<'a> {
        TrackRow {
            number: Some(1),
            title,
            subtitle: Some("Artist"),
            duration_ms: Some(61_000),
            art: None,
            playing: false,
            unavailable,
            menu: &[RowMenu::PlayNext],
        }
    }

    fn run(ctx: &egui::Context, events: Vec<egui::Event>, f: impl FnMut(&mut Ui)) -> egui::FullOutput {
        let mut raw = egui::RawInput::default();
        raw.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, vec2(400.0, 400.0)));
        raw.events = events;
        let mut out = ctx.run_ui(raw, f);
        out.textures_delta.clear();
        out
    }

    fn click_at(ctx: &egui::Context, pos: egui::Pos2, mut f: impl FnMut(&mut Ui) -> RowEvent) -> (Vec<String>, Vec<RowEvent>) {
        let btn = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        let mut all = Vec::new();
        let mut evs = Vec::new();
        for events in [vec![egui::Event::PointerMoved(pos)], vec![btn(true)], vec![btn(false)], vec![]] {
            let out = run(ctx, events, |ui| evs.push(f(ui)));
            all.extend(texts(&out));
        }
        (all, evs)
    }

    #[test]
    fn unavailable_row_reveals_reason_on_click() {
        let ctx = ctx();
        let pal = Palette::dark();
        let out = run(&ctx, vec![], |ui| {
            track_row(ui, &pal, Id::new("r"), &row("Song", true));
        });
        assert!(texts(&out).iter().any(|t| t == "Unavailable"));
        assert!(!texts(&out).iter().any(|t| t.contains("can't be played")));
        let (all, evs) = click_at(&ctx, egui::pos2(100.0, 24.0), |ui| track_row(ui, &pal, Id::new("r"), &row("Song", true)));
        assert!(all.iter().any(|t| t.contains("This track can't be played right now. It may not be available in your region or on your plan.")));
        assert!(evs.contains(&RowEvent::Click));
    }

    #[test]
    fn unavailable_double_click_is_click() {
        let ctx = ctx();
        let pal = Palette::dark();
        let pos = egui::pos2(100.0, 24.0);
        let btn = |pressed| egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
        let mut evs = Vec::new();
        for events in [vec![egui::Event::PointerMoved(pos)], vec![btn(true)], vec![btn(false)], vec![btn(true)], vec![btn(false)]] {
            run(&ctx, events, |ui| evs.push(track_row(ui, &pal, Id::new("r"), &row("Song", true))));
        }
        assert!(!evs.contains(&RowEvent::DoubleClick));
    }

    #[test]
    fn available_double_click() {
        let ctx = ctx();
        let pal = Palette::dark();
        let pos = egui::pos2(100.0, 24.0);
        let btn = |pressed| egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
        let mut evs = Vec::new();
        for events in [vec![egui::Event::PointerMoved(pos)], vec![btn(true)], vec![btn(false)], vec![btn(true)], vec![btn(false)]] {
            run(&ctx, events, |ui| evs.push(track_row(ui, &pal, Id::new("r"), &row("Song", false))));
        }
        assert!(evs.contains(&RowEvent::DoubleClick));
    }

    #[test]
    fn long_title_truncates() {
        let ctx = ctx();
        let pal = Palette::dark();
        let long = "x".repeat(120);
        let out = run(&ctx, vec![], |ui| {
            ui.set_max_width(300.0);
            track_row(ui, &pal, Id::new("r"), &row(&long, false));
        });
        assert!(texts(&out).iter().any(|t| t.ends_with('…') && t.starts_with('x')));
    }

    #[test]
    fn placeholder_stable() {
        assert_eq!(placeholder_color("al1"), placeholder_color("al1"));
        let set: std::collections::HashSet<_> = ["al1", "al2", "al3", "p1"].iter().map(|s| placeholder_color(s)).collect();
        assert!(set.len() >= 2);
    }

    #[test]
    fn chip_and_empty_state() {
        let ctx = ctx();
        let pal = Palette::dark();
        let out = run(&ctx, vec![], |ui| {
            demo_chip(ui, &pal);
            empty_state(ui, &pal, "Nothing here", "Add something.");
        });
        let t = texts(&out);
        assert!(t.iter().any(|s| s == "DEMO"));
        assert!(t.iter().any(|s| s == "Nothing here"));
        assert!(t.iter().any(|s| s == "Add something."));
    }

    #[test]
    fn misc_widgets_render() {
        let ctx = ctx();
        let pal = Palette::dark();
        let out = run(&ctx, vec![], |ui| {
            shelf(ui, &pal, "Shelf", true, |_| {});
            card(ui, &pal, &Art::Placeholder, "al1", "Album", Some("Artist"), false);
            banner(ui, &pal, BannerKind::Error, "Oops", Some("Try again"));
            loading_rows(ui, &pal, 2);
            thin_slider(ui, &pal, Id::new("s"), 0.5, 200.0, "Seek");
            icon_button(ui, &pal, Icon::Play, 16.0, false, "Play");
        });
        let t = texts(&out);
        assert!(t.iter().any(|s| s == "Album") && t.iter().any(|s| s == "Try again") && t.iter().any(|s| s == "See all"));
    }
}

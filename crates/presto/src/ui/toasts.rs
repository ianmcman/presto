//! Toast notifications.

use std::time::{Duration, Instant};

use egui::{Align2, Margin, Sense, vec2};

use crate::i18n::tr;
use crate::theme::{self, Icon, Palette};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Toast {
    pub text: String,
    pub kind: ToastKind,
}

impl Toast {
    pub fn info(t: impl Into<String>) -> Self {
        Self { text: t.into(), kind: ToastKind::Info }
    }
    pub fn warning(t: impl Into<String>) -> Self {
        Self { text: t.into(), kind: ToastKind::Warning }
    }
    pub fn error(t: impl Into<String>) -> Self {
        Self { text: t.into(), kind: ToastKind::Error }
    }
}

#[derive(Default)]
pub struct Toasts {
    items: Vec<(Toast, Duration)>,
    last_tick: Option<Instant>,
    hovered: Option<usize>,
}

impl Toasts {
    pub const MAX: usize = 3;
    pub const LIFETIME: Duration = Duration::from_secs(5);

    pub fn push(&mut self, t: Toast) {
        self.items.push((t, Self::LIFETIME));
        if self.items.len() > Self::MAX {
            self.items.remove(0);
        }
    }

    /// Ages toasts by the time since the last tick; the hovered one does not age.
    pub fn tick(&mut self, now: Instant, hovered: Option<usize>) {
        let dt = self.last_tick.map_or(Duration::ZERO, |t| now.saturating_duration_since(t));
        self.last_tick = Some(now);
        for (i, (_, left)) in self.items.iter_mut().enumerate() {
            if hovered != Some(i) {
                *left = left.saturating_sub(dt);
            }
        }
        self.items.retain(|(_, left)| !left.is_zero());
    }

    pub fn texts(&self) -> Vec<&str> {
        self.items.iter().map(|(t, _)| t.text.as_str()).collect()
    }

    pub fn show(&mut self, ctx: &egui::Context, pal: &Palette, now: Instant) {
        let hovered = self.hovered.take();
        self.tick(now, hovered);
        if self.items.is_empty() {
            return;
        }
        let mut close = None;
        let mut hover = None;
        egui::Area::new(egui::Id::new("presto-toasts"))
            .anchor(Align2::RIGHT_BOTTOM, vec2(-theme::MD, -(theme::PLAYER_H + theme::MD)))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.set_width(theme::TOAST_W);
                ui.spacing_mut().item_spacing.y = theme::SM;
                for (i, (t, _)) in self.items.iter().enumerate() {
                    let r = egui::Frame::new()
                        .fill(pal.surface_hover)
                        .corner_radius(8)
                        .inner_margin(Margin::symmetric(theme::MD as i8, theme::SM as i8))
                        .show(ui, |ui| {
                            ui.set_width(theme::TOAST_W - 2.0 * theme::MD);
                            ui.horizontal(|ui| {
                                ui.add_sized(
                                    vec2(theme::TOAST_W - 2.0 * theme::MD - theme::HIT, 0.0),
                                    egui::Label::new(egui::RichText::new(&t.text).font(theme::body()).color(pal.text))
                                        .wrap(),
                                );
                                let (rect, resp) = ui.allocate_exact_size(vec2(theme::HIT, theme::HIT), Sense::click());
                                resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, tr("Dismiss")));
                                theme::paint_icon(ui, Icon::X, rect, 16.0, if resp.hovered() { pal.text } else { pal.secondary });
                                theme::focus_ring(ui, &resp);
                                if resp.clicked() {
                                    close = Some(i);
                                }
                            });
                        });
                    let rc = r.response.rect;
                    let bar = match t.kind {
                        ToastKind::Error => Some(pal.danger),
                        ToastKind::Warning => Some(pal.warning),
                        ToastKind::Info => None,
                    };
                    if let Some(c) = bar {
                        ui.painter().rect_filled(egui::Rect::from_min_size(rc.min, vec2(4.0, rc.height())), 0, c);
                    }
                    if ui.rect_contains_pointer(rc) {
                        hover = Some(i);
                    }
                }
            });
        if let Some(i) = close {
            self.items.remove(i);
            hover = None;
        }
        self.hovered = hover;
        if !self.items.is_empty() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::widgets::texts;

    #[test]
    fn caps_at_three_keeping_newest() {
        let mut t = Toasts::default();
        for i in 0..4 {
            t.push(Toast::info(format!("t{i}")));
        }
        assert_eq!(t.texts(), ["t1", "t2", "t3"]);
    }

    #[test]
    fn expires_after_five_seconds() {
        let t0 = Instant::now();
        let mut t = Toasts::default();
        t.tick(t0, None);
        t.push(Toast::error("x"));
        t.tick(t0 + Duration::from_millis(4900), None);
        assert_eq!(t.texts().len(), 1);
        t.tick(t0 + Duration::from_millis(5100), None);
        assert!(t.texts().is_empty());
    }

    #[test]
    fn hover_pauses() {
        let t0 = Instant::now();
        let mut t = Toasts::default();
        t.tick(t0, None);
        t.push(Toast::warning("x"));
        t.tick(t0 + Duration::from_secs(4), None);
        t.tick(t0 + Duration::from_secs(20), Some(0));
        assert_eq!(t.texts().len(), 1);
        t.tick(t0 + Duration::from_millis(20_500), None);
        assert_eq!(t.texts().len(), 1);
        t.tick(t0 + Duration::from_millis(21_200), None);
        assert!(t.texts().is_empty());
    }

    #[test]
    fn renders_and_closes() {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let pal = Palette::dark();
        let mut t = Toasts::default();
        t.push(Toast::info("hello"));
        let now = Instant::now();
        let frame = |t: &mut Toasts, events: Vec<egui::Event>| {
            let mut raw = egui::RawInput::default();
            raw.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, vec2(800.0, 600.0)));
            raw.events = events;
            let mut out = ctx.run_ui(raw, |ui| t.show(ui.ctx(), &pal, now));
            out.textures_delta.clear();
            out
        };
        frame(&mut t, vec![]);
        assert!(texts(&frame(&mut t, vec![])).iter().any(|s| s == "hello"));
        // close button: right edge of the toast, above the player bar
        let pos = egui::pos2(800.0 - theme::MD - theme::MD - theme::HIT / 2.0, 600.0 - theme::PLAYER_H - theme::MD - 24.0);
        let b = |pressed| egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
        frame(&mut t, vec![egui::Event::PointerMoved(pos)]);
        frame(&mut t, vec![b(true)]);
        frame(&mut t, vec![b(false)]);
        assert!(t.texts().is_empty());
    }
}

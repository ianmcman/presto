//! Palette, typography, icons and style for the Apple Music look.

use egui::{Color32, CornerRadius, Response, Sense, Stroke, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Palette {
    pub dark: bool,
    pub window: Color32,
    pub panel: Color32,
    pub surface: Color32,
    pub surface_hover: Color32,
    pub surface_active: Color32,
    pub outline: Color32,
    pub text: Color32,
    pub secondary: Color32,
    pub dim: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub on_accent: Color32,
    pub danger: Color32,
    pub warning: Color32,
    pub overlay: Color32,
    pub shadow: Color32,
}

impl Palette {
    pub fn dark() -> Self {
        let c = |r, g, b| Color32::from_rgb(r, g, b);
        Self {
            dark: true,
            window: c(0x00, 0x00, 0x00),
            panel: c(0x1C, 0x1C, 0x1E),
            surface: c(0x1C, 0x1C, 0x1E),
            surface_hover: c(0x2C, 0x2C, 0x2E),
            surface_active: c(0x3A, 0x3A, 0x3C),
            outline: c(0x38, 0x38, 0x3A),
            text: c(0xFF, 0xFF, 0xFF),
            secondary: c(0x98, 0x98, 0x9F),
            dim: c(0x63, 0x63, 0x66),
            accent: c(0xFA, 0x24, 0x3C),
            accent_hover: c(0xFF, 0x37, 0x5F),
            on_accent: c(0xFF, 0xFF, 0xFF),
            danger: c(0xFF, 0x45, 0x3A),
            warning: c(0xFF, 0x9F, 0x0A),
            overlay: c(0x2C, 0x2C, 0x2E),
            shadow: Color32::from_black_alpha(140),
        }
    }
}

impl fastframe_theme::Palette for Palette {
    fn base(_base: fastframe_theme::Base) -> Self {
        // No light theme in this phase.
        Self::dark()
    }

    fn set(&mut self, name: &str, color: Color32) -> bool {
        match name {
            "window" => self.window = color,
            "panel" => self.panel = color,
            "surface" => self.surface = color,
            "surface_hover" => self.surface_hover = color,
            "surface_active" => self.surface_active = color,
            "outline" => self.outline = color,
            "text" => self.text = color,
            "secondary" => self.secondary = color,
            "dim" => self.dim = color,
            "accent" => self.accent = color,
            "accent_hover" => self.accent_hover = color,
            "on_accent" => self.on_accent = color,
            "danger" => self.danger = color,
            "warning" => self.warning = color,
            "overlay" => self.overlay = color,
            "shadow" => self.shadow = color,
            _ => return false,
        }
        true
    }
}

pub const XS: f32 = 4.0;
pub const SM: f32 = 8.0;
pub const MD: f32 = 16.0;
pub const LG: f32 = 24.0;
pub const XL: f32 = 32.0;
pub const XXL: f32 = 48.0;
pub const XXXL: f32 = 64.0;
pub const SIDEBAR_W: f32 = 224.0;
pub const QUEUE_W: f32 = 320.0;
pub const QUEUE_MIN_W: f32 = 280.0;
pub const PLAYER_H: f32 = 88.0;
pub const ROW_H: f32 = 48.0;
pub const ROW_ART: f32 = 40.0;
pub const CARD_ART: f32 = 160.0;
pub const HERO_ART: f32 = 200.0;
pub const BAR_ART: f32 = 56.0;
pub const HIT: f32 = 32.0;
pub const PLAY_HIT: f32 = 40.0;
pub const SEEK_H: f32 = 4.0;
pub const SEEK_HOVER_H: f32 = 8.0;
pub const TOAST_W: f32 = 320.0;

const RADIUS: u8 = 8;
const RADIUS_SMALL: u8 = 4;

use fastframe_fonts::Weight::{Regular, SemiBold};

pub fn body() -> egui::FontId {
    Regular.font_id(14.0)
}
pub fn body_strong() -> egui::FontId {
    SemiBold.font_id(14.0)
}
pub fn label() -> egui::FontId {
    Regular.font_id(12.0)
}
/// DEMO chip only.
pub fn label_strong() -> egui::FontId {
    SemiBold.font_id(12.0)
}
pub fn heading() -> egui::FontId {
    SemiBold.font_id(20.0)
}
pub fn display() -> egui::FontId {
    SemiBold.font_id(32.0)
}
/// Time readouts.
pub fn mono() -> egui::FontId {
    egui::FontId::monospace(12.0)
}

/// Install fonts, image loaders and icons once.
pub fn install(ctx: &egui::Context) {
    ctx.set_fonts(fastframe_fonts::FontSetup::default().definitions());
    egui_extras::install_image_loaders(ctx);
    fastframe_icons::install::<Icon>(ctx);
}

pub fn apply(ctx: &egui::Context, palette: &Palette) {
    let mut style = (*ctx.global_style()).clone();
    apply_to_style(&mut style, palette);
    ctx.set_global_style(style);
}

fn apply_to_style(style: &mut egui::Style, p: &Palette) {
    let v = &mut style.visuals;
    *v = egui::Visuals::dark();
    v.panel_fill = p.panel;
    v.window_fill = p.overlay;
    v.extreme_bg_color = p.surface;
    v.faint_bg_color = p.surface;
    v.code_bg_color = p.surface;
    v.override_text_color = Some(p.text);
    v.weak_text_color = Some(p.secondary);
    v.hyperlink_color = p.text;
    v.selection.bg_fill = p.accent.gamma_multiply(0.35);
    v.selection.stroke = Stroke::new(2.0, p.accent);
    v.window_stroke = Stroke::new(1.0, p.outline);
    v.window_corner_radius = CornerRadius::same(RADIUS + 2);
    v.menu_corner_radius = CornerRadius::same(RADIUS);
    let shadow = |y, blur| egui::epaint::Shadow { offset: [0, y], blur, spread: 0, color: p.shadow };
    v.window_shadow = shadow(6, 24);
    v.popup_shadow = shadow(4, 16);
    let corner = CornerRadius::same(RADIUS_SMALL + 2);
    for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
        w.corner_radius = corner;
        w.bg_stroke = Stroke::NONE;
        w.fg_stroke = Stroke::new(1.0, p.text);
        w.expansion = 0.0;
    }
    v.widgets.noninteractive.corner_radius = corner;
    v.widgets.noninteractive.bg_fill = p.panel;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.outline);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, p.text);
    v.widgets.inactive.bg_fill = p.surface;
    v.widgets.inactive.weak_bg_fill = p.surface;
    v.widgets.hovered.bg_fill = p.surface_hover;
    v.widgets.hovered.weak_bg_fill = p.surface_hover;
    v.widgets.active.bg_fill = p.surface_active;
    v.widgets.active.weak_bg_fill = p.surface_active;
    v.widgets.open.bg_fill = p.surface_hover;
    v.widgets.open.weak_bg_fill = p.surface_hover;
    v.text_cursor.stroke = Stroke::new(2.0, p.accent);
    v.striped = false;
    v.slider_trailing_fill = true;
    v.handle_shape = egui::style::HandleShape::Circle;

    use egui::FontFamily::{Monospace, Proportional};
    use egui::{FontId, TextStyle};
    style.text_styles = [
        (TextStyle::Small, FontId::new(12.0, Proportional)),
        (TextStyle::Body, FontId::new(14.0, Proportional)),
        (TextStyle::Button, FontId::new(14.0, Proportional)),
        (TextStyle::Heading, FontId::new(20.0, Proportional)),
        (TextStyle::Monospace, FontId::new(12.0, Monospace)),
    ]
    .into();
    style.spacing.item_spacing = Vec2::new(SM, 6.0);
    style.spacing.button_padding = Vec2::new(12.0, 6.0);
    style.spacing.interact_size = Vec2::new(PLAY_HIT, HIT);
    style.spacing.scroll = egui::style::ScrollStyle {
        bar_width: 8.0,
        floating_width: 6.0,
        floating_allocated_width: 0.0,
        handle_min_length: 28.0,
        bar_inner_margin: 3.0,
        bar_outer_margin: 2.0,
        dormant_background_opacity: 0.0,
        dormant_handle_opacity: 0.0,
        active_background_opacity: 0.0,
        active_handle_opacity: 0.55,
        interact_handle_opacity: 0.85,
        foreground_color: true,
        ..egui::style::ScrollStyle::floating()
    };
    style.interaction.selectable_labels = false;
    style.interaction.tooltip_delay = 0.4;
    style.animation_time = 0.1;
    style.url_in_tooltip = false;
}

fastframe_icons::icons! {
    /// Every icon the interface draws.
    pub enum Icon {
        prefix: "presto-icon-",
        directory: "../assets/icons/",
        ArrowLeft => lucide "arrow-left",
        ArrowRight => "arrow-right",
        AudioLines => "audio-lines",
        ChevronLeft => lucide "chevron-left",
        ChevronRight => lucide "chevron-right",
        CircleAlert => lucide "circle-alert",
        Disc => "disc-3",
        Ellipsis => lucide "ellipsis",
        House => "house",
        Info => lucide "info",
        Library => "library",
        ListEnd => "list-end",
        ListMusic => "list-music",
        ListPlus => "list-plus",
        ListStart => "list-start",
        Loader => "loader-circle",
        Music => "music",
        Pause => lucide "pause",
        PauseFilled => "pause-filled",
        PanelLeft => lucide "panel-left",
        Play => lucide "play",
        PlayFilled => "play-filled",
        Refresh => lucide "refresh-cw",
        Repeat => "repeat",
        Repeat1 => "repeat-1",
        Search => lucide "search",
        Settings => lucide "settings",
        Shuffle => "shuffle",
        SkipBackFilled => "skip-back-filled",
        SkipForwardFilled => "skip-forward-filled",
        Volume => "volume",
        Volume1 => "volume-1",
        Volume2 => lucide "volume-2",
        VolumeX => lucide "volume-x",
        X => lucide "x",
    }
}

/// A static icon.
pub fn icon(ui: &mut egui::Ui, icon: Icon, size: f32, color: Color32) -> Response {
    ui.add(icon.image(color, size))
}

/// Paints an icon centred in `rect` without allocating space.
pub fn paint_icon(ui: &egui::Ui, icon: Icon, rect: egui::Rect, size: f32, color: Color32) {
    let r = egui::Rect::from_center_size(rect.center() + play_glyph_offset(icon, size), Vec2::splat(size));
    icon.image(color, size).paint_at(ui, r);
}

/// Visible keyboard focus without changing layout.
pub fn focus_ring(ui: &egui::Ui, response: &Response) {
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect.expand(2.0),
            4.0,
            ui.visuals().selection.stroke,
            egui::StrokeKind::Outside,
        );
    }
    if response.gained_focus() {
        response.scroll_to_me(None);
    }
}

/// Optically centres play triangles.
pub fn play_glyph_offset(icon: Icon, icon_size: f32) -> Vec2 {
    if matches!(icon, Icon::PlayFilled | Icon::Play) {
        Vec2::new(icon_size * (0.03 - 1.0 / 24.0), 0.0)
    } else {
        Vec2::ZERO
    }
}

/// A frameless icon control whose colour lifts on hover.
pub fn icon_button(ui: &mut egui::Ui, icon: Icon, size: f32, color: Color32, hover: Color32, tooltip: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size + 12.0), Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), tooltip));
    if ui.is_rect_visible(rect) {
        let tint = if response.hovered() || response.has_focus() { hover } else { color };
        paint_icon(ui, icon, rect, size, tint);
    }
    focus_ring(ui, &response);
    if tooltip.is_empty() { response } else { response.on_hover_text(tooltip) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_matches_ui_spec() {
        let p = Palette::dark();
        let c = Color32::from_rgb;
        assert_eq!(p.window, c(0, 0, 0));
        assert_eq!(p.panel, c(0x1C, 0x1C, 0x1E));
        assert_eq!(p.surface, c(0x1C, 0x1C, 0x1E));
        assert_eq!(p.surface_hover, c(0x2C, 0x2C, 0x2E));
        assert_eq!(p.text, c(255, 255, 255));
        assert_eq!(p.secondary, c(0x98, 0x98, 0x9F));
        assert_eq!(p.accent, c(0xFA, 0x24, 0x3C));
        assert_eq!(p.danger, c(0xFF, 0x45, 0x3A));
        assert_eq!(p.warning, c(0xFF, 0x9F, 0x0A));
    }

    #[test]
    fn install_headless() {
        let ctx = egui::Context::default();
        install(&ctx);
        apply(&ctx, &Palette::dark());
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            icon(ui, Icon::PlayFilled, 16.0, Color32::WHITE);
            ui.label(egui::RichText::new("x").font(display()));
        });
        out.textures_delta.clear();
    }

    #[test]
    fn font_roles() {
        assert_eq!(body().size, 14.0);
        assert_eq!(label().size, 12.0);
        assert_eq!(heading().size, 20.0);
        assert_eq!(display().size, 32.0);
    }
}

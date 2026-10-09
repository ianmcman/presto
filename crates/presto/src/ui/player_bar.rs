//! Three-zone player bar: now playing, transport and seek, queue and volume.
use crate::app::App;
use crate::i18n::tr;
use crate::model::Action;
use crate::playback::fmt_time;
use crate::theme::{self, Icon};
use crate::ui::widgets::{self, SliderEvent, artwork, ellipsized, icon_button, thin_slider};
use egui::{Id, RichText, Sense, vec2};
use presto_ipc::{PlayState, RepeatMode};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let pal = app.palette;
    let p = app.state.player.clone();
    let enabled = app.ready() && p.track.is_some();
    let art = p.track.as_ref().map(|t| app.backend.art(t.artwork_url.as_deref(), 160));
    let mut acts = Vec::new();
    let pos = app.position();
    ui.columns(3, |cols| {
        // left: now playing
        cols[0].horizontal_centered(|ui| {
            let seed = p.track.as_ref().map_or("", |t| t.id.as_str());
            artwork(ui, &pal, art.as_ref().unwrap_or(&widgets::Art::Placeholder), seed, theme::BAR_ART, false);
            if let Some(t) = &p.track {
                let w = (ui.available_width() - theme::SM).max(0.0);
                ui.vertical(|ui| {
                    ui.add_space(theme::SM);
                    ellipsized(ui, &t.title, theme::body_strong(), pal.text, w);
                    let r = ellipsized(ui, &t.artist, theme::label(), pal.secondary, w).interact(Sense::click());
                    if r.clicked() {
                        acts.push(Action::OpenArtistNamed(t.artist.clone()));
                    }
                    r.on_hover_cursor(egui::CursorIcon::PointingHand);
                });
            }
        });
        // centre: transport and seek
        let ui = &mut cols[1];
        ui.add_enabled_ui(enabled, |ui| {
            ui.vertical_centered(|ui| {
                ui.horizontal(|ui| {
                    let row_w = 5.0 * theme::HIT + theme::PLAY_HIT;
                    ui.add_space(((ui.available_width() - row_w) / 2.0).max(0.0));
                    if icon_button(ui, &pal, Icon::Shuffle, 16.0, p.shuffle, &tr("Shuffle")).clicked() {
                        acts.push(Action::ToggleShuffle);
                    }
                    if icon_button(ui, &pal, Icon::SkipBackFilled, 18.0, false, &tr("Previous")).clicked() {
                        acts.push(Action::Previous);
                    }
                    let playing = matches!(p.state, PlayState::Playing | PlayState::Loading);
                    let (icon, label) = if playing { (Icon::PauseFilled, tr("Pause")) } else { (Icon::PlayFilled, tr("Play")) };
                    let (rect, resp) = ui.allocate_exact_size(vec2(theme::PLAY_HIT, theme::PLAY_HIT), Sense::click());
                    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &label));
                    ui.painter().circle_filled(rect.center(), theme::PLAY_HIT / 2.0, pal.accent);
                    theme::paint_icon(ui, icon, rect.translate(theme::play_glyph_offset(icon, 18.0)), 18.0, egui::Color32::WHITE);
                    theme::focus_ring(ui, &resp);
                    if resp.on_hover_text(&label).clicked() {
                        acts.push(Action::TogglePlay);
                    }
                    if icon_button(ui, &pal, Icon::SkipForwardFilled, 18.0, false, &tr("Next")).clicked() {
                        acts.push(Action::Next);
                    }
                    let ric = if p.repeat == RepeatMode::One { Icon::Repeat1 } else { Icon::Repeat };
                    if icon_button(ui, &pal, ric, 16.0, p.repeat != RepeatMode::Off, &tr("Repeat")).clicked() {
                        acts.push(Action::CycleRepeat);
                    }
                });
                ui.horizontal(|ui| {
                    let tw = 44.0;
                    let w = ui.available_width().min(480.0);
                    ui.add_space((ui.available_width() - w).max(0.0) / 2.0);
                    ui.add_sized([tw, 16.0], egui::Label::new(RichText::new(fmt_time(pos)).font(theme::mono()).color(pal.secondary)));
                    let dur = p.duration_ms;
                    let v = if dur > 0 { (pos as f32 / dur as f32).clamp(0.0, 1.0) } else { 0.0 };
                    let sw = (w - 2.0 * tw - 2.0 * theme::SM).max(40.0);
                    match thin_slider(ui, &pal, Id::new("seek"), v, sw, &tr("Seek")) {
                        SliderEvent::Drag(f) => app.seek.drag((f * dur as f32) as u64),
                        SliderEvent::Release(f) => {
                            app.seek.drag((f * dur as f32) as u64);
                            let id = p.track.as_ref().map_or("", |t| t.id.as_str());
                            if let Some(cmd) = app.seek.release(app.now(), p.seq, id) {
                                app.backend.command(cmd);
                            }
                        }
                        SliderEvent::None => {}
                    }
                    ui.add_sized([tw, 16.0], egui::Label::new(RichText::new(fmt_time(dur)).font(theme::mono()).color(pal.secondary)));
                });
            });
        });
        // right: queue toggle and volume, on the same line as the transport buttons
        let w = cols[2].available_width();
        cols[2].allocate_ui_with_layout(vec2(w, theme::PLAY_HIT), egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let now = app.now();
            match thin_slider(ui, &pal, Id::new("volume"), p.volume, 96.0, &tr("Volume")) {
                SliderEvent::Drag(v) => {
                    if let Some(v) = app.volume.drag(v, now) {
                        acts.push(Action::SetVolume(v));
                    }
                }
                SliderEvent::Release(v) => {
                    app.volume.release();
                    acts.push(Action::SetVolume(v));
                }
                SliderEvent::None => {}
            }
            let vic = if p.volume <= 0.0 {
                Icon::VolumeX
            } else if p.volume < 0.5 {
                Icon::Volume1
            } else {
                Icon::Volume2
            };
            if icon_button(ui, &pal, vic, 18.0, false, &tr("Mute")).clicked() {
                acts.push(Action::ToggleMute);
            }
            if icon_button(ui, &pal, Icon::ListMusic, 18.0, app.show_queue, &tr("Queue")).clicked() {
                acts.push(Action::ToggleQueuePanel);
            }
        });
    });
    for a in acts {
        app.act(a);
    }
}

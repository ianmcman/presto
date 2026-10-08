//! Keyboard shortcuts (D-06): spotifast map minus Spotify-only keys, plus N/P.
use crate::model::{Action, Page};
use egui::{Key, Modifiers};

pub fn handle(ctx: &egui::Context) -> Vec<Action> {
    let editing = ctx.text_edit_focused();
    let typing = ctx.memory(|m| m.focused().is_some());
    let mut out = Vec::new();
    ctx.input_mut(|i| {
        let (c, cs, none, shift, alt) = (Modifiers::CTRL, Modifiers::CTRL | Modifiers::SHIFT, Modifiers::NONE, Modifiers::SHIFT, Modifiers::ALT);
        let mut on = |m: Modifiers, k: Key, a: Action| {
            if i.consume_key(m, k) {
                out.push(a);
            }
        };
        // Shift variants before plain ones.
        on(cs, Key::A, Action::OpenCurrentArtist);
        on(cs, Key::Q, Action::ToggleQueuePanel);
        on(c, Key::F, Action::FocusSearch);
        on(c, Key::B, Action::ToggleSidebar);
        on(c, Key::Comma, Action::Open(Page::Settings));
        on(c, Key::Q, Action::Quit);
        on(c, Key::W, Action::Quit);
        on(c, Key::H, Action::Open(Page::Home));
        on(c, Key::Slash, Action::ShowShortcuts);
        if !editing {
            on(none, Key::Space, Action::TogglePlay);
            on(alt, Key::ArrowLeft, Action::Back);
            on(alt, Key::ArrowRight, Action::Forward);
            on(c, Key::ArrowLeft, Action::Previous);
            on(c, Key::ArrowRight, Action::Next);
            on(c, Key::ArrowUp, Action::VolumeBy(5));
            on(c, Key::ArrowDown, Action::VolumeBy(-5));
        }
        if !typing {
            on(shift, Key::Questionmark, Action::ShowShortcuts);
            on(none, Key::Questionmark, Action::ShowShortcuts);
            on(shift, Key::ArrowLeft, Action::SeekBy(-10_000));
            on(shift, Key::ArrowRight, Action::SeekBy(10_000));
            on(none, Key::M, Action::ToggleMute);
            on(none, Key::S, Action::ToggleShuffle);
            on(none, Key::R, Action::CycleRepeat);
            on(none, Key::Q, Action::ToggleQueuePanel);
            on(none, Key::Slash, Action::FocusSearch);
            on(none, Key::N, Action::Next);
            on(none, Key::P, Action::Previous);
        }
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(key: Key, modifiers: Modifiers) -> Vec<Action> {
        let ctx = egui::Context::default();
        let raw = egui::RawInput {
            events: vec![egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }],
            ..Default::default()
        };
        let mut out = vec![];
        ctx.run_ui(raw, |ui| out = handle(ui.ctx())).textures_delta.clear();
        out
    }

    #[test]
    fn space_toggles() {
        assert_eq!(keys(Key::Space, Modifiers::NONE), [Action::TogglePlay]);
    }

    #[test]
    fn ctrl_shift_q_is_queue_not_quit() {
        assert_eq!(keys(Key::Q, Modifiers::CTRL | Modifiers::SHIFT), [Action::ToggleQueuePanel]);
    }

    #[test]
    fn ctrl_q_quits() {
        assert_eq!(keys(Key::Q, Modifiers::CTRL), [Action::Quit]);
    }

    #[test]
    fn n_and_p() {
        assert_eq!(keys(Key::N, Modifiers::NONE), [Action::Next]);
        assert_eq!(keys(Key::P, Modifiers::NONE), [Action::Previous]);
    }

    #[test]
    fn shift_right_seeks() {
        assert_eq!(keys(Key::ArrowRight, Modifiers::SHIFT), [Action::SeekBy(10_000)]);
    }

    #[test]
    fn dropped_keys_do_nothing() {
        for k in [egui::Key::L, egui::Key::B] {
            assert!(keys(k, Modifiers::NONE).is_empty());
        }
    }

    #[test]
    fn ctrl_arrows() {
        assert_eq!(keys(Key::ArrowRight, Modifiers::CTRL), [Action::Next]);
        assert_eq!(keys(Key::ArrowUp, Modifiers::CTRL), [Action::VolumeBy(5)]);
    }
}

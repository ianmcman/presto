//! gettext plumbing. English only; the catalog hook exists for later locales.

pub use fastframe_i18n::gettext;

include!(concat!(env!("OUT_DIR"), "/catalogs.rs"));

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum Locale {
    #[default]
    English,
}

impl fastframe_i18n::Locale for Locale {
    fn catalog(self) -> Option<&'static dyn fastframe_i18n::Translator> {
        None
    }
}

/// Translates `msg` for the UI locale.
pub fn tr(msg: &'static str) -> String {
    gettext(Locale::English, msg).into_owned()
}

//! Contract types between views and the app shell. No egui here.
use presto_core::data::{
    models::{Item, ItemKind},
    search::Scope,
    view::{LibKind, Sort, ViewKey},
};

#[derive(Clone, Debug, PartialEq)]
pub enum Page {
    Home,
    Search,
    Library(LibKind),
    Settings,
    Album(Item),
    Playlist(Item),
    Artist(Item),
    /// "See all" from an artist shelf.
    ArtistAll { artist: Item, singles: bool },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    TogglePlay,
    Next,
    Previous,
    Seek(u64),
    SeekBy(i64),
    SetVolume(f32),
    VolumeBy(i32),
    ToggleMute,
    ToggleShuffle,
    CycleRepeat,
    PlayList { ids: Vec<String>, start: u32, shuffle: bool },
    QueuePlayFrom(u32),
    QueueRemove(u32),
    QueuePlayNext(String),
    QueueAdd(String),
    Open(Page),
    Replace(Page),
    Back,
    Forward,
    ToggleQueuePanel,
    ToggleSidebar,
    FocusSearch,
    ShowShortcuts,
    Quit,
    OpenArtistNamed(String),
    OpenCurrentArtist,
    LoadMore(ViewKey),
    Retry(ViewKey),
    Refresh(ViewKey),
    SetSort(Sort),
    SearchInput(String),
    SearchSubmit,
    SearchScope(Scope),
    ClearCache,
    SignIn,
    RestartEngine,
}

pub enum Event {
    Toast(crate::ui::toasts::Toast),
    Open(Page),
}

pub fn page_for(item: &Item) -> Option<Page> {
    match item.kind {
        ItemKind::Album => Some(Page::Album(item.clone())),
        ItemKind::Playlist => Some(Page::Playlist(item.clone())),
        ItemKind::Artist => Some(Page::Artist(item.clone())),
        _ => None,
    }
}

/// Id sent in SetQueue. 05-11 live check may switch library songs to playParams.catalogId.
pub fn queue_id(item: &Item) -> String {
    item.id.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(kind: ItemKind) -> Item {
        Item {
            id: "x".into(),
            kind,
            library: false,
            name: "n".into(),
            subtitle: None,
            album: None,
            duration_ms: None,
            artwork: None,
            play_params: None,
            release_date: None,
            track_count: None,
            playable: true,
        }
    }

    #[test]
    fn page_for_kinds() {
        assert!(matches!(page_for(&item(ItemKind::Album)), Some(Page::Album(_))));
        assert!(matches!(page_for(&item(ItemKind::Playlist)), Some(Page::Playlist(_))));
        assert!(matches!(page_for(&item(ItemKind::Artist)), Some(Page::Artist(_))));
        assert!(page_for(&item(ItemKind::Song)).is_none());
        assert_eq!(queue_id(&item(ItemKind::Song)), "x");
    }
}

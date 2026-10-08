//! App state, action dispatch, navigation and per-frame logic.
use crate::backend::Backend;
use crate::model::{Action, Event, Page};
use crate::playback::{Clock, Guard, GuardAction, GuardInput, SeekBar, Volume, next_repeat, seek_by, toggle_cmd, volume_by};
use crate::queue_ops::QueueOp;
use crate::theme::{self, Palette};
use crate::ui::{self, toasts::{Toast, Toasts}};
use presto_core::data::models::{Detail, Item, Shelf};
use presto_core::data::view::{ListState, Phase, Sort, ViewKey, detail_key, tracks_key};
use presto_core::{CoreState, EngineStatus};
use presto_ipc::AuthState;
use presto_ipc::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::watch;

type Rx<T> = watch::Receiver<ListState<T>>;

pub enum ViewData {
    None,
    Home { recent: Rx<Item>, shelves: Rx<Shelf> },
    /// Library(kind)
    List { key: ViewKey, rx: Rx<Item> },
    /// Album, Playlist, Artist, ArtistAll
    Detail { key: ViewKey, rx: Rx<Detail>, tracks: Option<(ViewKey, Rx<Item>)> },
}

pub struct App {
    pub backend: Backend,
    pub demo: bool,
    pub palette: Palette,
    /// Refreshed in `logic` only when the watch changed.
    pub state: CoreState,
    pub page: Page,
    pub view: ViewData,
    pub show_queue: bool,
    pub show_sidebar: bool,
    pub show_shortcuts: bool,
    pub confirm_clear: bool,
    pub focus_search: bool,
    pub clock: Clock,
    pub seek: SeekBar,
    pub volume: Volume,
    pub guard: Guard,
    pub toasts: Toasts,
    pub lib_sort: Sort,
    ctx: egui::Context,
    core_rx: watch::Receiver<CoreState>,
    back: Vec<Page>,
    forward: Vec<Page>,
    actions: Vec<Action>,
}

impl App {
    pub fn new(backend: Backend, demo: bool, ctx: &egui::Context) -> App {
        let palette = Palette::dark();
        theme::install(ctx);
        theme::apply(ctx, &palette);
        let wake = ctx.clone();
        backend.on_change(Arc::new(move || wake.request_repaint()));
        let core_rx = backend.state();
        let state = core_rx.borrow().clone();
        let mut app = App {
            backend,
            demo,
            palette,
            guard: Guard::new(state.engine_errors),
            state,
            page: Page::Home,
            view: ViewData::None,
            show_queue: false,
            show_sidebar: true,
            show_shortcuts: false,
            confirm_clear: false,
            focus_search: false,
            clock: Clock::default(),
            seek: SeekBar::default(),
            volume: Volume::default(),
            toasts: Toasts::default(),
            lib_sort: Sort::Default,
            ctx: ctx.clone(),
            core_rx,
            back: vec![],
            forward: vec![],
            actions: vec![],
        };
        app.navigate(Page::Home);
        app
    }

    pub fn now(&self) -> Instant {
        Instant::now()
    }

    pub fn position(&self) -> u64 {
        let (p, id) = (&self.state.player, self.state.player.track.as_ref().map_or("", |t| t.id.as_str()));
        self.seek.shown(self.clock.position(self.now()), p.seq, id, self.now())
    }

    pub fn ready(&self) -> bool {
        self.state.engine == EngineStatus::Ready && self.state.auth == Some(AuthState::SignedIn)
    }

    /// Queued; applied at the end of `frame_ui`.
    pub fn act(&mut self, a: Action) {
        self.actions.push(a);
    }

    fn open(&mut self, page: Page) {
        let data = self.backend.data();
        self.view = match &page {
            Page::Home => ViewData::Home { recent: data.list(ViewKey::RecentlyPlayed), shelves: data.shelves() },
            Page::Library(kind) => {
                let key = ViewKey::Library { kind: *kind, sort: self.lib_sort };
                ViewData::List { rx: data.list(key.clone()), key }
            }
            Page::Album(i) | Page::Playlist(i) => match detail_key(i) {
                Some(key) => ViewData::Detail {
                    rx: data.detail(key.clone()),
                    tracks: tracks_key(i).map(|k| (k.clone(), data.list(k))),
                    key,
                },
                None => ViewData::None,
            },
            Page::Artist(i) | Page::ArtistAll { artist: i, .. } => match detail_key(i) {
                Some(key) => ViewData::Detail { rx: data.detail(key.clone()), tracks: None, key },
                None => ViewData::None,
            },
            Page::Search | Page::Settings => ViewData::None,
        };
        self.page = page;
    }

    fn navigate(&mut self, page: Page) {
        if page == self.page && !matches!(self.view, ViewData::None) {
            return;
        }
        self.back.push(self.page.clone());
        self.forward.clear();
        self.open(page);
    }

    pub fn dispatch(&mut self, a: Action) {
        use Action::*;
        let playback = matches!(
            a,
            TogglePlay | Next | Previous | Seek(_) | SeekBy(_) | SetVolume(_) | VolumeBy(_) | ToggleMute
                | ToggleShuffle | CycleRepeat | PlayList { .. } | QueuePlayFrom(_) | QueueRemove(_)
                | QueuePlayNext(_) | QueueAdd(_)
        );
        if playback && !self.ready() {
            self.toasts.push(Toast::warning("Engine not ready"));
            return;
        }
        let p = self.state.player.clone();
        match a {
            TogglePlay => self.backend.command(toggle_cmd(p.state)),
            Next => self.backend.command(Command::Next),
            Previous => self.backend.command(Command::Prev),
            Seek(ms) => self.backend.command(Command::Seek { ms }),
            SeekBy(d) => self.backend.command(Command::Seek { ms: seek_by(self.position(), d, p.duration_ms) }),
            SetVolume(v) => self.backend.command(Command::SetVolume { volume: v }),
            VolumeBy(pct) => self.backend.command(Command::SetVolume { volume: volume_by(p.volume, pct) }),
            ToggleMute => {
                let v = self.volume.toggle_mute(p.volume);
                self.backend.command(Command::SetVolume { volume: v })
            }
            ToggleShuffle => self.backend.command(Command::SetShuffle { on: !p.shuffle }),
            CycleRepeat => self.backend.command(Command::SetRepeat { mode: next_repeat(p.repeat) }),
            PlayList { ids, start, shuffle } => self.backend.play_list(ids, start, shuffle),
            QueuePlayFrom(i) => self.edit(QueueOp::PlayFrom(i)),
            QueueRemove(i) => self.edit(QueueOp::Remove(i)),
            QueuePlayNext(id) => self.edit(QueueOp::PlayNext(id)),
            QueueAdd(id) => self.edit(QueueOp::Add(id)),
            Open(page) => self.navigate(page),
            Replace(page) => self.open(page),
            Back => {
                if let Some(prev) = self.back.pop() {
                    self.forward.push(self.page.clone());
                    self.open(prev);
                }
            }
            Forward => {
                if let Some(next) = self.forward.pop() {
                    self.back.push(self.page.clone());
                    self.open(next);
                }
            }
            ToggleQueuePanel => self.show_queue = !self.show_queue,
            ToggleSidebar => self.show_sidebar = !self.show_sidebar,
            FocusSearch => {
                self.navigate(Page::Search);
                self.focus_search = true;
            }
            ShowShortcuts => self.show_shortcuts = !self.show_shortcuts,
            Quit => self.ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            OpenArtistNamed(n) => self.backend.open_artist_named(n),
            OpenCurrentArtist => {
                if let Some(t) = &p.track {
                    self.backend.open_artist_named(t.artist.clone());
                }
            }
            LoadMore(k) => self.backend.data().load_more(&k),
            Retry(k) => self.backend.data().retry(&k),
            Refresh(k) => self.backend.data().refresh(&k),
            SetSort(s) => {
                self.lib_sort = s;
                if let Page::Library(kind) = self.page {
                    self.open(Page::Library(kind));
                }
            }
            SearchInput(t) => self.backend.search().input(&t),
            SearchSubmit => self.backend.search().submit(),
            SearchScope(s) => self.backend.search().set_scope(s),
            ClearCache => {
                self.backend.clear_cache();
                self.toasts.push(Toast::info("Cache cleared"));
            }
            SignIn => self.backend.show_sign_in(),
            RestartEngine => self.backend.restart_engine(),
        }
    }

    fn edit(&mut self, op: QueueOp) {
        self.backend.edit_queue(op, self.state.queue.rev);
    }

    fn loading(&self) -> bool {
        fn busy<T>(rx: &Rx<T>) -> bool {
            rx.borrow().phase != Phase::Idle
        }
        let v = match &self.view {
            ViewData::None => false,
            ViewData::Home { recent, shelves } => busy(recent) || busy(shelves),
            ViewData::List { rx, .. } => busy(rx),
            ViewData::Detail { rx, tracks, .. } => busy(rx) || tracks.as_ref().is_some_and(|(_, t)| busy(t)),
        };
        v || self.backend.search().state().borrow().searching
    }

    pub fn logic(&mut self, ctx: &egui::Context) {
        let now = self.now();
        if self.core_rx.has_changed().unwrap_or(false) {
            self.state = self.core_rx.borrow_and_update().clone();
        }
        self.clock.observe(&self.state.player, now);
        for ev in self.backend.poll() {
            match ev {
                Event::Toast(t) => self.toasts.push(t),
                Event::Open(p) => self.navigate(p),
            }
        }
        self.run_guard();
        for a in ui::keys::handle(ctx) {
            self.dispatch(a);
        }
        if self.state.player.state == presto_ipc::PlayState::Playing {
            ctx.request_repaint_after(Duration::from_millis(250));
        }
        if self.loading() {
            // ponytail: polling repaint instead of per-receiver wake tasks
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    fn run_guard(&mut self) {
        let s = &self.state;
        let input = GuardInput {
            track: s.player.track.as_ref(),
            state: s.player.state,
            position_ms: s.player.position_ms,
            queue_rev: s.queue.rev,
            queue_index: s.queue.index,
            queue_len: s.queue.items.len(),
            engine_errors: s.engine_errors,
            last_error: s.last_engine_error.as_ref().map(|e| &e.kind),
        };
        let Some(action) = self.guard.observe(&input) else { return };
        match action {
            GuardAction::SkipUnavailable { title } => {
                self.backend.command(Command::Next);
                self.toasts.push(Toast::warning(format!("Skipped \"{title}\": unavailable.")));
            }
            GuardAction::SkipFailed { title, kind } => {
                self.backend.command(Command::Next);
                self.toasts.push(Toast::error(format!(
                    "Couldn't play \"{title}\" ({kind}). Skipping to the next track."
                )));
            }
            GuardAction::AllFailed => self.toasts.push(Toast::error("Nothing in the queue can be played.")),
        }
    }

    pub fn frame_ui(&mut self, ui: &mut egui::Ui) {
        ui::show(self, ui);
        for a in std::mem::take(&mut self.actions) {
            self.dispatch(a);
        }
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        App::logic(self, ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.frame_ui(ui);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.backend.shutdown();
    }
}

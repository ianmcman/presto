//! Backend over presto-core: owns the runtime, sends commands, polls events. The UI never blocks on it
//! except in `start` and `shutdown`.
use crate::model::{Event, Page};
use crate::playback::MIN_SEEK_MS;
use crate::queue_ops::{self, Plan, QueueOp};
use crate::ui::toasts::Toast;
use crate::ui::widgets::Art;
use presto_core::data::artwork::{ArtState, expand};
use presto_core::data::error::UiErrorKind;
use presto_core::data::models::parse_search;
use presto_core::data::search::Search;
use presto_core::data::DataHandle;
use presto_core::{Core, CoreConfig, CoreHandle, CoreState, EngineStatus};
use presto_ipc::{Command, Outcome, PlayState};
use std::io;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;
use tokio::sync::watch;

pub struct Backend {
    rt: tokio::runtime::Runtime,
    core: CoreHandle,
    data: DataHandle,
    search: Search,
    tx: Sender<Event>,
    rx: Receiver<Event>,
}

fn verb(c: &Command) -> &'static str {
    match c {
        Command::Play => "play",
        Command::Pause => "pause",
        Command::Seek { .. } => "seek",
        Command::Next => "skip",
        Command::Prev => "go back",
        Command::SetVolume { .. } => "set the volume",
        Command::SetShuffle { .. } => "change shuffle",
        Command::SetRepeat { .. } => "change repeat",
        Command::SetQueue { .. } => "start playback",
        _ => "do that",
    }
}

/// Sends `cmd`; an error outcome becomes an error toast. Returns whether it succeeded.
async fn send(core: &CoreHandle, tx: &Sender<Event>, cmd: Command) -> bool {
    let v = verb(&cmd);
    match core.command(cmd).await {
        Outcome::Ok { .. } => true,
        Outcome::Err { error } => {
            let msg = UiErrorKind::from_ipc(&error.kind).message();
            let _ = tx.send(Event::Toast(Toast::error(format!("Couldn't {v}: {msg}"))));
            false
        }
    }
}

impl Backend {
    pub fn start(cfg: CoreConfig) -> io::Result<Backend> {
        let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
        let paths = cfg.paths.clone();
        let (core, data, search) = rt.block_on(async {
            let core = Core::start(cfg).await?;
            let data = DataHandle::new(core.clone(), &paths)?;
            let search = Search::new(data.client().clone(), data.store().clone());
            io::Result::Ok((core, data, search))
        })?;
        let (tx, rx) = channel();
        Ok(Backend { rt, core, data, search, tx, rx })
    }

    pub fn state(&self) -> watch::Receiver<CoreState> {
        self.core.state()
    }

    pub fn command(&self, cmd: Command) {
        let (core, tx) = (self.core.clone(), self.tx.clone());
        self.rt.spawn(async move {
            send(&core, &tx, cmd).await;
        });
    }

    pub fn play_list(&self, ids: Vec<String>, start: u32, shuffle: bool) {
        let (core, tx) = (self.core.clone(), self.tx.clone());
        self.rt.spawn(async move {
            if send(&core, &tx, Command::SetShuffle { on: shuffle }).await {
                send(&core, &tx, Command::SetQueue { ids, start, play: true }).await;
            }
        });
    }

    /// D-08: edits go through SetQueue; a stale `seen_rev` drops the edit (Pitfall 3).
    pub fn edit_queue(&self, op: QueueOp, seen_rev: u64) {
        let (core, tx) = (self.core.clone(), self.tx.clone());
        self.rt.spawn(async move {
            let mut rx = core.state();
            let s = rx.borrow().clone();
            if s.engine != EngineStatus::Ready || s.queue.rev != seen_rev {
                let _ = tx.send(Event::Toast(Toast::warning("Queue changed. Try again.")));
                return;
            }
            let toast = match &op {
                QueueOp::PlayNext(_) => Some("Added to Up Next"),
                QueueOp::Add(_) => Some("Added to queue"),
                _ => None,
            };
            let ok = match queue_ops::apply(&s.queue, &op) {
                Plan::Noop => return,
                Plan::Clear => send(&core, &tx, Command::Pause).await,
                Plan::Edit(e) => {
                    let pos = s.player.position_ms;
                    let cmd = Command::SetQueue { ids: e.ids, start: e.start, play: s.player.state == PlayState::Playing };
                    let ok = send(&core, &tx, cmd).await;
                    if ok && e.keep_position && pos >= MIN_SEEK_MS {
                        let _ = tokio::time::timeout(
                            Duration::from_secs(5),
                            rx.wait_for(|s| s.player.state != PlayState::Loading),
                        )
                        .await;
                        send(&core, &tx, Command::Seek { ms: pos }).await;
                    }
                    ok
                }
            };
            if let (true, Some(t)) = (ok, toast) {
                let _ = tx.send(Event::Toast(Toast::info(t)));
            }
        });
    }

    pub fn open_artist_named(&self, name: String) {
        let (client, tx) = (self.data.client().clone(), self.tx.clone());
        self.rt.spawn(async move {
            let q = [("term", name.as_str()), ("types", "artists"), ("limit", "1")];
            let ev = match client.catalog("search", &q).await {
                Ok(v) => match parse_search(&name, &v).artists.into_iter().next() {
                    Some(a) => Event::Open(Page::Artist(a)),
                    None => Event::Toast(Toast::warning("Couldn't find that artist.")),
                },
                Err(e) => Event::Toast(Toast::error(format!("Couldn't open the artist: {}", e.message()))),
            };
            let _ = tx.send(ev);
        });
    }

    pub fn art(&self, url: Option<&str>, px: u32) -> Art {
        let Some(u) = url.filter(|u| !u.contains("example.invalid")) else { return Art::Placeholder };
        let _g = self.rt.enter();
        match self.data.art().get(&expand(u, px)) {
            ArtState::Ready(p) => Art::Ready(p),
            ArtState::Pending => Art::Pending,
            ArtState::Failed => Art::Placeholder,
        }
    }

    pub fn data(&self) -> &DataHandle {
        &self.data
    }

    pub fn search(&self) -> &Search {
        &self.search
    }

    pub fn on_change(&self, wake: Arc<dyn Fn() + Send + Sync>) {
        let mut rx = self.core.state();
        self.rt.spawn(async move {
            while rx.changed().await.is_ok() {
                wake();
            }
        });
    }

    pub fn poll(&self) -> Vec<Event> {
        self.rx.try_iter().collect()
    }

    pub fn show_sign_in(&self) {
        let core = self.core.clone();
        self.rt.spawn(async move {
            core.show_sign_in().await;
        });
    }

    pub fn restart_engine(&self) {
        self.core.restart_engine();
    }

    pub fn clear_cache(&self) {
        self.data.clear_cache();
    }

    pub fn shutdown(&self) {
        self.rt.block_on(self.core.clone().shutdown());
    }
}

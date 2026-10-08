mod checks;
mod rss;
mod session;

use clap::{Parser, Subcommand};
use session::{EngineOpts, R};
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "presto-spike", about = "Phase 2 engine feasibility driver")]
pub struct Cli {
    /// Engine executable. Default: <engine-dir>/node_modules/.bin/electron, with <engine-dir> as app path.
    #[arg(long)]
    engine_bin: Option<PathBuf>,
    #[arg(long, default_value = "engine")]
    engine_dir: PathBuf,
    /// Extra engine argument, repeatable (e.g. --engine-arg=--diag)
    #[arg(long = "engine-arg", allow_hyphen_values = true)]
    engine_arg: Vec<String>,
    #[arg(long)]
    socket: Option<PathBuf>,
    /// Default: $XDG_STATE_HOME/presto/engine-profile, else $HOME/.local/state/presto/engine-profile
    #[arg(long)]
    profile: Option<PathBuf>,
    #[arg(
        long,
        default_value = ".planning/phases/02-engine-feasibility-spike-gate/logs"
    )]
    log_dir: PathBuf,
    #[arg(long, default_value = "run")]
    label: String,
    /// Accept an engine that lists the mock capability (tests only)
    #[arg(long)]
    allow_mock: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(clap::Args, Clone)]
pub struct SongArgs {
    /// Catalog song id; if absent, search for one
    #[arg(long)]
    pub song: Option<String>,
    #[arg(long, default_value = "Bohemian Rhapsody")]
    pub search_term: String,
}

#[derive(Subcommand)]
enum Cmd {
    /// NAME = hello | musickit | session | api | playback | events | all
    Check {
        name: String,
        #[command(flatten)]
        song: SongArgs,
        #[arg(long, default_value_t = 60)]
        min_play_secs: u64,
        #[arg(long, default_value_t = 120)]
        seek_secs: u64,
        #[arg(long, default_value_t = 90)]
        auth_wait_secs: u64,
    },
    /// Show the engine window until signed in, then quit cleanly
    Signin {
        #[arg(long, default_value_t = 600)]
        wait_secs: u64,
    },
    /// Sample engine process-tree RSS/PSS into a CSV
    Measure {
        #[command(flatten)]
        song: SongArgs,
        #[arg(long, default_value_t = 300)]
        play_secs: u64,
        #[arg(long, default_value_t = 5)]
        interval_secs: u64,
        #[arg(long, default_value_t = 20)]
        settle_secs: u64,
    },
}

pub struct Ctx {
    pub eo: EngineOpts,
    pub allow_mock: bool,
}

fn private_dir(p: &std::path::Path) -> R<()> {
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(p)?;
    std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o700))?;
    Ok(())
}

fn default_profile() -> PathBuf {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))
        .unwrap_or_else(|| PathBuf::from(".local/state"));
    base.join("presto").join("engine-profile")
}

async fn run(cli: Cli) -> R<i32> {
    let profile = cli.profile.unwrap_or_else(default_profile);
    private_dir(&profile)?;
    std::fs::create_dir_all(&cli.log_dir)?;
    let (bin, app_path) = match cli.engine_bin {
        Some(b) => (b, None),
        None => (
            cli.engine_dir.join("node_modules/.bin/electron"),
            Some(cli.engine_dir),
        ),
    };
    let eo = EngineOpts {
        bin,
        app_path,
        socket: match cli.socket {
            Some(s) => s,
            None => presto_ipc::transport::socket_path()?,
        },
        profile,
        extra: cli.engine_arg,
        log_dir: cli.log_dir,
        label: cli.label,
    };
    let mut cx = Ctx {
        eo,
        allow_mock: cli.allow_mock,
    };
    match cli.cmd {
        Cmd::Check {
            name,
            song,
            min_play_secs,
            seek_secs,
            auth_wait_secs,
        } => checks::run_checks(&cx, &name, &song, min_play_secs, seek_secs, auth_wait_secs).await,
        Cmd::Signin { wait_secs } => checks::signin(&mut cx, wait_secs).await,
        Cmd::Measure {
            song,
            play_secs,
            interval_secs,
            settle_secs,
        } => checks::measure(&cx, &song, play_secs, interval_secs, settle_secs).await,
    }
}

#[tokio::main]
async fn main() {
    match run(Cli::parse()).await {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("presto-spike: {e}");
            std::process::exit(2);
        }
    }
}

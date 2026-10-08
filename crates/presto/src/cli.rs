//! Command line surface.

use crate::ctl::parse;
use presto_ipc::ctl::CtlOp;

#[derive(clap::Parser, Debug)]
#[command(name = "presto", about = "Apple Music for Linux")]
pub struct Cli {
    /// Run against the mock engine. No account or Widevine needed.
    #[arg(long, global = true)]
    pub demo: bool,
    /// Mock engine fault, passed through (demo only, repeatable): none, hang, crash[@ms], auth_expired, slow[=ms], rate_limited[=ms], signed_out.
    #[arg(long = "fault", value_parser = parse_fault, requires = "demo")]
    pub faults: Vec<String>,
    /// Engine directory holding node_modules/electron (real mode).
    #[arg(long, env = "PRESTO_ENGINE_DIR", default_value = "engine")]
    pub engine_dir: std::path::PathBuf,
    /// Subcommand to run; if not specified, launches the GUI.
    #[command(subcommand)]
    pub cmd: Option<Sub>,
}

#[derive(clap::Subcommand, Debug, Clone, PartialEq)]
pub enum Sub {
    Play,
    Pause,
    Toggle,
    Next,
    Prev,
    Stop,
    Seek {
        #[arg(allow_hyphen_values = true)]
        to: String,
    },
    Volume {
        #[arg(allow_hyphen_values = true)]
        level: String,
    },
    Shuffle {
        #[arg(value_enum)]
        mode: Option<OnOff>,
    },
    Repeat {
        #[arg(value_enum)]
        mode: Option<RepeatArg>,
    },
    Status {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        watch: bool,
    },
    Raise,
    Quit,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq)]
pub enum OnOff {
    On,
    Off,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq)]
pub enum RepeatArg {
    Off,
    All,
    One,
}

impl Sub {
    pub fn to_op(&self) -> Result<CtlOp, String> {
        match self {
            Sub::Play => Ok(CtlOp::Play),
            Sub::Pause => Ok(CtlOp::Pause),
            Sub::Toggle => Ok(CtlOp::Toggle),
            Sub::Next => Ok(CtlOp::Next),
            Sub::Prev => Ok(CtlOp::Prev),
            Sub::Stop => Ok(CtlOp::Stop),
            Sub::Seek { to } => parse::parse_seek(to),
            Sub::Volume { level } => parse::parse_volume(level),
            Sub::Shuffle { mode } => Ok(CtlOp::Shuffle {
                on: mode.map(|m| m == OnOff::On),
            }),
            Sub::Repeat { mode } => Ok(CtlOp::Repeat {
                mode: mode.map(|m| match m {
                    RepeatArg::Off => presto_ipc::RepeatMode::Off,
                    RepeatArg::All => presto_ipc::RepeatMode::All,
                    RepeatArg::One => presto_ipc::RepeatMode::One,
                }),
            }),
            Sub::Status { json: _, watch } => {
                if *watch {
                    Ok(CtlOp::Subscribe)
                } else {
                    Ok(CtlOp::Status)
                }
            }
            Sub::Raise => Ok(CtlOp::Raise),
            Sub::Quit => Ok(CtlOp::Quit),
        }
    }
}

fn parse_fault(s: &str) -> Result<String, String> {
    s.parse::<presto_ipc::FaultSpec>().map(|_| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn cli_demo_fault() {
        let c = Cli::try_parse_from(["presto", "--demo", "--fault", "slow", "--fault", "crash@500"]).unwrap();
        assert!(c.demo);
        assert_eq!(c.faults, ["slow", "crash@500"]);
    }

    #[test]
    fn cli_bad_fault() {
        assert!(Cli::try_parse_from(["presto", "--demo", "--fault", "bogus"]).is_err());
    }

    #[test]
    fn cli_fault_needs_demo() {
        assert!(Cli::try_parse_from(["presto", "--fault", "slow"]).is_err());
    }

    #[test]
    fn cli_engine_dir_default() {
        if std::env::var_os("PRESTO_ENGINE_DIR").is_some() {
            return;
        }
        let c = Cli::try_parse_from(["presto"]).unwrap();
        assert_eq!(c.engine_dir, std::path::PathBuf::from("engine"));
        assert!(!c.demo);
        assert!(c.cmd.is_none());
    }

    #[test]
    fn parse_seek_examples() {
        let c = Cli::try_parse_from(["presto", "seek", "-10"]).unwrap();
        assert_eq!(c.cmd, Some(Sub::Seek { to: "-10".to_string() }));
        let op = c.cmd.unwrap().to_op().unwrap();
        assert_eq!(op, CtlOp::SeekBy { ms: -10000 });
    }

    #[test]
    fn parse_volume_examples() {
        let c = Cli::try_parse_from(["presto", "volume", "-5"]).unwrap();
        assert_eq!(c.cmd, Some(Sub::Volume { level: "-5".to_string() }));
        let op = c.cmd.unwrap().to_op().unwrap();
        assert_eq!(op, CtlOp::VolumeBy { pct: -5 });
    }

    #[test]
    fn parse_status_with_flags() {
        let c = Cli::try_parse_from(["presto", "--demo", "status", "--json", "--watch"]).unwrap();
        assert!(c.demo);
        assert_eq!(
            c.cmd,
            Some(Sub::Status {
                json: true,
                watch: true
            })
        );
    }

    #[test]
    fn parse_status_watch_reverse_order() {
        let c = Cli::try_parse_from(["presto", "status", "--json", "--demo"]).unwrap();
        assert!(c.demo);
        assert_eq!(
            c.cmd,
            Some(Sub::Status {
                json: true,
                watch: false
            })
        );
    }

    #[test]
    fn to_op_mapping() {
        let sub = Sub::Shuffle { mode: None };
        assert_eq!(sub.to_op().unwrap(), CtlOp::Shuffle { on: None });

        let sub = Sub::Shuffle {
            mode: Some(OnOff::On),
        };
        assert_eq!(sub.to_op().unwrap(), CtlOp::Shuffle { on: Some(true) });

        let sub = Sub::Repeat { mode: None };
        assert_eq!(sub.to_op().unwrap(), CtlOp::Repeat { mode: None });

        let sub = Sub::Repeat {
            mode: Some(RepeatArg::All),
        };
        assert_eq!(
            sub.to_op().unwrap(),
            CtlOp::Repeat {
                mode: Some(presto_ipc::RepeatMode::All)
            }
        );
    }
}

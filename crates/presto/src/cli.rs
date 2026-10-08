//! Command line surface.

#[derive(clap::Parser, Debug)]
#[command(name = "presto", about = "Apple Music for Linux")]
pub struct Cli {
    /// Run against the mock engine. No account or Widevine needed.
    #[arg(long)]
    pub demo: bool,
    /// Mock engine fault, passed through (demo only, repeatable): none, hang, crash[@ms], auth_expired, slow[=ms], rate_limited[=ms], signed_out.
    #[arg(long = "fault", value_parser = parse_fault, requires = "demo")]
    pub faults: Vec<String>,
    /// Engine directory holding node_modules/electron (real mode).
    #[arg(long, env = "PRESTO_ENGINE_DIR", default_value = "engine")]
    pub engine_dir: std::path::PathBuf,
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
    }
}

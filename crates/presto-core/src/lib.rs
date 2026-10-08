//! Presto core: engine supervisor, queue mirror, auth state, data layer. No UI.
pub mod auth;
pub mod backoff;
pub mod config;
pub mod data;
pub mod mirror;
pub mod paths;
pub mod state;
mod supervisor;

pub use config::{CoreConfig, Launch, Launcher, REQUIRED_BRIDGE_CAPS, Timings, check_bridge};
pub use state::{BridgeInfo, CoreState, EngineStatus};
pub use supervisor::{Core, CoreHandle};

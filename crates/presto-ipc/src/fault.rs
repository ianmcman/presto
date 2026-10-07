use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

pub const DEFAULT_SLOW_MS: u64 = 3000;

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FaultSpec {
    None,
    Hang,
    Crash { after_ms: Option<u64> },
    AuthExpired,
    Slow { delay_ms: u64 },
}

impl FromStr for FaultSpec {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        let bad = || {
            format!(
                "unknown fault \"{s}\"; expected none, hang, crash[@ms], auth_expired, slow[=ms]"
            )
        };
        Ok(match s {
            "none" => Self::None,
            "hang" => Self::Hang,
            "crash" => Self::Crash { after_ms: None },
            "auth_expired" => Self::AuthExpired,
            "slow" => Self::Slow {
                delay_ms: DEFAULT_SLOW_MS,
            },
            _ => {
                if let Some(ms) = s.strip_prefix("crash@") {
                    Self::Crash {
                        after_ms: Some(ms.parse().map_err(|_| bad())?),
                    }
                } else if let Some(ms) = s.strip_prefix("slow=") {
                    Self::Slow {
                        delay_ms: ms.parse().map_err(|_| bad())?,
                    }
                } else {
                    return Err(bad());
                }
            }
        })
    }
}

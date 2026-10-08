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
    RateLimited { retry_after_ms: Option<u64> },
    SignedOut,
}

impl FromStr for FaultSpec {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        let bad = || {
            format!(
                "unknown fault \"{s}\"; expected none, hang, crash[@ms], auth_expired, slow[=ms], rate_limited[=ms], signed_out"
            )
        };
        Ok(match s {
            "none" => Self::None,
            "hang" => Self::Hang,
            "crash" => Self::Crash { after_ms: None },
            "auth_expired" => Self::AuthExpired,
            "signed_out" => Self::SignedOut,
            "rate_limited" => Self::RateLimited {
                retry_after_ms: None,
            },
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
                } else if let Some(ms) = s.strip_prefix("rate_limited=") {
                    Self::RateLimited {
                        retry_after_ms: Some(ms.parse().map_err(|_| bad())?),
                    }
                } else {
                    return Err(bad());
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rate_limited_and_signed_out() {
        let p = |s: &str| s.parse::<FaultSpec>();
        assert_eq!(
            p("rate_limited"),
            Ok(FaultSpec::RateLimited {
                retry_after_ms: None
            })
        );
        assert_eq!(
            p("rate_limited=1500"),
            Ok(FaultSpec::RateLimited {
                retry_after_ms: Some(1500)
            })
        );
        assert!(p("rate_limited=x").is_err());
        assert_eq!(p("signed_out"), Ok(FaultSpec::SignedOut));
    }
}

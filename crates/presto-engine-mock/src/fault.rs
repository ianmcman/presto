use presto_ipc::{AuthState, Event, FaultSpec};
use std::time::Duration;

#[derive(Default)]
pub struct Faults {
    pub hang: bool,
    pub auth_expired: bool,
    /// Started signed out: cmd and req are rejected until `none`.
    pub signed_out: bool,
    pub slow: Option<Duration>,
    /// Every req gets `rate_limited` with this retry_after_ms.
    pub rate_limited: Option<Option<u64>>,
}

impl Faults {
    /// Applies one fault and returns events to emit. Crash is handled by the caller.
    pub fn apply(&mut self, spec: &FaultSpec) -> Vec<Event> {
        match spec {
            FaultSpec::None => {
                let was_expired =
                    std::mem::take(&mut self.auth_expired) | std::mem::take(&mut self.signed_out);
                self.hang = false;
                self.slow = None;
                self.rate_limited = None;
                if was_expired {
                    return vec![Event::Auth {
                        state: AuthState::SignedIn,
                    }];
                }
            }
            FaultSpec::Hang => self.hang = true,
            FaultSpec::AuthExpired => {
                self.auth_expired = true;
                return vec![Event::Auth {
                    state: AuthState::Expired,
                }];
            }
            FaultSpec::Slow { delay_ms } => self.slow = Some(Duration::from_millis(*delay_ms)),
            FaultSpec::RateLimited { retry_after_ms } => self.rate_limited = Some(*retry_after_ms),
            FaultSpec::SignedOut => {
                self.signed_out = true;
                return vec![Event::Auth {
                    state: AuthState::SignedOut,
                }];
            }
            FaultSpec::Crash { .. } => {}
        }
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_clean() {
        let f = Faults::default();
        assert!(!f.hang && !f.auth_expired && f.slow.is_none());
    }

    #[test]
    fn auth_expired_then_none() {
        let mut f = Faults::default();
        let ev = f.apply(&FaultSpec::AuthExpired);
        assert!(matches!(
            ev.as_slice(),
            [Event::Auth {
                state: AuthState::Expired
            }]
        ));
        assert!(f.auth_expired);
        let ev = f.apply(&FaultSpec::None);
        assert!(matches!(
            ev.as_slice(),
            [Event::Auth {
                state: AuthState::SignedIn
            }]
        ));
        assert!(!f.auth_expired);
    }

    #[test]
    fn signed_out_then_none() {
        let mut f = Faults {
            signed_out: true,
            ..Faults::default()
        };
        let ev = f.apply(&FaultSpec::None);
        assert!(matches!(
            ev.as_slice(),
            [Event::Auth {
                state: AuthState::SignedIn
            }]
        ));
        assert!(!f.signed_out);
    }

    #[test]
    fn slow_hang_then_none() {
        let mut f = Faults::default();
        assert!(f.apply(&FaultSpec::Slow { delay_ms: 1500 }).is_empty());
        assert_eq!(f.slow, Some(Duration::from_millis(1500)));
        f.apply(&FaultSpec::Hang);
        assert!(f.hang);
        assert!(f.apply(&FaultSpec::None).is_empty());
        assert!(!f.hang && f.slow.is_none());
    }

    #[test]
    fn rate_limited_then_none() {
        let mut f = Faults::default();
        let spec = FaultSpec::RateLimited {
            retry_after_ms: Some(200),
        };
        assert!(f.apply(&spec).is_empty());
        assert_eq!(f.rate_limited, Some(Some(200)));
        assert!(f.apply(&FaultSpec::None).is_empty());
        assert_eq!(f.rate_limited, None);
    }

    #[test]
    fn signed_out_live_then_none() {
        let mut f = Faults::default();
        let ev = f.apply(&FaultSpec::SignedOut);
        assert!(matches!(
            ev.as_slice(),
            [Event::Auth {
                state: AuthState::SignedOut
            }]
        ));
        assert!(f.signed_out);
        let ev = f.apply(&FaultSpec::None);
        assert!(matches!(
            ev.as_slice(),
            [Event::Auth {
                state: AuthState::SignedIn
            }]
        ));
    }
}

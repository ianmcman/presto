//! D-02 restart backoff. Pure: callers pass `Instant`s.
use std::time::{Duration, Instant};

pub const MAX_FAST_FAILURES: u32 = 5;

/// D-02: 1s doubling to 30s; give up after 5 consecutive failures under 60s uptime; 120s uptime resets.
pub struct Backoff {
    consec: u32,
    fast: u32,
    started: Option<Instant>,
    base: Duration,
    cap: Duration,
    fast_under: Duration,
    stable: Duration,
}

impl Backoff {
    pub fn new(base: Duration, cap: Duration, fast: Duration, stable: Duration) -> Backoff {
        Backoff { consec: 0, fast: 0, started: None, base, cap, fast_under: fast, stable }
    }

    pub fn on_start(&mut self, now: Instant) {
        self.started = Some(now);
    }

    /// None = give up (5th fast failure). Some(delay) otherwise.
    pub fn on_failure(&mut self, now: Instant) -> Option<Duration> {
        let up = self.started.map_or(Duration::ZERO, |s| now.saturating_duration_since(s));
        if up >= self.stable {
            self.consec = 0;
            self.fast = 0;
        }
        if up < self.fast_under {
            self.fast += 1;
            if self.fast >= MAX_FAST_FAILURES {
                return None;
            }
        }
        let delay = self.base.saturating_mul(1u32.checked_shl(self.consec).unwrap_or(u32::MAX)).min(self.cap);
        self.consec = self.consec.saturating_add(1);
        Some(delay)
    }

    /// Manual Restart.
    pub fn reset(&mut self) {
        self.consec = 0;
        self.fast = 0;
        self.started = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const S: Duration = Duration::from_secs(1);
    fn b() -> Backoff {
        Backoff::new(S, 30 * S, 60 * S, 120 * S)
    }

    fn fail(b: &mut Backoff, t0: Instant, up: u64) -> Option<Duration> {
        b.on_start(t0);
        b.on_failure(t0 + Duration::from_secs(up))
    }

    #[test]
    fn backoff_sequence() {
        let (mut b, t0) = (b(), Instant::now());
        let got: Vec<_> = (0..7).map(|_| fail(&mut b, t0, 70).unwrap().as_secs()).collect();
        assert_eq!(got, [1, 2, 4, 8, 16, 30, 30]);
    }

    #[test]
    fn backoff_gives_up_on_fifth_fast() {
        let (mut b, t0) = (b(), Instant::now());
        for _ in 0..4 {
            assert!(fail(&mut b, t0, 1).is_some());
        }
        assert!(fail(&mut b, t0, 1).is_none());
    }

    #[test]
    fn backoff_stable_resets() {
        let (mut b, t0) = (b(), Instant::now());
        for _ in 0..3 {
            fail(&mut b, t0, 1).unwrap();
        }
        assert_eq!(fail(&mut b, t0, 121), Some(S));
        for _ in 0..4 {
            assert!(fail(&mut b, t0, 1).is_some());
        }
        assert!(fail(&mut b, t0, 1).is_none());
    }

    #[test]
    fn backoff_reset() {
        let (mut b, t0) = (b(), Instant::now());
        while fail(&mut b, t0, 1).is_some() {}
        b.reset();
        assert_eq!(fail(&mut b, t0, 1), Some(S));
    }

    #[test]
    fn backoff_on_failure_without_start() {
        let mut b = b();
        assert_eq!(b.on_failure(Instant::now()), Some(S));
        assert_eq!(b.fast, 1);
    }
}

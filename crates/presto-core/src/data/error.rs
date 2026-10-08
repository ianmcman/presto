//! UI-facing error kinds mapped from IPC errors.
use presto_ipc::ErrorKind;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum UiErrorKind {
    Offline,
    SignedOut,
    AuthExpired,
    RateLimited { retry_after_ms: Option<u64> },
    Timeout,
    Unavailable,
    Upstream { status: u16 },
    NotFound,
    Internal,
}

impl UiErrorKind {
    pub fn from_ipc(k: &ErrorKind) -> UiErrorKind {
        match k {
            ErrorKind::Timeout => Self::Timeout,
            ErrorKind::AuthExpired => Self::AuthExpired,
            ErrorKind::RateLimited { retry_after_ms } => Self::RateLimited { retry_after_ms: *retry_after_ms },
            ErrorKind::NotFound => Self::NotFound,
            ErrorKind::Unavailable => Self::Unavailable,
            ErrorKind::Upstream { status } => Self::Upstream { status: *status },
            ErrorKind::Internal => Self::Internal,
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::Offline => "Engine offline. Showing cached data.".into(),
            Self::SignedOut => "Sign in to load this view.".into(),
            Self::AuthExpired => "Session expired. Sign in again.".into(),
            Self::RateLimited { .. } => "Apple Music is rate limiting requests.".into(),
            Self::Timeout => "The request timed out.".into(),
            Self::Unavailable => "Apple Music is unavailable right now.".into(),
            Self::Upstream { status } => format!("Apple Music returned HTTP {status}."),
            Self::NotFound => "Not found.".into(),
            Self::Internal => "Something went wrong in the engine.".into(),
        }
    }

    pub fn retryable(&self) -> bool {
        !matches!(self, Self::NotFound | Self::AuthExpired | Self::SignedOut)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_each_variant() {
        use ErrorKind as K;
        assert_eq!(UiErrorKind::from_ipc(&K::Timeout), UiErrorKind::Timeout);
        assert_eq!(UiErrorKind::from_ipc(&K::AuthExpired), UiErrorKind::AuthExpired);
        assert_eq!(
            UiErrorKind::from_ipc(&K::RateLimited { retry_after_ms: Some(5) }),
            UiErrorKind::RateLimited { retry_after_ms: Some(5) }
        );
        assert_eq!(UiErrorKind::from_ipc(&K::NotFound), UiErrorKind::NotFound);
        assert_eq!(UiErrorKind::from_ipc(&K::Unavailable), UiErrorKind::Unavailable);
        assert_eq!(UiErrorKind::from_ipc(&K::Upstream { status: 503 }), UiErrorKind::Upstream { status: 503 });
        assert_eq!(UiErrorKind::from_ipc(&K::Internal), UiErrorKind::Internal);
    }

    #[test]
    fn retryable_and_message() {
        assert!(!UiErrorKind::NotFound.retryable());
        assert!(!UiErrorKind::AuthExpired.retryable());
        assert!(!UiErrorKind::SignedOut.retryable());
        assert!(UiErrorKind::Offline.retryable());
        assert!(UiErrorKind::Timeout.retryable());
        assert!(UiErrorKind::Upstream { status: 503 }.message().contains("503"));
    }
}

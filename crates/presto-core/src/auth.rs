//! Auth state as reported by the engine, plus the fail-fast rule (D-12).
use presto_ipc::AuthState;

#[derive(Debug, PartialEq)]
pub enum AuthEffect {
    None,
    ShowWindow,
}

#[derive(Default)]
pub struct AuthMachine {
    pub state: Option<AuthState>,
}

impl AuthMachine {
    /// Signed out opens the sign-in window (D-06). Expired never does (D-10).
    pub fn on_event(&mut self, s: AuthState) -> AuthEffect {
        self.state = Some(s);
        if s == AuthState::SignedOut { AuthEffect::ShowWindow } else { AuthEffect::None }
    }

    /// An auth_expired reply while signed in means the session lapsed (D-13).
    pub fn on_auth_expired_outcome(&mut self) {
        if self.state == Some(AuthState::SignedIn) {
            self.state = Some(AuthState::Expired);
        }
    }

    pub fn allows_traffic(&self) -> bool {
        self.state == Some(AuthState::SignedIn)
    }

    pub fn blocked(&self) -> bool {
        matches!(self.state, Some(AuthState::SignedOut | AuthState::Expired))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_signed_out_shows_window() {
        let mut a = AuthMachine::default();
        assert_eq!(a.on_event(AuthState::SignedOut), AuthEffect::ShowWindow);
        assert!(a.blocked());
    }

    #[test]
    fn auth_expired_no_window() {
        let mut a = AuthMachine::default();
        assert_eq!(a.on_event(AuthState::Expired), AuthEffect::None);
        assert!(a.blocked());
    }

    #[test]
    fn auth_outcome_expires_signed_in() {
        let mut a = AuthMachine::default();
        a.on_event(AuthState::SignedIn);
        a.on_auth_expired_outcome();
        assert_eq!(a.state, Some(AuthState::Expired));
        let mut b = AuthMachine::default();
        b.on_auth_expired_outcome();
        assert_eq!(b.state, None);
    }

    #[test]
    fn auth_signed_in_allows() {
        let mut a = AuthMachine::default();
        assert!(!a.allows_traffic());
        a.on_event(AuthState::SigningIn);
        assert!(!a.allows_traffic());
        a.on_event(AuthState::SignedIn);
        assert!(a.allows_traffic() && !a.blocked());
    }
}

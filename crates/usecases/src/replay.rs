//! Replay verification.

use orexis_core::{derive_messages, hash_messages, SessionEvent};

/// Replay check: every logged `ModelRequest` hash must equal the pure
/// derivation from the events that preceded it.
pub fn verify_replay(events: &[SessionEvent]) -> bool {
    for (index, event) in events.iter().enumerate() {
        if let SessionEvent::ModelRequest { request_hash, .. } = event {
            let messages = derive_messages(&events[..index]);
            if hash_messages(&messages) != *request_hash {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use orexis_core::FORMAT_VERSION;

    #[test]
    fn replay_verifies_logged_hashes() {
        let user = SessionEvent::UserMessage { text: "hi".into() };
        let messages = derive_messages(std::slice::from_ref(&user));
        let events = vec![
            SessionEvent::SessionStart {
                session_id: "s".into(),
                format_version: FORMAT_VERSION,
            },
            user,
            SessionEvent::ModelRequest {
                profile_id: "fake".into(),
                request_hash: hash_messages(&messages),
            },
        ];
        assert!(verify_replay(&events));
    }

    #[test]
    fn replay_detects_a_tampered_hash() {
        let events = vec![
            SessionEvent::UserMessage { text: "hi".into() },
            SessionEvent::ModelRequest {
                profile_id: "fake".into(),
                request_hash: "deadbeef".into(),
            },
        ];
        assert!(!verify_replay(&events));
    }
}

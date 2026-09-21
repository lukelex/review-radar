//! Platform-neutral contracts for effects requested by a native client.
//!
//! This crate intentionally has no UI toolkit or operating-system dependencies.
//! Each native shell supplies its own implementation of [`PlatformEffects`].

use std::error::Error;

/// A side effect requested by the client after domain and local-state processing.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PlatformCommand {
    OpenUrl { url: String },
    CopyText { text: String },
    ShowNotification(Notification),
}

/// A notification that has already been deduplicated by the local state store.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Notification {
    /// A stable platform-visible identifier, normally based on the attention
    /// fingerprint, so a platform can replace or group its own notifications.
    pub id: String,
    pub title: String,
    pub body: String,
    /// Optional client or browser destination opened when the notification is
    /// activated.
    pub activation_url: Option<String>,
}

/// Native implementations perform the requested operating-system effect.
///
/// Domain and state code return commands but never depend on this trait or on a
/// particular platform implementation.
pub trait PlatformEffects {
    type Error: Error + Send + Sync + 'static;

    fn execute(&self, command: PlatformCommand) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_command_preserves_its_activation_destination() {
        let command = PlatformCommand::ShowNotification(Notification {
            id: "attention:pr-1:event-2".into(),
            title: "Review requested".into(),
            body: "example/repository-001#1 needs your review".into(),
            activation_url: Some("review-radar://pull-request/pr-1".into()),
        });
        assert!(matches!(
            command,
            PlatformCommand::ShowNotification(Notification {
                activation_url: Some(_),
                ..
            })
        ));
    }
}

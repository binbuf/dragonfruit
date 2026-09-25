// SPDX-License-Identifier: MIT
//! The adapter state contract: the three states and their menu-bar slot.

use std::fmt;

/// Stable identity of a status slot, chosen to match the shell's status-item
/// ids (`shell/menubar/StatusItem.qml`), so a projection maps straight onto
/// the QML delegate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AdapterId(&'static str);

impl AdapterId {
    /// Wi-Fi (NetworkManager).
    pub const WIFI: AdapterId = AdapterId("wifi");
    /// Bluetooth (BlueZ; later task).
    pub const BLUETOOTH: AdapterId = AdapterId("bluetooth");
    /// Default-sink volume/mute (PipeWire/WirePlumber).
    pub const AUDIO: AdapterId = AdapterId("volume");
    /// Battery level/charging (UPower).
    pub const POWER: AdapterId = AdapterId("battery");

    /// An id for a status slot the constants above do not name.
    pub const fn new(id: &'static str) -> Self {
        AdapterId(id)
    }

    /// The id as it appears on the wire and in the QML.
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for AdapterId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// Why an adapter that should be present could not read its daemon.
///
/// An adapter error is shown to the user, but is never fatal and never
/// blocks session startup ([07-system-integration.md] principle 4).
///
/// [07-system-integration.md]: ../../../docs/design/07-system-integration.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterError {
    message: String,
}

impl AdapterError {
    /// An error with a human-readable message.
    pub fn new(message: impl Into<String>) -> Self {
        AdapterError {
            message: message.into(),
        }
    }

    /// The human-readable message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for AdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for AdapterError {}

/// The state of one adapter.
///
/// The three states are exhaustive by design: a consumer must decide how an
/// absent daemon and a read failure differ, because they render differently
/// (hidden vs. visible-inert).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterState<T> {
    /// The daemon answered; this is the live snapshot.
    Available(T),
    /// The daemon is absent. A normal state, not an error.
    Unavailable,
    /// The daemon is present but could not be read.
    Error(AdapterError),
}

impl<T> AdapterState<T> {
    /// The daemon answered with `snapshot`.
    pub const fn available(snapshot: T) -> Self {
        AdapterState::Available(snapshot)
    }

    /// The daemon is absent.
    pub const fn unavailable() -> Self {
        AdapterState::Unavailable
    }

    /// The daemon is present but could not be read.
    pub fn with_error(message: impl Into<String>) -> Self {
        AdapterState::Error(AdapterError::new(message))
    }

    /// Whether the daemon answered.
    pub fn is_available(&self) -> bool {
        matches!(self, AdapterState::Available(_))
    }

    /// Whether the daemon is absent.
    pub fn is_unavailable(&self) -> bool {
        matches!(self, AdapterState::Unavailable)
    }

    /// Whether the daemon is present but could not be read.
    pub fn is_error(&self) -> bool {
        matches!(self, AdapterState::Error(_))
    }

    /// The live snapshot, when available.
    pub fn snapshot(&self) -> Option<&T> {
        match self {
            AdapterState::Available(snapshot) => Some(snapshot),
            AdapterState::Unavailable | AdapterState::Error(_) => None,
        }
    }

    /// The read error, when the adapter is in the error state.
    pub fn error(&self) -> Option<&AdapterError> {
        match self {
            AdapterState::Error(error) => Some(error),
            AdapterState::Available(_) | AdapterState::Unavailable => None,
        }
    }

    /// Whether the status slot is drawn at all. An absent daemon hides the
    /// item; available and errored daemons show one.
    pub fn is_visible(&self) -> bool {
        !self.is_unavailable()
    }

    /// Whether the status slot is interactive. Only an available adapter is:
    /// an errored slot is visible but inert.
    pub fn is_enabled(&self) -> bool {
        self.is_available()
    }

    /// Project the state onto the menu-bar slot the shell renders.
    pub fn slot(&self, id: AdapterId) -> StatusSlot {
        StatusSlot {
            id,
            visible: self.is_visible(),
            enabled: self.is_enabled(),
            error: self.error().map(|error| error.message().to_owned()),
        }
    }
}

/// The menu-bar projection of one adapter state.
///
/// This mirrors the shell `StatusItem` fields that the adapter layer owns:
/// an absent daemon is `visible: false`; a read error is `visible: true` with
/// `enabled: false` and a message. Icons/labels/level come from the concrete
/// adapter snapshot, not the state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusSlot {
    /// The slot identity (matches the shell status-item id).
    pub id: AdapterId,
    /// `false` hides the item entirely (daemon absent).
    pub visible: bool,
    /// `false` keeps the item visible but dimmed/inert (read error).
    pub enabled: bool,
    /// The read error to surface, when in the error state.
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn available_shows_an_interactive_slot() {
        let state = AdapterState::available(7u32);
        assert!(state.is_available());
        assert!(state.is_visible());
        assert!(state.is_enabled());
        assert_eq!(state.snapshot(), Some(&7));
        assert!(state.error().is_none());

        let slot = state.slot(AdapterId::AUDIO);
        assert_eq!(
            slot,
            StatusSlot {
                id: AdapterId::AUDIO,
                visible: true,
                enabled: true,
                error: None,
            }
        );
    }

    #[test]
    fn unavailable_hides_the_slot() {
        let state: AdapterState<u32> = AdapterState::unavailable();
        assert!(state.is_unavailable());
        assert!(!state.is_visible());
        assert!(!state.is_enabled());
        assert!(state.snapshot().is_none());

        let slot = state.slot(AdapterId::POWER);
        assert!(!slot.visible);
        assert!(!slot.enabled);
        assert_eq!(slot.error, None);
    }

    #[test]
    fn error_shows_an_inert_slot_with_the_message() {
        let state: AdapterState<u32> = AdapterState::with_error("NetworkManager: timeout");
        assert!(state.is_error());
        assert!(state.is_visible());
        assert!(!state.is_enabled());
        assert!(state.snapshot().is_none());
        assert_eq!(state.error().unwrap().message(), "NetworkManager: timeout");

        let slot = state.slot(AdapterId::WIFI);
        assert_eq!(slot.id, AdapterId::WIFI);
        assert!(slot.visible);
        assert!(!slot.enabled);
        assert_eq!(slot.error.as_deref(), Some("NetworkManager: timeout"));
    }

    #[test]
    fn adapter_ids_match_the_shell_slot_ids() {
        assert_eq!(AdapterId::WIFI.as_str(), "wifi");
        assert_eq!(AdapterId::BLUETOOTH.as_str(), "bluetooth");
        assert_eq!(AdapterId::AUDIO.as_str(), "volume");
        assert_eq!(AdapterId::POWER.as_str(), "battery");
    }
}

// SPDX-License-Identifier: MIT
//! The portal backend's pure model (T-13.1a).
//!
//! This module has no D-Bus dependency: the backend identity, the set of
//! advertised portal interfaces, and the frontend-presence state are plain
//! values the bus layer and the tests share. It is where the "frontend
//! absent is a normal state" rule lives, so it can be unit-tested without a
//! bus.

use std::sync::{Arc, Mutex};

/// The portal backend's name in `portals.conf` and `dragonfruit.portal`
/// (the file's stem, not the D-Bus name).
pub const BACKEND_NAME: &str = "dragonfruit";

/// The standard backend well-known name the `xdg-desktop-portal` frontend
/// looks up for a desktop whose `XDG_CURRENT_DESKTOP` contains `dragonfruit`.
pub const DBUS_NAME: &str = "org.freedesktop.impl.portal.desktop.dragonfruit";

/// The standard object path every portal backend serves its
/// `org.freedesktop.impl.portal.*` interfaces at.
pub const DBUS_PATH: &str = "/org/freedesktop/portal/desktop";

/// The `xdg-desktop-portal` frontend's well-known name. The backend never
/// requires it: it only observes whether it is on the bus.
pub const FRONTEND_NAME: &str = "org.freedesktop.portal.Desktop";

/// The Dragonfruit-facing diagnostic interface the backend serves at
/// [`DBUS_PATH`] alongside the standard portal interfaces. It is not part of
/// the portal contract; it exists so the registration and the
/// frontend-presence state are observable (tests, `--check-frontend`).
pub const STATUS_INTERFACE: &str = "org.dragonfruit.Portal1";

/// The backend interfaces advertised in `dragonfruit.portal` today. Empty in
/// T-13.1a: Settings and GlobalShortcuts arrive in T-13.1b, FileChooser in
/// T-13.2a, Screenshot in T-13.3a, and ScreenCast in T-13.4a. The data-file
/// test keeps the descriptor and this list in lockstep.
pub const BACKEND_INTERFACES: &[&str] = &[];

/// Whether the `xdg-desktop-portal` frontend is on the session bus.
///
/// "Absent" is a normal, supported state (the backend is still valid and
/// serves whatever it can); "unknown" means the bus could not be asked.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum FrontendPresence {
    /// The bus has not answered yet.
    #[default]
    Unknown,
    /// The frontend does not own its well-known name.
    Absent,
    /// The frontend owns its well-known name; `owner` is its unique name.
    Present { owner: String },
}

impl FrontendPresence {
    /// Whether the bus gave a definite answer.
    pub const fn is_known(&self) -> bool {
        !matches!(self, FrontendPresence::Unknown)
    }

    /// Whether the frontend is present.
    pub const fn is_present(&self) -> bool {
        matches!(self, FrontendPresence::Present { .. })
    }

    /// The frontend's unique name, when present.
    pub fn owner(&self) -> Option<&str> {
        match self {
            FrontendPresence::Present { owner } => Some(owner),
            _ => None,
        }
    }

    /// The stable spelling exposed by the diagnostic interface.
    pub const fn label(&self) -> &'static str {
        match self {
            FrontendPresence::Unknown => "unknown",
            FrontendPresence::Absent => "absent",
            FrontendPresence::Present { .. } => "present",
        }
    }
}

/// The backend's current identity and frontend-presence projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendStatus {
    pub backend_name: String,
    pub dbus_name: String,
    pub object_path: String,
    pub version: String,
    pub frontend: FrontendPresence,
    pub interfaces: Vec<String>,
}

impl BackendStatus {
    /// The status for the given frontend presence, with this build's
    /// identity and advertised interfaces.
    pub fn new(frontend: FrontendPresence) -> Self {
        BackendStatus {
            backend_name: BACKEND_NAME.to_owned(),
            dbus_name: DBUS_NAME.to_owned(),
            object_path: DBUS_PATH.to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            frontend,
            interfaces: BACKEND_INTERFACES
                .iter()
                .map(|name| (*name).to_owned())
                .collect(),
        }
    }
}

/// The shared, live frontend-presence cell. The bus watch thread writes it;
/// the served diagnostic interface reads it.
#[derive(Debug, Clone, Default)]
pub struct FrontendTracker {
    inner: Arc<Mutex<FrontendPresence>>,
}

impl FrontendTracker {
    /// A tracker whose initial state is [`FrontendPresence::Unknown`].
    pub fn new() -> Self {
        Self::default()
    }

    /// The current presence.
    pub fn presence(&self) -> FrontendPresence {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Store a new presence, returning whether it changed.
    pub fn set(&self, presence: FrontendPresence) -> bool {
        let mut slot = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let changed = *slot != presence;
        *slot = presence;
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presence_labels_cover_all_three_states() {
        assert_eq!(FrontendPresence::Unknown.label(), "unknown");
        assert!(!FrontendPresence::Unknown.is_known());
        assert_eq!(FrontendPresence::Absent.label(), "absent");
        assert!(FrontendPresence::Absent.is_known());
        assert!(!FrontendPresence::Absent.is_present());
        let present = FrontendPresence::Present {
            owner: ":1.7".to_owned(),
        };
        assert_eq!(present.label(), "present");
        assert!(present.is_present());
        assert_eq!(present.owner(), Some(":1.7"));
        assert_eq!(FrontendPresence::Absent.owner(), None);
    }

    #[test]
    fn the_tracker_reports_whether_the_state_changed() {
        let tracker = FrontendTracker::new();
        assert_eq!(tracker.presence(), FrontendPresence::Unknown);
        assert!(tracker.set(FrontendPresence::Absent));
        assert!(!tracker.set(FrontendPresence::Absent));
        assert!(tracker.set(FrontendPresence::Present {
            owner: ":1.2".to_owned()
        }));
        assert!(tracker.presence().is_present());
    }

    #[test]
    fn the_status_carries_the_standard_identity() {
        let status = BackendStatus::new(FrontendPresence::Absent);
        assert_eq!(status.backend_name, "dragonfruit");
        assert_eq!(status.dbus_name, DBUS_NAME);
        assert_eq!(status.object_path, DBUS_PATH);
        assert_eq!(status.interfaces.len(), BACKEND_INTERFACES.len());
    }

    #[test]
    fn the_backend_name_does_not_leak_other_desktops() {
        assert_eq!(df_ipc::DESKTOP_NAME, BACKEND_NAME);
    }
}

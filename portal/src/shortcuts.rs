// SPDX-License-Identifier: MIT
//! The GlobalShortcuts portal's pure session model (T-13.1b).
//!
//! The backend serves the standard
//! `org.freedesktop.impl.portal.GlobalShortcuts` interface so sandboxed
//! applications register accelerators through the portal instead of grabbing
//! keys directly ([02-compositor.md](../../docs/design/02-compositor.md)).
//!
//! This module owns the *bookkeeping* only: a [`ShortcutRegistry`] of the
//! shortcut sessions a caller created and the shortcuts it bound. It has no
//! D-Bus and no compositor dependency, so the create/bind/list/close
//! lifecycle is unit-tested directly. The compositor's
//! [`ShortcutEngine`](../../compositor/src/input/shortcuts.rs) remains the one
//! arbiter; a later task feeds each bound shortcut into it with the same
//! focus rules and calls back here to raise `Activated` (the diagnostic
//! `ActivateShortcut` on `org.dragonfruit.Portal1` is that bridge today).

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard};

use zbus::zvariant::OwnedValue;

/// The standard GlobalShortcuts backend interface.
pub const GLOBAL_SHORTCUTS_INTERFACE: &str = "org.freedesktop.impl.portal.GlobalShortcuts";
/// The standard session interface every created session object implements.
pub const SESSION_INTERFACE: &str = "org.freedesktop.impl.portal.Session";
/// The backend's GlobalShortcuts portal version.
pub const GLOBAL_SHORTCUTS_VERSION: u32 = 1;

/// Portal response codes shared with the caller.
pub const RESPONSE_SUCCESS: u32 = 0;
pub const RESPONSE_CANCELLED: u32 = 1;
pub const RESPONSE_OTHER: u32 = 2;

/// `org.freedesktop.impl.portal.Session` close reasons.
pub const CLOSED_BY_USER: u32 = 1;
pub const CLOSED_BY_APP: u32 = 2;

/// A shortcut's option dictionary (`a{sv}`).
pub type ShortcutOptions = HashMap<String, OwnedValue>;

/// One shortcut a session bound: its id and its option dictionary.
#[derive(Debug, Clone, PartialEq)]
pub struct PortalShortcut {
    /// The caller's opaque shortcut id.
    pub id: String,
    /// `description`, `trigger`, `preferred_trigger`, … — kept verbatim so
    /// the bound shortcut can be echoed back unchanged.
    pub options: ShortcutOptions,
}

/// One live GlobalShortcuts session.
#[derive(Debug, Clone, PartialEq)]
pub struct ShortcutSession {
    /// The `security-context` app id (or the caller-supplied id).
    pub app_id: String,
    /// The object path the session is served at.
    pub handle: String,
    /// The shortcuts bound so far, in binding order.
    pub shortcuts: Vec<PortalShortcut>,
}

/// Why a session operation was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionError {
    /// A session already exists at that path.
    Duplicate(String),
    /// No session exists at that path.
    Unknown(String),
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionError::Duplicate(handle) => write!(f, "session {handle} already exists"),
            SessionError::Unknown(handle) => write!(f, "no session at {handle}"),
        }
    }
}

impl std::error::Error for SessionError {}

/// The bookkeeping for every live shortcut session.
#[derive(Debug, Default)]
pub struct ShortcutRegistry {
    sessions: BTreeMap<String, ShortcutSession>,
}

impl ShortcutRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a session at `handle` for `app_id`. A duplicate path is
    /// refused so a caller cannot hijack another session.
    pub fn create(
        &mut self,
        app_id: impl Into<String>,
        handle: impl Into<String>,
    ) -> Result<(), SessionError> {
        let handle = handle.into();
        if self.sessions.contains_key(&handle) {
            return Err(SessionError::Duplicate(handle));
        }
        self.sessions.insert(
            handle.clone(),
            ShortcutSession {
                app_id: app_id.into(),
                handle,
                shortcuts: Vec::new(),
            },
        );
        Ok(())
    }

    /// Replace a session's bound shortcuts and return the bound list.
    pub fn bind(
        &mut self,
        handle: &str,
        shortcuts: Vec<PortalShortcut>,
    ) -> Result<Vec<PortalShortcut>, SessionError> {
        let session = self
            .sessions
            .get_mut(handle)
            .ok_or_else(|| SessionError::Unknown(handle.to_owned()))?;
        session.shortcuts = shortcuts;
        Ok(session.shortcuts.clone())
    }

    /// A session's bound shortcuts.
    pub fn list(&self, handle: &str) -> Result<&[PortalShortcut], SessionError> {
        self.sessions
            .get(handle)
            .map(|session| session.shortcuts.as_slice())
            .ok_or_else(|| SessionError::Unknown(handle.to_owned()))
    }

    /// Drop a session, returning whether it existed.
    pub fn close(&mut self, handle: &str) -> bool {
        self.sessions.remove(handle).is_some()
    }

    /// Look a session up.
    pub fn session(&self, handle: &str) -> Option<&ShortcutSession> {
        self.sessions.get(handle)
    }

    /// Whether a session exists.
    pub fn contains(&self, handle: &str) -> bool {
        self.sessions.contains_key(handle)
    }

    /// Whether a session has bound the given shortcut id.
    pub fn has_shortcut(&self, handle: &str, id: &str) -> bool {
        self.session(handle)
            .is_some_and(|session| session.shortcuts.iter().any(|shortcut| shortcut.id == id))
    }

    /// The number of live sessions.
    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    /// Whether there are no live sessions.
    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }
}

/// The shared, live registry: the D-Bus objects and the diagnostic bridge
/// all read the same bookkeeping.
pub type SharedRegistry = Arc<Mutex<ShortcutRegistry>>;

/// A shared, empty registry.
pub fn registry() -> SharedRegistry {
    Arc::new(Mutex::new(ShortcutRegistry::new()))
}

/// Lock a registry, recovering from a poisoned mutex so a panicking D-Bus
/// call never takes the service down.
pub fn lock(registry: &SharedRegistry) -> MutexGuard<'_, ShortcutRegistry> {
    registry
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shortcut(id: &str) -> PortalShortcut {
        PortalShortcut {
            id: id.to_owned(),
            options: HashMap::new(),
        }
    }

    #[test]
    fn a_session_round_trips_create_bind_list_close() {
        let mut registry = ShortcutRegistry::new();
        registry
            .create(
                "org.example.App",
                "/org/freedesktop/portal/desktop/session/1/a",
            )
            .expect("create");

        let bound = registry
            .bind(
                "/org/freedesktop/portal/desktop/session/1/a",
                vec![shortcut("play"), shortcut("pause")],
            )
            .expect("bind");
        assert_eq!(bound.len(), 2);
        assert_eq!(bound[0].id, "play");

        let listed = registry
            .list("/org/freedesktop/portal/desktop/session/1/a")
            .expect("list");
        assert_eq!(listed, bound.as_slice());

        assert!(registry.close("/org/freedesktop/portal/desktop/session/1/a"));
        assert!(registry.is_empty());
        assert!(!registry.close("/org/freedesktop/portal/desktop/session/1/a"));
    }

    #[test]
    fn duplicate_paths_and_unknown_sessions_are_refused() {
        let mut registry = ShortcutRegistry::new();
        registry.create("app", "/s").unwrap();
        assert_eq!(
            registry.create("other", "/s"),
            Err(SessionError::Duplicate("/s".into()))
        );
        assert_eq!(
            registry.bind("/missing", vec![]),
            Err(SessionError::Unknown("/missing".into()))
        );
        assert_eq!(
            registry.list("/missing"),
            Err(SessionError::Unknown("/missing".into()))
        );
    }

    #[test]
    fn has_shortcut_only_matches_a_bound_id() {
        let mut registry = ShortcutRegistry::new();
        registry.create("app", "/s").unwrap();
        assert!(!registry.has_shortcut("/s", "play"));
        registry.bind("/s", vec![shortcut("play")]).unwrap();
        assert!(registry.has_shortcut("/s", "play"));
        assert!(!registry.has_shortcut("/s", "pause"));
        assert!(!registry.has_shortcut("/other", "play"));
    }

    #[test]
    fn the_interface_names_follow_the_portal_contract() {
        assert_eq!(
            GLOBAL_SHORTCUTS_INTERFACE,
            "org.freedesktop.impl.portal.GlobalShortcuts"
        );
        assert_eq!(SESSION_INTERFACE, "org.freedesktop.impl.portal.Session");
        assert_eq!(RESPONSE_SUCCESS, 0);
        assert_eq!(GLOBAL_SHORTCUTS_VERSION, 1);
    }
}

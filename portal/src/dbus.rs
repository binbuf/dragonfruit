// SPDX-License-Identifier: MIT
//! The session-bus service (T-13.1a).
//!
//! The backend owns one well-known name, [`DBUS_NAME`], and serves the
//! standard object path [`DBUS_PATH`]. T-13.1a installs the service and its
//! registration only; the concrete `org.freedesktop.impl.portal.*`
//! interfaces are added additively by T-13.1b and later tasks.
//!
//! Alongside the standard interfaces (session bus / object manager plumbing
//! and `org.freedesktop.DBus.Introspectable`, provided by zbus), the backend
//! serves [`STATUS_INTERFACE`] at the same path: a Dragonfruit diagnostic
//! surface that reports the identity and whether the `xdg-desktop-portal`
//! frontend is on the bus. It is not part of the portal contract.
//!
//! ## Frontend absence is normal
//!
//! The backend does not depend on the frontend: it registers regardless and
//! keeps serving. It only *observes* the frontend's well-known name — once at
//! startup and then live via `org.freedesktop.DBus.NameOwnerChanged` — and
//! exposes that presence on [`STATUS_INTERFACE`] so a session with no
//! `xdg-desktop-portal` is a visible, supported state rather than an error
//! (`tests/session_bus.rs` proves both paths).

use zbus::blocking::connection;
use zbus::interface;
use zbus::object_server::SignalEmitter;

use crate::model::{
    BackendStatus, FrontendPresence, FrontendTracker, BACKEND_INTERFACES, BACKEND_NAME, DBUS_NAME,
    DBUS_PATH, FRONTEND_NAME, STATUS_INTERFACE,
};

/// The `org.dragonfruit.Portal1` object. It shares the standard portal path
/// with the future `org.freedesktop.impl.portal.*` interfaces.
#[derive(Clone)]
pub struct Backend {
    frontend: FrontendTracker,
}

impl Backend {
    /// A diagnostic object over `frontend`.
    pub fn new(frontend: FrontendTracker) -> Self {
        Backend { frontend }
    }

    /// The frontend tracker behind this object.
    pub fn frontend_tracker(&self) -> &FrontendTracker {
        &self.frontend
    }

    /// The backend's current status.
    pub fn status(&self) -> BackendStatus {
        BackendStatus::new(self.frontend.presence())
    }
}

#[interface(name = "org.dragonfruit.Portal1")]
impl Backend {
    /// The portal backend name used in `portals.conf` (`dragonfruit`).
    fn backend_name(&self) -> String {
        BACKEND_NAME.to_owned()
    }

    /// The standard backend well-known name.
    fn dbus_name(&self) -> String {
        DBUS_NAME.to_owned()
    }

    /// The standard backend object path.
    fn object_path(&self) -> String {
        DBUS_PATH.to_owned()
    }

    /// The backend build version.
    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_owned()
    }

    /// The advertised `org.freedesktop.impl.portal.*` interfaces (empty until
    /// T-13.1b).
    fn interfaces(&self) -> Vec<String> {
        BACKEND_INTERFACES
            .iter()
            .map(|name| (*name).to_owned())
            .collect()
    }

    /// `present` / `absent` / `unknown` — the frontend-presence label.
    fn frontend(&self) -> String {
        self.frontend.presence().label().to_owned()
    }

    /// Whether the `xdg-desktop-portal` frontend is on the bus.
    fn frontend_present(&self) -> bool {
        self.frontend.presence().is_present()
    }

    /// The frontend's unique name when present, else the empty string.
    fn frontend_owner(&self) -> String {
        self.frontend
            .presence()
            .owner()
            .unwrap_or_default()
            .to_owned()
    }

    /// The frontend appeared or left. Only the diagnostic interface emits
    /// this; the portal frontend never needs it.
    #[zbus(signal)]
    async fn frontend_changed(
        emitter: &SignalEmitter<'_>,
        present: bool,
        owner: &str,
    ) -> zbus::Result<()>;
}

/// Ask the bus whether `name` currently has an owner. A bus error is not a
/// fatal condition: the caller maps it to [`FrontendPresence::Unknown`].
pub fn name_has_owner(connection: &connection::Connection, name: &str) -> zbus::Result<bool> {
    let reply = connection.call_method(
        Some("org.freedesktop.DBus"),
        "/org/freedesktop/DBus",
        Some("org.freedesktop.DBus"),
        "NameHasOwner",
        &(name,),
    )?;
    reply.body().deserialize::<bool>()
}

/// The unique name that owns `name`, if any.
pub fn name_owner(connection: &connection::Connection, name: &str) -> zbus::Result<String> {
    let reply = connection.call_method(
        Some("org.freedesktop.DBus"),
        "/org/freedesktop/DBus",
        Some("org.freedesktop.DBus"),
        "GetNameOwner",
        &(name,),
    )?;
    reply.body().deserialize::<String>()
}

/// One synchronous probe of the frontend's presence. Never fails: an
/// unreachable bus is [`FrontendPresence::Unknown`], and an unowned name is
/// [`FrontendPresence::Absent`].
pub fn probe_frontend(connection: &connection::Connection) -> FrontendPresence {
    match name_has_owner(connection, FRONTEND_NAME) {
        Ok(true) => FrontendPresence::Present {
            owner: name_owner(connection, FRONTEND_NAME).unwrap_or_default(),
        },
        Ok(false) => FrontendPresence::Absent,
        Err(_) => FrontendPresence::Unknown,
    }
}

/// Serve [`STATUS_INTERFACE`] at [`DBUS_PATH`], record the current frontend
/// presence, and start the live watch. Call this after the connection owns
/// [`DBUS_NAME`]. The connection must outlive the returned tracker; the watch
/// thread holds its own clone.
pub fn initialize(connection: &connection::Connection) -> Backend {
    let frontend = FrontendTracker::new();
    frontend.set(probe_frontend(connection));
    let backend = Backend::new(frontend.clone());

    if let Err(error) = connection.object_server().at(DBUS_PATH, backend.clone()) {
        eprintln!("xdg-desktop-portal-dragonfruit: cannot serve {STATUS_INTERFACE} at {DBUS_PATH}: {error}");
    }

    spawn_frontend_watch(connection.clone(), frontend);
    backend
}

/// Watch `org.freedesktop.DBus.NameOwnerChanged` for the frontend name and
/// keep `tracker` live, emitting [`STATUS_INTERFACE`]'s `FrontendChanged`
/// each time the state flips.
pub fn spawn_frontend_watch(connection: connection::Connection, tracker: FrontendTracker) {
    let _ = std::thread::Builder::new()
        .name("dragonfruit-portal-frontend".to_owned())
        .spawn(move || frontend_watch_loop(connection, tracker));
}

fn frontend_watch_loop(connection: connection::Connection, tracker: FrontendTracker) {
    let rule = zbus::MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .sender("org.freedesktop.DBus")
        .expect("valid sender")
        .interface("org.freedesktop.DBus")
        .expect("valid interface")
        .member("NameOwnerChanged")
        .expect("valid member")
        .add_arg(FRONTEND_NAME)
        .expect("valid arg")
        .build();

    let iterator = match zbus::blocking::MessageIterator::for_match_rule(rule, &connection, Some(4))
    {
        Ok(iterator) => iterator,
        Err(_) => return,
    };

    for message in iterator {
        if message.is_err() {
            break;
        }
        let presence = probe_frontend(&connection);
        if !tracker.set(presence.clone()) {
            continue;
        }
        let present = presence.is_present();
        let owner = presence.owner().unwrap_or_default();
        let _ = connection.emit_signal(
            None::<&str>,
            DBUS_PATH,
            STATUS_INTERFACE,
            "FrontendChanged",
            &(present, owner),
        );
    }
}

/// Build a service connection on `address` (a session bus, or a private bus
/// when given), own [`DBUS_NAME`], and serve the diagnostic object with the
/// live frontend watch. Returns the connection and the backend object; the
/// caller keeps the connection alive for as long as it wants to serve.
///
/// A missing frontend never makes this fail; only an unreachable bus or an
/// unavailable name does.
pub fn serve(address: Option<&str>) -> zbus::Result<(connection::Connection, Backend)> {
    let builder = match address {
        Some(address) => connection::Builder::address(address)?,
        None => connection::Builder::session()?,
    };
    let connection = builder.name(DBUS_NAME)?.build()?;
    let backend = initialize(&connection);
    Ok((connection, backend))
}

/// Serve the backend on the session bus until the process is asked to stop.
/// Returns an error only when the bus or the well-known name cannot be
/// obtained; a session with no frontend is not an error.
pub fn run() -> zbus::Result<()> {
    let (_connection, _backend) = serve(None)?;

    // The object server runs on its own executor; parking the main thread
    // keeps the process (and the name) alive without a poll loop.
    loop {
        std::thread::park();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_served_identity_follows_the_backend_contract() {
        let backend = Backend::new(FrontendTracker::new());
        assert_eq!(backend.backend_name(), "dragonfruit");
        assert_eq!(backend.dbus_name(), DBUS_NAME);
        assert_eq!(backend.object_path(), DBUS_PATH);
        assert_eq!(backend.frontend(), "unknown");
        assert!(!backend.frontend_present());
        assert_eq!(backend.frontend_owner(), "");
        assert!(df_ipc::is_valid_dbus_name(STATUS_INTERFACE));
    }
}
